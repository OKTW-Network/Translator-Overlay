//! Always-on-top start / pause / stop bar with pipeline status.

use std::mem::size_of;

use tokio::sync::mpsc;
use translator_core::OverlayConfig;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::Gdi::{DeleteObject, HFONT, ScreenToClient},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{
                CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, GWL_STYLE, GWLP_USERDATA, GetSystemMetrics,
                GetWindowLongPtrW, GetWindowRect, HTCAPTION, HTCLIENT, HWND_TOPMOST, IDC_ARROW, LoadCursorW, RegisterClassExW, SM_CXSCREEN,
                SW_HIDE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
                SetWindowLongPtrW, SetWindowPos, ShowWindow, USER_DEFAULT_SCREEN_DPI, UnregisterClassW, WM_CLOSE, WM_DESTROY,
                WM_DPICHANGED, WM_GETDPISCALEDSIZE, WM_LBUTTONUP, WM_NCHITTEST, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
                WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP,
            },
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
// Fluent standard sizing, in 96 DPI epx: 40px targets, 8px between buttons,
// 12px before the status label, 14px body text.
const PAD: i32 = 8;
const BTN: i32 = 40;
const GAP: i32 = 8;
const LABEL_GAP: i32 = 12;
const STATUS_W: i32 = 200;
const BAR_H: i32 = PAD * 2 + BTN;
const BAR_W: i32 = PAD + BTN + GAP + BTN + LABEL_GAP + STATUS_W + PAD;
const FONT_PX: i32 = 14;

/// Design pixels are 96 DPI. On-screen size is `dpi / USER_DEFAULT_SCREEN_DPI`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HudLayout {
    pad: i32,
    btn: i32,
    gap: i32,
    label_gap: i32,
    bar_w: i32,
    bar_h: i32,
    font_px: i32,
}

impl HudLayout {
    fn primary(self) -> SurfaceRect {
        SurfaceRect {
            x: self.pad,
            y: self.pad,
            w: self.btn,
            h: self.btn,
        }
    }

    fn stop(self) -> SurfaceRect {
        SurfaceRect {
            x: self.pad + self.btn + self.gap,
            y: self.pad,
            w: self.btn,
            h: self.btn,
        }
    }

    fn status(self) -> SurfaceRect {
        let x = self.pad + self.btn + self.gap + self.btn + self.label_gap;
        SurfaceRect {
            x,
            y: self.pad,
            w: (self.bar_w - x - self.pad).max(1),
            h: self.btn,
        }
    }
}

fn scale_px(px: i32, dpi: u32) -> i32 {
    let base = i64::from(USER_DEFAULT_SCREEN_DPI);
    let v = (i64::from(px) * i64::from(dpi.max(1)) + base / 2) / base;
    i32::try_from(v).unwrap_or(i32::MAX).max(1)
}

fn hud_layout(dpi: u32) -> HudLayout {
    let pad = scale_px(PAD, dpi);
    let btn = scale_px(BTN, dpi);
    let gap = scale_px(GAP, dpi);
    let label_gap = scale_px(LABEL_GAP, dpi);
    let status_w = scale_px(STATUS_W, dpi);
    HudLayout {
        pad,
        btn,
        gap,
        label_gap,
        bar_w: pad + btn + gap + btn + label_gap + status_w + pad,
        bar_h: pad * 2 + btn,
        font_px: scale_px(FONT_PX, dpi),
    }
}

fn window_dpi(hwnd: HWND) -> u32 {
    match unsafe { GetDpiForWindow(hwnd) } {
        0 => USER_DEFAULT_SCREEN_DPI,
        dpi => dpi,
    }
}

pub(crate) struct HudWindow {
    hwnd: HWND,
    class_atom: u16,
    config: OverlayConfig,
    surface: DibSurface,
    hfont: HFONT,
    dpi: u32,
    layout: HudLayout,
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
        let dpi = window_dpi(hwnd);
        let layout = hud_layout(dpi);
        let (x, y) = default_hud_origin(layout.bar_w);
        let _ = unsafe { SetWindowPos(hwnd, None, x, y, layout.bar_w, layout.bar_h, SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED) };

        let surface = match DibSurface::create() {
            Ok(s) => s,
            Err(e) => {
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(e);
            }
        };
        let hfont = match text::create_segoe_font(layout.font_px) {
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
            dpi,
            layout,
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

    fn hit_button(&self, pt: POINT) -> Option<OverlayEvent> {
        if contains(self.layout.primary(), pt.x, pt.y) {
            return self.snapshot.primary_enabled.then_some(OverlayEvent::HudPrimary);
        }
        if contains(self.layout.stop(), pt.x, pt.y) {
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
        let layout = self.layout;
        if contains(layout.primary(), pt.x, pt.y) || contains(layout.stop(), pt.x, pt.y) {
            LRESULT(HTCLIENT as isize)
        } else {
            LRESULT(HTCAPTION as isize)
        }
    }

    fn on_dpi_changed(&mut self, dpi: u32, suggested: *const RECT) {
        let dpi = dpi.max(1);
        if dpi == self.dpi {
            return;
        }
        self.dpi = dpi;
        self.layout = hud_layout(dpi);
        self.replace_font(self.layout.font_px);
        let (mut x, mut y) = self.window_origin();
        if !suggested.is_null() {
            let rc = unsafe { *suggested };
            x = rc.left;
            y = rc.top;
        }
        let _ = unsafe { SetWindowPos(self.hwnd, None, x, y, self.layout.bar_w, self.layout.bar_h, SWP_NOZORDER | SWP_NOACTIVATE) };
        if let Err(e) = self.repaint() {
            tracing::warn!(error = %e, "hud dpi repaint failed");
        }
    }

    fn on_get_dpi_scaled_size(&self, dpi: u32, size: *mut SIZE) -> bool {
        if size.is_null() {
            return false;
        }
        let layout = hud_layout(dpi.max(1));
        unsafe {
            (*size).cx = layout.bar_w;
            (*size).cy = layout.bar_h;
        }
        true
    }

    fn replace_font(&mut self, px: i32) {
        let Ok(font) = text::create_segoe_font(px) else {
            tracing::warn!(px, "hud font recreate failed");
            return;
        };
        if !self.hfont.is_invalid() {
            let _ = unsafe { DeleteObject(self.hfont.into()) };
        }
        self.hfont = font;
    }

    fn window_origin(&self) -> (i32, i32) {
        let mut wnd = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut wnd) }.is_err() {
            return default_hud_origin(self.layout.bar_w);
        }
        (wnd.left, wnd.top)
    }

    fn repaint(&mut self) -> Result<(), OverlayError> {
        let layout = self.layout;
        let (w, h) = (layout.bar_w, layout.bar_h);
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
            paint_button(buf, surface, layout.primary(), fg, self.snapshot.primary_enabled, |buf, surface, r, c| {
                match self.snapshot.primary {
                    HudPrimary::Play => paint_play(buf, surface, r, c),
                    HudPrimary::Pause => paint_pause(buf, surface, r, c),
                }
            });
            paint_button(buf, surface, layout.stop(), fg, self.snapshot.stop_enabled, paint_stop);
        }

        let hdc = self.surface.hdc();
        let hfont = self.hfont;
        let buf = self
            .surface
            .pixels()
            .ok_or_else(|| OverlayError::Other("hud paint bitmap missing".into()))?;
        text::draw_text_label(hdc, hfont, buf, surface, layout.status(), &self.snapshot.label, LabelStyle {
            font_px: layout.font_px,
            color: fg,
            vcenter: true,
            vertical: None,
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

fn default_hud_origin(bar_w: i32) -> (i32, i32) {
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let x = (screen_w - bar_w).max(0) / 2;
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
    let bar_w = (rect.w / 16).max(2);
    let bar_h = (rect.h / 3).max(bar_w * 2);
    let gap = bar_w;
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
        WM_GETDPISCALEDSIZE => LRESULT(i32::from(hud.on_get_dpi_scaled_size(wparam.0 as u32, lparam.0 as *mut SIZE)) as isize),
        WM_DPICHANGED => {
            hud.on_dpi_changed((wparam.0 as u32) & 0xFFFF, lparam.0 as *const RECT);
            LRESULT(0)
        }
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
    fn layout_scales_from_96_dpi() {
        let here = hud_layout(USER_DEFAULT_SCREEN_DPI);
        assert_eq!((here.bar_w, here.bar_h, here.font_px), (BAR_W, BAR_H, FONT_PX));
        let up = hud_layout(USER_DEFAULT_SCREEN_DPI * 2);
        assert_eq!((up.bar_w, up.bar_h, up.font_px), (BAR_W * 2, BAR_H * 2, FONT_PX * 2));
    }

    #[test]
    fn layout_rounds_fractional_dpi() {
        // 14px at 125% is 17.5 and rounds to 18. At 175% it is 24.5 and rounds to 25.
        let at_125 = hud_layout(120);
        assert_eq!((at_125.pad, at_125.btn, at_125.gap, at_125.label_gap, at_125.font_px), (10, 50, 10, 15, 18));
        assert_eq!((at_125.bar_w, at_125.bar_h), (395, 70));

        let at_150 = hud_layout(144);
        assert_eq!((at_150.pad, at_150.btn, at_150.gap, at_150.label_gap, at_150.font_px), (12, 60, 12, 18, 21));
        assert_eq!((at_150.bar_w, at_150.bar_h), (474, 84));

        let at_175 = hud_layout(168);
        assert_eq!((at_175.pad, at_175.btn, at_175.gap, at_175.label_gap, at_175.font_px), (14, 70, 14, 21, 25));
        assert_eq!((at_175.bar_w, at_175.bar_h), (553, 98));
    }

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

    #[test]
    fn dpi_change_scales_from_96_dpi() {
        let _lock = crate::host::win32::lock_hwnd_tests();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut hud = HudWindow::create(&OverlayConfig::default(), tx).expect("hud create");
        let created = hud_layout(hud.dpi);
        assert_eq!(hud.surface.size(), (created.bar_w, created.bar_h));
        let next = hud.dpi.saturating_mul(2);
        hud.on_dpi_changed(next, std::ptr::null());
        let scaled = hud_layout(next);
        assert_eq!(hud.surface.size(), (scaled.bar_w, scaled.bar_h));
        hud.teardown();
    }
}
