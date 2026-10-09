//! Borderless layered translation window (same paint path as overlay labels).

use std::mem::size_of;

use tracing::warn;
use translator_core::{OverlayConfig, TranslatedBlock};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{DeleteObject, HFONT, ScreenToClient},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, GWL_STYLE, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW,
            GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, HWND_TOPMOST,
            MINMAXINFO, RegisterClassExW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos, ShowWindow, UnregisterClassW, WM_CLOSE, WM_DESTROY,
            WM_GETMINMAXINFO, WM_NCHITTEST, WM_SIZE, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP,
        },
    },
    core::{PCWSTR, w},
};

use crate::{
    error::OverlayError,
    gfx::{
        draw::{self, SurfaceRect},
        surface::DibSurface,
        text::{self, LabelStyle},
    },
    host::win32::{ensure_topmost, lparam_point},
};

const CLASS_NAME: PCWSTR = w!("TranslatorOverlayReader.v2");
/// Distinct from the WinUI control window (`Translator Overlay`) so OBS Window Capture can pick this HWND.
const WINDOW_TITLE: PCWSTR = w!("Translator Overlay Translation");
/// Shown until the app sends a localized placeholder.
const DEFAULT_PLACEHOLDER: &str = "(no translation yet)";
const DEFAULT_W: i32 = 440;
const DEFAULT_H: i32 = 200;
const MIN_W: i32 = 160;
const MIN_H: i32 = 80;
const EDGE: i32 = 8;
const TEXT_INSET: i32 = 12;

/// Join the non-empty block translations for the reader, one per line.
pub fn format_reader_text(blocks: &[TranslatedBlock]) -> String {
    blocks
        .iter()
        .map(|b| b.translation.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) struct ReaderWindow {
    hwnd: HWND,
    class_atom: u16,
    config: OverlayConfig,
    surface: DibSurface,
    hfont: HFONT,
    font_px: i32,
    text: String,
    placeholder: String,
    dismissed: bool,
}

impl ReaderWindow {
    pub(crate) fn create(config: &OverlayConfig) -> Result<Box<Self>, OverlayError> {
        let hinstance = unsafe { GetModuleHandleW(None) }.map_err(|e| OverlayError::Other(format!("GetModuleHandleW: {e}")))?;

        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(reader_wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        let atom = unsafe { RegisterClassExW(&wc) };

        // CW_USEDEFAULT is ignored for WS_POPUP (the window lands at 0,0).
        // Create overlapped so the window manager can cascade, then drop chrome.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                CLASS_NAME,
                WINDOW_TITLE,
                WS_OVERLAPPED,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                DEFAULT_W,
                DEFAULT_H,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .map_err(|e| OverlayError::Other(format!("CreateWindowExW(reader): {e}")))?;
        unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, WS_POPUP.0 as isize) };
        let _ = unsafe { SetWindowPos(hwnd, None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED) };

        let surface = match DibSurface::create() {
            Ok(s) => s,
            Err(e) => {
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(e);
            }
        };

        let font_px = config.reader_font_px_clamped();
        let hfont = match text::create_segoe_font(font_px) {
            Ok(font) => font,
            Err(e) => {
                drop(surface);
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(e);
            }
        };

        let mut reader = Box::new(Self {
            hwnd,
            class_atom: atom,
            config: config.clone(),
            surface,
            hfont,
            font_px,
            text: String::new(),
            placeholder: DEFAULT_PLACEHOLDER.to_string(),
            dismissed: false,
        });

        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, &raw mut *reader as isize) };
        if let Err(e) = reader.repaint() {
            reader.teardown();
            return Err(e);
        }
        if config.reader_enabled {
            reader.show();
        } else {
            reader.hide();
        }
        Ok(reader)
    }

    pub(crate) fn set_text(&mut self, text: &str) {
        self.restore_topmost_if_needed();
        if text == self.text {
            return;
        }
        self.text = text.to_string();
        if let Err(e) = self.repaint() {
            warn!(error = %e, "reader repaint failed");
        }
    }

    pub(crate) fn set_placeholder(&mut self, placeholder: String) {
        self.restore_topmost_if_needed();
        if placeholder == self.placeholder {
            return;
        }
        self.placeholder = placeholder;
        if self.text.trim().is_empty()
            && let Err(e) = self.repaint()
        {
            warn!(error = %e, "reader repaint failed");
        }
    }

    pub(crate) fn restore_topmost_if_needed(&self) {
        if self.config.reader_enabled && !self.dismissed {
            ensure_topmost(self.hwnd);
        }
    }

    pub(crate) fn apply_config(&mut self, config: &OverlayConfig) {
        self.config = config.clone();
        let font_px = config.reader_font_px_clamped();
        if font_px != self.font_px
            && let Ok(font) = text::create_segoe_font(font_px)
        {
            if !self.hfont.is_invalid() {
                let _ = unsafe { DeleteObject(self.hfont.into()) };
            }
            self.hfont = font;
            self.font_px = font_px;
        }
        if let Err(e) = self.repaint() {
            warn!(error = %e, "reader config repaint failed");
        }
        self.sync_visibility(config.reader_enabled);
    }

    pub(crate) fn teardown(&mut self) {
        if !self.hwnd.is_invalid() {
            unsafe { SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0) };
            let _ = unsafe { DestroyWindow(self.hwnd) };
            self.hwnd = HWND::default();
        }
        if !self.hfont.is_invalid() {
            let _ = unsafe { DeleteObject(self.hfont.into()) };
            self.hfont = HFONT::default();
        }
        self.surface.teardown();
        if self.class_atom != 0 {
            if let Ok(hi) = unsafe { GetModuleHandleW(None) } {
                let _ = unsafe { UnregisterClassW(CLASS_NAME, Some(hi.into())) };
            }
            self.class_atom = 0;
        }
    }

    fn sync_visibility(&mut self, enabled: bool) {
        if !enabled {
            self.dismissed = false;
            self.hide();
            return;
        }
        if self.dismissed {
            return;
        }
        self.show();
    }

    fn show(&mut self) {
        let _ =
            unsafe { SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW) };
        let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
        if let Err(e) = self.present() {
            warn!(error = %e, "reader present failed");
        }
    }

    fn hide(&mut self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    fn dismiss(&mut self) {
        self.dismissed = true;
        self.hide();
    }

    fn client_size(&self) -> (i32, i32) {
        let mut client = RECT::default();
        if unsafe { GetClientRect(self.hwnd, &mut client) }.is_err() {
            return (DEFAULT_W, DEFAULT_H);
        }
        ((client.right - client.left).max(1), (client.bottom - client.top).max(1))
    }

    fn repaint(&mut self) -> Result<(), OverlayError> {
        let (w, h) = self.client_size();
        self.surface.ensure(w, h)?;
        let bg = draw::Rgba::from_argb(self.config.background_color_argb);
        let fg = draw::Rgba::from_argb(self.config.text_color_argb);
        let surface = draw::SurfaceSize::new(w, h);
        {
            let buf = self
                .surface
                .pixels()
                .ok_or_else(|| OverlayError::Other("reader paint bitmap missing".into()))?;
            buf.fill(0);
            draw::fill_rect(buf, surface, SurfaceRect::new(0, 0, w, h), bg);
        }

        let inset = TEXT_INSET.min(w / 4).min(h / 4).max(0);
        let text_box = SurfaceRect::new(inset, inset, (w - inset * 2).max(1), (h - inset * 2).max(1));
        let hdc = self.surface.hdc();
        let hfont = self.hfont;
        let shown = if self.text.trim().is_empty() {
            &self.placeholder
        } else {
            &self.text
        };
        let buf = self
            .surface
            .pixels()
            .ok_or_else(|| OverlayError::Other("reader paint bitmap missing".into()))?;
        text::draw_text_label(hdc, hfont, buf, surface, text_box, shown, LabelStyle {
            font_px: self.font_px,
            color: fg,
            vcenter: false,
        })?;
        self.present()
    }

    fn present(&mut self) -> Result<(), OverlayError> {
        let (bw, bh) = self.surface.size();
        let mut wnd = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut wnd) }.is_err() {
            return Err(OverlayError::Other("GetWindowRect(reader) failed".into()));
        }
        self.surface.present(self.hwnd, wnd.left, wnd.top, bw, bh)
    }

    fn hit_test(&self, lparam: LPARAM) -> LRESULT {
        let (x, y) = lparam_point(lparam);
        let mut pt = POINT { x, y };
        if !unsafe { ScreenToClient(self.hwnd, &mut pt) }.as_bool() {
            return LRESULT(HTCAPTION as isize);
        }
        let (w, h) = self.client_size();
        let left = pt.x < EDGE;
        let right = pt.x >= w - EDGE;
        let top = pt.y < EDGE;
        let bottom = pt.y >= h - EDGE;
        let ht = match (top, bottom, left, right) {
            (true, _, true, _) => HTTOPLEFT,
            (true, _, _, true) => HTTOPRIGHT,
            (_, true, true, _) => HTBOTTOMLEFT,
            (_, true, _, true) => HTBOTTOMRIGHT,
            (true, _, _, _) => HTTOP,
            (_, true, _, _) => HTBOTTOM,
            (_, _, true, _) => HTLEFT,
            (_, _, _, true) => HTRIGHT,
            _ => HTCAPTION,
        };
        LRESULT(ht as isize)
    }
}

impl Drop for ReaderWindow {
    fn drop(&mut self) {
        self.teardown();
    }
}

unsafe extern "system" fn reader_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_GETMINMAXINFO {
        let info = lparam.0 as *mut MINMAXINFO;
        if !info.is_null() {
            unsafe { (*info).ptMinTrackSize = POINT { x: MIN_W, y: MIN_H } };
        }
        return LRESULT(0);
    }
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let reader = unsafe { &mut *(ptr as *mut ReaderWindow) };
    match msg {
        WM_NCHITTEST => reader.hit_test(lparam),
        WM_SIZE => {
            if let Err(e) = reader.repaint() {
                warn!(error = %e, "reader resize paint failed");
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            reader.dismiss();
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use translator_core::{Rect, TranslatedBlock};

    use super::*;

    fn block(translation: &str) -> TranslatedBlock {
        TranslatedBlock {
            id: 1,
            source: "src".into(),
            translation: translation.into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_lines: 1,
            source_height: 10.0,
        }
    }

    #[test]
    fn format_reader_text_joins_and_skips_empty() {
        assert_eq!(format_reader_text(&[]), "");
        assert_eq!(format_reader_text(&[block("  "), block("")]), "");
        assert_eq!(format_reader_text(&[block("hello"), block("  world  ")]), "hello\nworld");
    }

    #[test]
    fn reader_hwnd_is_obs_capturable() -> Result<(), String> {
        use translator_core::OverlayConfig;
        use windows::Win32::UI::WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongPtrW, GetWindowTextW, IsWindowVisible, WINDOW_EX_STYLE, WS_EX_TOOLWINDOW,
        };

        let _lock = crate::host::win32::lock_hwnd_tests();
        let mut reader = ReaderWindow::create(&OverlayConfig {
            reader_enabled: true,
            ..OverlayConfig::default()
        })
        .map_err(|e| format!("ReaderWindow::create: {e}"))?;

        let mut title_buf = [0u16; 256];
        let n = unsafe { GetWindowTextW(reader.hwnd, &mut title_buf) }.max(0) as usize;
        let title = String::from_utf16_lossy(&title_buf[..n]);
        let ex = WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(reader.hwnd, GWL_EXSTYLE) } as u32);
        let visible = unsafe { IsWindowVisible(reader.hwnd) }.as_bool();
        let toolwindow = ex.contains(WS_EX_TOOLWINDOW);

        reader.teardown();

        if title != "Translator Overlay Translation" {
            return Err(format!("reader title is {title:?}, expected Translator Overlay Translation"));
        }
        if toolwindow {
            return Err("reader HWND is WS_EX_TOOLWINDOW (OBS skips it)".into());
        }
        if !visible {
            return Err("enabled reader HWND is not visible".into());
        }
        Ok(())
    }

    #[test]
    fn set_text_restores_topmost_without_content_change() {
        use windows::Win32::UI::WindowsAndMessaging::{HWND_NOTOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos};

        let _lock = crate::host::win32::lock_hwnd_tests();
        let mut reader = ReaderWindow::create(&OverlayConfig {
            reader_enabled: true,
            ..OverlayConfig::default()
        })
        .expect("reader create");
        let _ = unsafe { SetWindowPos(reader.hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
        assert!(!crate::host::win32::window_is_topmost(reader.hwnd), "HWND_NOTOPMOST should drop WS_EX_TOPMOST");
        reader.set_text("");
        assert!(crate::host::win32::window_is_topmost(reader.hwnd), "set_text should restore topmost");
        reader.teardown();
    }
}
