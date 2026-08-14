//! Borderless layered translation window (same paint path as overlay labels).

use std::mem::size_of;

use translator_core::{OverlayConfig, TranslatedBlock};
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection,
            DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HBITMAP, HDC, HFONT, HGDIOBJ, ReleaseDC, ScreenToClient, SelectObject,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, GWL_STYLE, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW,
            GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, HWND_TOPMOST,
            MINMAXINFO, RegisterClassExW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos, ShowWindow, ULW_ALPHA, UnregisterClassW, UpdateLayeredWindow,
            WM_CLOSE, WM_DESTROY, WM_GETMINMAXINFO, WM_NCHITTEST, WM_SIZE, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP,
        },
    },
    core::{PCWSTR, w},
};

use crate::{
    OverlayError,
    draw::{self, SurfaceRect},
    text::{self, LabelStyle},
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
    hdc_screen: HDC,
    hdc_mem: HDC,
    hbmp: HBITMAP,
    bits: *mut u8,
    bmp_w: i32,
    bmp_h: i32,
    hfont: HFONT,
    font_px: i32,
    last_text: String,
    dismissed: bool,
}

impl ReaderWindow {
    pub(crate) fn create(config: &OverlayConfig) -> Result<Box<Self>, OverlayError> {
        unsafe {
            let hinstance = GetModuleHandleW(None).map_err(|e| OverlayError::Other(format!("GetModuleHandleW: {e}")))?;

            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(reader_wnd_proc),
                hInstance: hinstance.into(),
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };
            let atom = RegisterClassExW(&wc);

            // CW_USEDEFAULT is ignored for WS_POPUP (the window lands at 0,0).
            // Create overlapped so the window manager can cascade, then drop chrome.
            let hwnd = CreateWindowExW(
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
            .map_err(|e| OverlayError::Other(format!("CreateWindowExW(reader): {e}")))?;
            SetWindowLongPtrW(hwnd, GWL_STYLE, WS_POPUP.0 as isize);
            let _ = SetWindowPos(hwnd, None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);

            let hdc_screen = GetDC(Some(hwnd));
            if hdc_screen.is_invalid() {
                let _ = DestroyWindow(hwnd);
                return Err(OverlayError::Other("GetDC(reader) failed".into()));
            }
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            if hdc_mem.is_invalid() {
                ReleaseDC(Some(hwnd), hdc_screen);
                let _ = DestroyWindow(hwnd);
                return Err(OverlayError::Other("CreateCompatibleDC(reader) failed".into()));
            }

            let font_px = config.reader_font_px_clamped();
            let hfont = match text::create_segoe_font(font_px) {
                Ok(font) => font,
                Err(e) => {
                    let _ = DeleteDC(hdc_mem);
                    ReleaseDC(Some(hwnd), hdc_screen);
                    let _ = DestroyWindow(hwnd);
                    return Err(e);
                }
            };

            let mut reader = Box::new(Self {
                hwnd,
                class_atom: atom,
                config: config.clone(),
                hdc_screen,
                hdc_mem,
                hbmp: HBITMAP::default(),
                bits: std::ptr::null_mut(),
                bmp_w: 0,
                bmp_h: 0,
                hfont,
                font_px,
                last_text: EMPTY_PLACEHOLDER.to_string(),
                dismissed: false,
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &raw mut *reader as isize);
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
            unsafe {
                if !self.hfont.is_invalid() {
                    let _ = DeleteObject(self.hfont.into());
                }
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
        unsafe {
            if !self.hwnd.is_invalid() {
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                let _ = DestroyWindow(self.hwnd);
                self.hwnd = HWND::default();
            }
            if !self.hfont.is_invalid() {
                let _ = DeleteObject(self.hfont.into());
                self.hfont = HFONT::default();
            }
            if !self.hbmp.is_invalid() {
                let _ = DeleteObject(self.hbmp.into());
                self.hbmp = HBITMAP::default();
                self.bits = std::ptr::null_mut();
            }
            if !self.hdc_mem.is_invalid() {
                let _ = DeleteDC(self.hdc_mem);
                self.hdc_mem = HDC::default();
            }
            if !self.hdc_screen.is_invalid() {
                ReleaseDC(None, self.hdc_screen);
                self.hdc_screen = HDC::default();
            }
            if self.class_atom != 0 {
                if let Ok(hi) = GetModuleHandleW(None) {
                    let _ = UnregisterClassW(CLASS_NAME, Some(hi.into()));
                }
                self.class_atom = 0;
            }
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
        unsafe {
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
        if let Err(e) = self.present() {
            tracing::warn!(error = %e, "reader present failed");
        }
    }

    fn hide(&mut self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    fn dismiss(&mut self) {
        self.dismissed = true;
        self.hide();
    }

    fn ensure_bitmap(&mut self, w: i32, h: i32) -> Result<(), OverlayError> {
        if w <= 0 || h <= 0 {
            return Err(OverlayError::Other("invalid reader bitmap size".into()));
        }
        if w == self.bmp_w && h == self.bmp_h && !self.bits.is_null() && !self.hbmp.is_invalid() {
            return Ok(());
        }
        unsafe {
            if !self.hbmp.is_invalid() {
                let _ = SelectObject(self.hdc_mem, HGDIOBJ::default());
                let _ = DeleteObject(self.hbmp.into());
                self.hbmp = HBITMAP::default();
                self.bits = std::ptr::null_mut();
            }
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            let hbmp = CreateDIBSection(Some(self.hdc_mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                .map_err(|e| OverlayError::Other(format!("CreateDIBSection(reader): {e}")))?;
            if hbmp.is_invalid() || bits.is_null() {
                return Err(OverlayError::Other("CreateDIBSection(reader) returned null".into()));
            }
            let _ = SelectObject(self.hdc_mem, HGDIOBJ(hbmp.0));
            self.hbmp = hbmp;
            self.bits = bits.cast();
            self.bmp_w = w;
            self.bmp_h = h;
        }
        Ok(())
    }

    fn client_size(&self) -> (i32, i32) {
        unsafe {
            let mut client = RECT::default();
            if GetClientRect(self.hwnd, &mut client).is_err() {
                return (DEFAULT_W, DEFAULT_H);
            }
            ((client.right - client.left).max(1), (client.bottom - client.top).max(1))
        }
    }

    fn repaint(&mut self) -> Result<(), OverlayError> {
        let (w, h) = self.client_size();
        self.ensure_bitmap(w, h)?;
        let len = (w as usize) * (h as usize) * 4;
        let buf = unsafe { std::slice::from_raw_parts_mut(self.bits, len) };
        draw::clear(buf);

        let bg = draw::background_rgba(&self.config);
        let fg = draw::text_rgba(&self.config);
        let surface = draw::SurfaceSize::new(w, h);
        draw::fill_rect(buf, surface, SurfaceRect { x: 0, y: 0, w, h }, bg);

        let inset = TEXT_INSET.min(w / 4).min(h / 4).max(0);
        let text_box = SurfaceRect {
            x: inset,
            y: inset,
            w: (w - inset * 2).max(1),
            h: (h - inset * 2).max(1),
        };
        crate::text::draw_text_label(self.hdc_mem, self.hfont, buf, surface, text_box, &self.last_text, LabelStyle {
            font_px: self.font_px,
            color: fg,
        })?;
        self.present()
    }

    fn present(&mut self) -> Result<(), OverlayError> {
        if self.bits.is_null() || self.bmp_w <= 0 || self.bmp_h <= 0 {
            return Err(OverlayError::Other("reader paint bitmap missing".into()));
        }
        unsafe {
            let mut wnd = RECT::default();
            if GetWindowRect(self.hwnd, &mut wnd).is_err() {
                return Err(OverlayError::Other("GetWindowRect(reader) failed".into()));
            }
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let ppt_dst = POINT { x: wnd.left, y: wnd.top };
            let psize = SIZE {
                cx: self.bmp_w,
                cy: self.bmp_h,
            };
            let ppt_src = POINT { x: 0, y: 0 };
            UpdateLayeredWindow(
                self.hwnd,
                Some(self.hdc_screen),
                Some(&ppt_dst),
                Some(&psize),
                Some(self.hdc_mem),
                Some(&ppt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
            .map_err(|e| OverlayError::Other(format!("UpdateLayeredWindow(reader): {e}")))?;
        }
        Ok(())
    }

    fn hit_test(&self, lparam: LPARAM) -> LRESULT {
        unsafe {
            // GET_X_LPARAM / GET_Y_LPARAM: signed 16-bit halves of screen coords.
            let packed = lparam.0 as u32;
            let sx = (packed & 0xFFFF) as i16 as i32;
            let sy = ((packed >> 16) & 0xFFFF) as i16 as i32;
            let mut pt = POINT { x: sx, y: sy };
            if !ScreenToClient(self.hwnd, &mut pt).as_bool() {
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
}

impl Drop for ReaderWindow {
    fn drop(&mut self) {
        self.teardown();
    }
}

unsafe extern "system" fn reader_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_GETMINMAXINFO {
            let info = lparam.0 as *mut MINMAXINFO;
            if !info.is_null() {
                (*info).ptMinTrackSize = POINT { x: MIN_W, y: MIN_H };
            }
            return LRESULT(0);
        }
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if ptr == 0 {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let reader = &mut *(ptr as *mut ReaderWindow);
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
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
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
