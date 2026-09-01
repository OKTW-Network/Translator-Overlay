//! Borderless layered translation window (same paint path as overlay labels).

use std::mem::size_of;

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
            WM_GETMINMAXINFO, WM_NCHITTEST, WM_SIZE, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
            WS_OVERLAPPED, WS_POPUP,
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
};

const CLASS_NAME: PCWSTR = w!("TranslatorOverlayReader.v2");
const EMPTY_PLACEHOLDER: &str = "(no translation yet)";
const DEFAULT_W: i32 = 440;
const DEFAULT_H: i32 = 200;
const MIN_W: i32 = 160;
const MIN_H: i32 = 80;
const EDGE: i32 = 8;
const TEXT_INSET: i32 = 12;

/// Join block translations for the reader, or a placeholder when empty.
pub fn format_reader_text(blocks: &[TranslatedBlock]) -> String {
    let joined = blocks
        .iter()
        .map(|b| b.translation.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if joined.is_empty() { EMPTY_PLACEHOLDER.to_string() } else { joined }
}

pub(crate) struct ReaderWindow {
    hwnd: HWND,
    class_atom: u16,
    config: OverlayConfig,
    surface: DibSurface,
    hfont: HFONT,
    font_px: i32,
    last_text: String,
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
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("Translation"),
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
            last_text: EMPTY_PLACEHOLDER.to_string(),
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
        let display = if text.trim().is_empty() {
            EMPTY_PLACEHOLDER.to_string()
        } else {
            text.to_string()
        };
        if display == self.last_text {
            return;
        }
        self.last_text = display;
        if let Err(e) = self.repaint() {
            tracing::warn!(error = %e, "reader repaint failed");
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
            tracing::warn!(error = %e, "reader config repaint failed");
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
            tracing::warn!(error = %e, "reader present failed");
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
        let bg = draw::background_rgba(&self.config);
        let fg = draw::text_rgba(&self.config);
        let surface = draw::SurfaceSize::new(w, h);
        {
            let buf = self
                .surface
                .pixels()
                .ok_or_else(|| OverlayError::Other("reader paint bitmap missing".into()))?;
            draw::clear(buf);
            draw::fill_rect(buf, surface, SurfaceRect { x: 0, y: 0, w, h }, bg);
        }

        let inset = TEXT_INSET.min(w / 4).min(h / 4).max(0);
        let text_box = SurfaceRect {
            x: inset,
            y: inset,
            w: (w - inset * 2).max(1),
            h: (h - inset * 2).max(1),
        };
        let hdc = self.surface.hdc();
        let hfont = self.hfont;
        let buf = self
            .surface
            .pixels()
            .ok_or_else(|| OverlayError::Other("reader paint bitmap missing".into()))?;
        text::draw_text_label(hdc, hfont, buf, surface, text_box, &self.last_text, LabelStyle {
            font_px: self.font_px,
            color: fg,
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
        // GET_X_LPARAM / GET_Y_LPARAM: signed 16-bit halves of screen coords.
        let packed = lparam.0 as u32;
        let sx = (packed & 0xFFFF) as i16 as i32;
        let sy = ((packed >> 16) & 0xFFFF) as i16 as i32;
        let mut pt = POINT { x: sx, y: sy };
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
                tracing::warn!(error = %e, "reader resize paint failed");
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
        }
    }

    #[test]
    fn format_reader_text_joins_and_skips_empty() {
        assert_eq!(format_reader_text(&[]), EMPTY_PLACEHOLDER);
        assert_eq!(format_reader_text(&[block("  "), block("")]), EMPTY_PLACEHOLDER);
        assert_eq!(format_reader_text(&[block("hello"), block("  world  ")]), "hello\nworld");
    }
}
