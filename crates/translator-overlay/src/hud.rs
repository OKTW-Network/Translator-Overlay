//! Always-on-top start / pause / stop bar with pipeline status.

use std::mem::size_of;

use tokio::sync::mpsc;
use translator_core::OverlayConfig;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{DeleteObject, HFONT, ScreenToClient},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, GWL_STYLE, GWLP_USERDATA, GetClientRect, GetSystemMetrics,
            GetWindowLongPtrW, GetWindowRect, HTCAPTION, HTCLIENT, HWND_TOPMOST, IDC_ARROW, LoadCursorW, RegisterClassExW, SM_CXSCREEN,
            SW_HIDE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
            SetWindowLongPtrW, SetWindowPos, ShowWindow, UnregisterClassW, WM_CLOSE, WM_DESTROY, WM_LBUTTONUP, WM_NCHITTEST, WNDCLASSEXW,
            WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP,
        },
    },
    core::{PCWSTR, w},
};

use crate::{
    command::{HudPrimary, HudSnapshot, OverlayEvent},
    error::OverlayError,
    gfx::{
        draw::{self, Rgba, SurfaceRect},
        surface::DibSurface,
        text::{self, LabelStyle},
    },
    host::win32::ensure_topmost,
};

const CLASS_NAME: PCWSTR = w!("TranslatorOverlayHud.v1");
const PAD: i32 = 10;
const BTN: i32 = 48;
const GAP: i32 = 10;
const STATUS_W: i32 = 280;
const BAR_H: i32 = PAD * 2 + BTN;
const BAR_W: i32 = PAD + BTN + GAP + BTN + GAP + STATUS_W + PAD;
const FONT_PX: i32 = 20;

pub(crate) struct HudWindow {
    hwnd: HWND,
    class_atom: u16,
    config: OverlayConfig,
    surface: DibSurface,
    hfont: HFONT,
    snapshot: HudSnapshot,
    dismissed: bool,
    event_tx: mpsc::UnboundedSender<OverlayEvent>,
}

impl HudWindow {
    pub(crate) fn create(config: &OverlayConfig, event_tx: mpsc::UnboundedSender<OverlayEvent>) -> Result<Box<Self>, OverlayError> {
        let hinstance = unsafe { GetModuleHandleW(None) }.map_err(|e| OverlayError::Other(format!("GetModuleHandleW: {e}")))?;

        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(hud_wnd_proc),
            hInstance: hinstance.into(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.map_err(|e| OverlayError::Other(format!("LoadCursorW: {e}")))?,
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
                w!("Translator Overlay HUD"),
                WS_OVERLAPPED,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                BAR_W,
                BAR_H,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .map_err(|e| OverlayError::Other(format!("CreateWindowExW(hud): {e}")))?;
        unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, WS_POPUP.0 as isize) };
        let (x, y) = default_hud_origin();
        let _ = unsafe { SetWindowPos(hwnd, None, x, y, BAR_W, BAR_H, SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED) };

        let surface = match DibSurface::create() {
            Ok(s) => s,
            Err(e) => {
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(e);
            }
        };
        let hfont = match text::create_segoe_font(FONT_PX) {
            Ok(font) => font,
            Err(e) => {
                drop(surface);
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(e);
            }
        };

        let mut hud = Box::new(Self {
            hwnd,
            class_atom: atom,
            config: config.clone(),
            surface,
            hfont,
            snapshot: HudSnapshot {
                label: String::new(),
                primary: HudPrimary::Play,
                primary_enabled: false,
                stop_enabled: false,
            },
            dismissed: false,
            event_tx,
        });
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, &raw mut *hud as isize) };
        if let Err(e) = hud.repaint() {
            hud.teardown();
            return Err(e);
        }
        if config.hud_enabled {
            hud.show();
        } else {
            hud.hide();
        }
        Ok(hud)
    }

    pub(crate) fn apply_snapshot(&mut self, snapshot: HudSnapshot) {
        self.restore_topmost_if_needed();
        if self.snapshot != snapshot {
            self.snapshot = snapshot;
            if let Err(e) = self.repaint() {
                tracing::warn!(error = %e, "hud repaint failed");
            }
        }
    }

    pub(crate) fn apply_config(&mut self, config: &OverlayConfig) {
        let was_enabled = self.config.hud_enabled;
        self.config = config.clone();
        if config.hud_enabled && !was_enabled {
            self.dismissed = false;
        }
        if let Err(e) = self.repaint() {
            tracing::warn!(error = %e, "hud config repaint failed");
        }
        self.sync_visibility(config.hud_enabled);
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

    fn restore_topmost_if_needed(&self) {
        if self.config.hud_enabled && !self.dismissed {
            ensure_topmost(self.hwnd);
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
            tracing::warn!(error = %e, "hud present failed");
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
            return (BAR_W, BAR_H);
        }
        ((client.right - client.left).max(1), (client.bottom - client.top).max(1))
    }

    fn primary_rect() -> SurfaceRect {
        SurfaceRect {
            x: PAD,
            y: PAD,
            w: BTN,
            h: BTN,
        }
    }

    fn stop_rect() -> SurfaceRect {
        SurfaceRect {
            x: PAD + BTN + GAP,
            y: PAD,
            w: BTN,
            h: BTN,
        }
    }

    fn status_rect(w: i32, h: i32) -> SurfaceRect {
        let x = PAD + BTN + GAP + BTN + GAP;
        SurfaceRect {
            x,
            y: PAD,
            w: (w - x - PAD).max(1),
            h: (h - PAD * 2).max(1),
        }
    }

    fn hit_button(&self, pt: POINT) -> Option<OverlayEvent> {
        let p = Self::primary_rect();
        if contains(p, pt.x, pt.y) {
            return self.snapshot.primary_enabled.then_some(OverlayEvent::HudPrimary);
        }
        let s = Self::stop_rect();
        if contains(s, pt.x, pt.y) {
            return self.snapshot.stop_enabled.then_some(OverlayEvent::HudStop);
        }
        None
    }

    fn on_click(&self, lparam: LPARAM) {
        let packed = lparam.0 as u32;
        let x = (packed & 0xFFFF) as i16 as i32;
        let y = ((packed >> 16) & 0xFFFF) as i16 as i32;
        if let Some(ev) = self.hit_button(POINT { x, y }) {
            let _ = self.event_tx.send(ev);
        }
    }

    fn hit_test(&self, lparam: LPARAM) -> LRESULT {
        let packed = lparam.0 as u32;
        let sx = (packed & 0xFFFF) as i16 as i32;
        let sy = ((packed >> 16) & 0xFFFF) as i16 as i32;
        let mut pt = POINT { x: sx, y: sy };
        if !unsafe { ScreenToClient(self.hwnd, &mut pt) }.as_bool() {
            return LRESULT(HTCAPTION as isize);
        }
        let p = Self::primary_rect();
        let s = Self::stop_rect();
        if contains(p, pt.x, pt.y) || contains(s, pt.x, pt.y) {
            LRESULT(HTCLIENT as isize)
        } else {
            LRESULT(HTCAPTION as isize)
        }
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
                .ok_or_else(|| OverlayError::Other("hud paint bitmap missing".into()))?;
            buf.fill(0);
            draw::fill_rect(buf, surface, SurfaceRect { x: 0, y: 0, w, h }, bg);
            paint_button(buf, surface, Self::primary_rect(), fg, self.snapshot.primary_enabled, |buf, surface, r, c| {
                match self.snapshot.primary {
                    HudPrimary::Play => paint_play(buf, surface, r, c),
                    HudPrimary::Pause => paint_pause(buf, surface, r, c),
                }
            });
            paint_button(buf, surface, Self::stop_rect(), fg, self.snapshot.stop_enabled, paint_stop);
        }

        let hdc = self.surface.hdc();
        let hfont = self.hfont;
        let buf = self
            .surface
            .pixels()
            .ok_or_else(|| OverlayError::Other("hud paint bitmap missing".into()))?;
        text::draw_text_label(hdc, hfont, buf, surface, Self::status_rect(w, h), &self.snapshot.label, LabelStyle {
            font_px: FONT_PX,
            color: fg,
            vcenter: true,
        })?;
        self.present()
    }

    fn present(&mut self) -> Result<(), OverlayError> {
        let (bw, bh) = self.surface.size();
        let mut wnd = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut wnd) }.is_err() {
            return Err(OverlayError::Other("GetWindowRect(hud) failed".into()));
        }
        self.surface.present(self.hwnd, wnd.left, wnd.top, bw, bh)
    }
}

impl Drop for HudWindow {
    fn drop(&mut self) {
        self.teardown();
    }
}

fn contains(r: SurfaceRect, x: i32, y: i32) -> bool {
    x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h
}

fn default_hud_origin() -> (i32, i32) {
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let x = (screen_w - BAR_W).max(0) / 2;
    (x, 12)
}

fn paint_button(
    buf: &mut [u8],
    surface: draw::SurfaceSize,
    rect: SurfaceRect,
    fg: Rgba,
    enabled: bool,
    icon: impl FnOnce(&mut [u8], draw::SurfaceSize, SurfaceRect, Rgba),
) {
    let fill = if enabled {
        Rgba::new(fg.r, fg.g, fg.b, 40)
    } else {
        Rgba::new(fg.r, fg.g, fg.b, 16)
    };
    draw::fill_rect(buf, surface, rect, fill);
    let icon_color = if enabled { fg } else { Rgba::new(fg.r, fg.g, fg.b, fg.a / 3) };
    icon(buf, surface, rect, icon_color);
}

fn paint_play(buf: &mut [u8], surface: draw::SurfaceSize, rect: SurfaceRect, color: Rgba) {
    let cx = rect.x + rect.w / 2;
    let cy = rect.y + rect.h / 2;
    let s = (rect.w.min(rect.h) / 5).max(4);
    let left = cx - s / 2;
    for dy in 0..=s {
        let w = (3 * (s - dy) / 2).max(1);
        draw::fill_rect(
            buf,
            surface,
            SurfaceRect {
                x: left,
                y: cy - dy,
                w,
                h: 1,
            },
            color,
        );
        if dy > 0 {
            draw::fill_rect(
                buf,
                surface,
                SurfaceRect {
                    x: left,
                    y: cy + dy,
                    w,
                    h: 1,
                },
                color,
            );
        }
    }
}

fn paint_pause(buf: &mut [u8], surface: draw::SurfaceSize, rect: SurfaceRect, color: Rgba) {
    let cx = rect.x + rect.w / 2;
    let cy = rect.y + rect.h / 2;
    let bar_w = 3;
    let bar_h = (rect.h / 3).max(8);
    let gap = 3;
    draw::fill_rect(
        buf,
        surface,
        SurfaceRect {
            x: cx - gap - bar_w,
            y: cy - bar_h / 2,
            w: bar_w,
            h: bar_h,
        },
        color,
    );
    draw::fill_rect(
        buf,
        surface,
        SurfaceRect {
            x: cx + gap,
            y: cy - bar_h / 2,
            w: bar_w,
            h: bar_h,
        },
        color,
    );
}

fn paint_stop(buf: &mut [u8], surface: draw::SurfaceSize, rect: SurfaceRect, color: Rgba) {
    let s = (rect.w.min(rect.h) / 3).max(8);
    draw::fill_rect(
        buf,
        surface,
        SurfaceRect {
            x: rect.x + (rect.w - s) / 2,
            y: rect.y + (rect.h - s) / 2,
            w: s,
            h: s,
        },
        color,
    );
}

unsafe extern "system" fn hud_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let hud = unsafe { &mut *(ptr as *mut HudWindow) };
    match msg {
        WM_NCHITTEST => hud.hit_test(lparam),
        WM_LBUTTONUP => {
            hud.on_click(lparam);
            LRESULT(0)
        }
        WM_CLOSE => {
            hud.dismiss();
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
    use super::*;

    #[test]
    fn contains_button_rect() {
        let r = SurfaceRect { x: 8, y: 8, w: 32, h: 32 };
        assert!(contains(r, 8, 8));
        assert!(contains(r, 39, 39));
        assert!(!contains(r, 40, 8));
        assert!(!contains(r, 8, 40));
    }

    #[test]
    fn hud_window_creates_and_tears_down() {
        let _lock = crate::host::win32::lock_hwnd_tests();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut hud = HudWindow::create(&OverlayConfig::default(), tx).expect("hud create");
        hud.apply_snapshot(HudSnapshot {
            label: "Idle".into(),
            primary: HudPrimary::Play,
            primary_enabled: true,
            stop_enabled: false,
        });
        hud.teardown();
    }

    #[test]
    fn apply_snapshot_restores_topmost_without_content_change() {
        use windows::Win32::UI::WindowsAndMessaging::{HWND_NOTOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos};

        let _lock = crate::host::win32::lock_hwnd_tests();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut hud = HudWindow::create(
            &OverlayConfig {
                hud_enabled: true,
                ..OverlayConfig::default()
            },
            tx,
        )
        .expect("hud create");
        let snap = hud.snapshot.clone();
        let _ = unsafe { SetWindowPos(hud.hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
        assert!(!crate::host::win32::window_is_topmost(hud.hwnd), "HWND_NOTOPMOST should drop WS_EX_TOPMOST");
        hud.apply_snapshot(snap);
        assert!(crate::host::win32::window_is_topmost(hud.hwnd), "apply_snapshot should restore topmost");
        hud.teardown();
    }
}
