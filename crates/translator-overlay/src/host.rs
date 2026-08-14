//! Win32 layered-window host thread.
//!
//! Target follow uses out-of-context `SetWinEventHook`. The overlay thread
//! already pumps messages, so the hooks live here. `WM_APP` wakes `WaitMessage`
//! when an [`OverlayCommand`] arrives or a WinEvent needs a sync.

use std::{
    mem::size_of,
    sync::{
        atomic::{AtomicBool, AtomicIsize, AtomicU8, AtomicU32, Ordering},
        mpsc::{Receiver, Sender},
    },
};

use tracing::{debug, warn};
use translator_core::{NormRect, OverlayConfig, TranslatedBlock};
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, ClientToScreen, CreateCompatibleDC,
            CreateDIBSection, DIB_RGB_COLORS, DT_CALCRECT, DT_EDITCONTROL, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_TOP, DT_WORDBREAK,
            DeleteDC, DeleteObject, DrawTextW, GetDC, GetTextMetricsW, HALFTONE, HBITMAP, HDC, HFONT, HGDIOBJ, ReleaseDC, SelectObject,
            SetStretchBltMode, StretchBlt, TEXTMETRICW,
        },
        System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
        UI::{
            Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
            Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
                EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, GA_ROOT,
                GWL_EXSTYLE, GetAncestor, GetClientRect, GetForegroundWindow, GetWindowLongPtrW, HTCLIENT, HTTRANSPARENT, HWND_NOTOPMOST,
                HWND_TOPMOST, IDC_ARROW, IDC_CROSS, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IsChild, IsIconic,
                IsWindow, IsWindowVisible, LoadCursorW, MA_NOACTIVATE, MSG, OBJID_WINDOW, PM_REMOVE, PeekMessageW, PostQuitMessage,
                PostThreadMessageW, RegisterClassExW, SW_HIDE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_HIDEWINDOW, SWP_NOACTIVATE,
                SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetCursor, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
                ShowWindow, TranslateMessage, ULW_ALPHA, UnregisterClassW, UpdateLayeredWindow, WINDOW_EX_STYLE, WINEVENT_OUTOFCONTEXT,
                WM_APP, WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCHITTEST, WM_QUIT, WM_RBUTTONUP,
                WM_SETCURSOR, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP, WaitMessage,
            },
        },
    },
    core::{PCWSTR, w},
};

use crate::{
    OverlayError,
    draw::{self, Rgba, SurfaceRect},
    layout::{label_pad, place_label},
    picker::{HANDLE_SIZE, PickerAction, PickerCursor, RegionPicker},
    reader::{ReaderWindow, format_reader_text},
    text::{self, LabelStyle},
};

/// `wnd_proc` cannot reach `OverlayHost`; picker hit-testing is a process-wide flag
/// because this crate hosts a single overlay window.
static PICKER_HIT_TEST: AtomicBool = AtomicBool::new(false);
/// Last picker cursor. `WM_SETCURSOR` is sent (not posted) and hits `wnd_proc`
/// inside `PeekMessage` / `WaitMessage`, so the Peek-loop swallow cannot win.
static PICKER_CURSOR: AtomicU8 = AtomicU8::new(0);

static FOLLOW_TARGET: AtomicIsize = AtomicIsize::new(0);
static FOLLOW_THREAD: AtomicU32 = AtomicU32::new(0);
static FOLLOW_SYNC: AtomicBool = AtomicBool::new(false);

const CLASS_NAME: PCWSTR = w!("TranslatorOverlayLayer.v1");

pub enum OverlayCommand {
    Attach {
        target_hwnd: isize,
    },
    Detach,
    SetBlocks {
        blocks: Vec<TranslatedBlock>,
        content_width: u32,
        content_height: u32,
    },
    Clear,
    UpdateConfig(OverlayConfig),
    BeginRegionSelect {
        regions: Vec<NormRect>,
    },
    CancelRegionSelect,
    ConfirmRegionSelect,
    ClearRegionSelect,
    Shutdown,
}

/// Overlay thread → pipeline (picker results).
#[derive(Debug, Clone)]
pub enum OverlayEvent {
    RegionsCommitted(Vec<NormRect>),
    RegionSelectCancelled,
    RegionSelectUpdated(Vec<NormRect>),
}

pub struct OverlayHost {
    hwnd: HWND,
    class_atom: u16,
    config: OverlayConfig,
    target: Option<HWND>,
    blocks: Vec<TranslatedBlock>,
    content_w: u32,
    content_h: u32,
    /// Paint buffer size in **OCR / capture content** pixels (not screen client).
    surface_w: i32,
    surface_h: i32,
    dirty: bool,
    hdc_screen: HDC,
    hdc_mem: HDC,
    hbmp: HBITMAP,
    bits: *mut u8,
    bmp_w: i32,
    bmp_h: i32,
    /// Optional stretched present buffer when client size ≠ content size.
    hdc_present: HDC,
    hbmp_present: HBITMAP,
    present_w: i32,
    present_h: i32,
    hfont: HFONT,
    font_px: i32,
    reader: Option<Box<ReaderWindow>>,
    picker: Option<RegionPicker>,
    event_tx: Sender<OverlayEvent>,
    follow_hooks: [HWINEVENTHOOK; 3],
}

impl OverlayHost {
    pub fn create(config: OverlayConfig, event_tx: Sender<OverlayEvent>) -> Result<Self, OverlayError> {
        unsafe {
            let hinstance = GetModuleHandleW(None).map_err(|e| OverlayError::Other(format!("GetModuleHandleW: {e}")))?;

            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).map_err(|e| OverlayError::Other(format!("LoadCursorW: {e}")))?,
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };

            let atom = RegisterClassExW(&wc);
            if atom == 0 {
                // Class may already exist from a previous run in the same process.
                // Continue — CreateWindowEx will still work if registered.
            }

            // Not TOPMOST: only float above the target while it is in the
            // foreground; otherwise we hide so other apps are not covered.
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("Translator Overlay"),
                WS_POPUP,
                CW_USEDEFAULT,
                0,
                100,
                100,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
            .map_err(|e| OverlayError::Other(format!("CreateWindowExW: {e}")))?;

            let hdc_screen = GetDC(Some(hwnd));
            if hdc_screen.is_invalid() {
                let _ = DestroyWindow(hwnd);
                return Err(OverlayError::Other("GetDC failed".into()));
            }
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            if hdc_mem.is_invalid() {
                ReleaseDC(Some(hwnd), hdc_screen);
                let _ = DestroyWindow(hwnd);
                return Err(OverlayError::Other("CreateCompatibleDC failed".into()));
            }

            let font_px = 16;
            let hfont = text::create_segoe_font(font_px)?;

            let hdc_present = CreateCompatibleDC(Some(hdc_screen));
            if hdc_present.is_invalid() {
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(Some(hwnd), hdc_screen);
                let _ = DestroyWindow(hwnd);
                return Err(OverlayError::Other("CreateCompatibleDC(present) failed".into()));
            }

            let mut host = Self {
                hwnd,
                class_atom: atom,
                config,
                target: None,
                blocks: Vec::new(),
                content_w: 0,
                content_h: 0,
                surface_w: 0,
                surface_h: 0,
                dirty: true,
                hdc_screen,
                hdc_mem,
                hbmp: HBITMAP::default(),
                bits: std::ptr::null_mut(),
                bmp_w: 0,
                bmp_h: 0,
                hdc_present,
                hbmp_present: HBITMAP::default(),
                present_w: 0,
                present_h: 0,
                hfont,
                font_px,
                reader: None,
                picker: None,
                event_tx,
                follow_hooks: [HWINEVENTHOOK::default(); 3],
            };

            FOLLOW_THREAD.store(GetCurrentThreadId(), Ordering::Release);

            let _ = ShowWindow(hwnd, SW_HIDE);
            host.ensure_bitmap(100, 100)?;
            match ReaderWindow::create(&host.config) {
                Ok(reader) => host.reader = Some(reader),
                Err(e) => {
                    host.teardown();
                    return Err(e);
                }
            }
            Ok(host)
        }
    }

    pub fn run(&mut self, rx: Receiver<OverlayCommand>) {
        self.follow_hooks = install_follow_hooks();

        loop {
            let mut sync = false;
            loop {
                match rx.try_recv() {
                    Ok(OverlayCommand::Shutdown) => {
                        self.teardown();
                        return;
                    }
                    Ok(cmd) => {
                        self.handle(cmd);
                        sync = true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.teardown();
                        return;
                    }
                }
            }

            if self.drain_thread_messages(&mut sync) {
                self.teardown();
                return;
            }

            if FOLLOW_SYNC.swap(false, Ordering::AcqRel) || sync {
                self.tick();
            }

            if unsafe { WaitMessage() }.is_err() {
                warn!("WaitMessage failed");
                self.teardown();
                return;
            }
        }
    }

    /// Returns `true` when the thread should exit (`WM_QUIT`).
    fn drain_thread_messages(&mut self, sync: &mut bool) -> bool {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    return true;
                }
                if msg.hwnd.is_invalid() && msg.message == WM_APP {
                    continue;
                }
                if self.picker.is_some() && msg.hwnd == self.hwnd && is_picker_message(msg.message) {
                    self.dispatch_picker_msg(&msg);
                    *sync = true;
                    continue;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        false
    }

    fn handle(&mut self, cmd: OverlayCommand) {
        match cmd {
            OverlayCommand::Attach { target_hwnd } => {
                let hwnd = HWND(target_hwnd as *mut _);
                if unsafe { IsWindow(Some(hwnd)).as_bool() } {
                    self.target = Some(hwnd);
                    FOLLOW_TARGET.store(target_hwnd, Ordering::Release);
                    self.dirty = true;
                    debug!(?target_hwnd, "overlay attached");
                } else {
                    warn!(?target_hwnd, "attach ignored — invalid hwnd");
                    self.target = None;
                    FOLLOW_TARGET.store(0, Ordering::Release);
                }
            }
            OverlayCommand::Detach => {
                if self.picker.is_some() {
                    self.finish_picker(PickerEnd::Cancel);
                }
                self.target = None;
                FOLLOW_TARGET.store(0, Ordering::Release);
                self.hide();
            }
            OverlayCommand::SetBlocks {
                blocks,
                content_width,
                content_height,
            } => {
                let size_changed = self.content_w != content_width || self.content_h != content_height;
                self.blocks = blocks;
                self.content_w = content_width;
                self.content_h = content_height;
                if size_changed {
                    self.surface_w = content_width as i32;
                    self.surface_h = content_height as i32;
                }
                self.dirty = true;
                if let Some(reader) = self.reader.as_mut() {
                    reader.set_text(&format_reader_text(&self.blocks));
                }
            }
            OverlayCommand::Clear => {
                self.blocks.clear();
                self.content_w = 0;
                self.content_h = 0;
                self.dirty = true;
                self.hide();
                if let Some(reader) = self.reader.as_mut() {
                    reader.set_text("");
                }
            }
            OverlayCommand::UpdateConfig(cfg) => {
                self.config = cfg;
                self.dirty = true;
                if let Some(reader) = self.reader.as_mut() {
                    reader.apply_config(&self.config);
                }
                if !self.config.enabled && self.picker.is_none() {
                    self.hide();
                }
            }
            OverlayCommand::BeginRegionSelect { regions } => self.begin_picker(regions),
            OverlayCommand::CancelRegionSelect => {
                if self.picker.is_some() {
                    self.finish_picker(PickerEnd::Cancel);
                }
            }
            OverlayCommand::ConfirmRegionSelect => {
                if self.picker.is_some() {
                    self.finish_picker(PickerEnd::Confirm);
                }
            }
            OverlayCommand::ClearRegionSelect => {
                if let Some(p) = self.picker.as_mut() {
                    p.regions.clear();
                    p.selected = None;
                    let _ = self.event_tx.send(OverlayEvent::RegionSelectUpdated(Vec::new()));
                    self.dirty = true;
                }
            }
            OverlayCommand::Shutdown => {}
        }
    }

    fn begin_picker(&mut self, regions: Vec<NormRect>) {
        let (cw, ch) = self
            .target
            .and_then(client_screen_rect)
            .map(|(_, _, w, h)| (w, h))
            .unwrap_or((800, 600));
        self.picker = Some(RegionPicker::new(regions, cw, ch));
        self.set_click_through(false);
        PICKER_HIT_TEST.store(true, Ordering::Relaxed);
        set_picker_cursor(PickerCursor::Cross);
        self.dirty = true;
        // Dashboard just received the click, so this process may set foreground.
        // Bring the target up so the picker is visible without an extra click.
        if let Some(target) = self.target {
            unsafe {
                let _ = SetForegroundWindow(target);
            }
        }
    }

    fn finish_picker(&mut self, end: PickerEnd) {
        let Some(picker) = self.picker.take() else {
            return;
        };
        self.set_click_through(true);
        PICKER_HIT_TEST.store(false, Ordering::Relaxed);
        unsafe {
            let _ = ReleaseCapture();
        }
        match end {
            PickerEnd::Confirm => {
                let _ = self.event_tx.send(OverlayEvent::RegionsCommitted(picker.regions));
            }
            PickerEnd::Cancel => {
                let _ = self.event_tx.send(OverlayEvent::RegionSelectCancelled);
            }
        }
        self.dirty = true;
        if self.blocks.is_empty() || !self.config.enabled {
            self.hide();
        }
    }

    fn set_click_through(&self, through: bool) {
        unsafe {
            let raw = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) as u32;
            let mut style = WINDOW_EX_STYLE(raw);
            if through {
                style |= WS_EX_TRANSPARENT;
            } else {
                style &= !WS_EX_TRANSPARENT;
            }
            SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, style.0 as isize);
            let _ = SetWindowPos(self.hwnd, None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
        }
    }

    fn tick(&mut self) {
        if self.picker.is_some() {
            self.tick_picker();
            return;
        }

        if !self.config.enabled {
            self.hide();
            return;
        }

        let Some(target) = self.target else {
            return;
        };

        unsafe {
            if !IsWindow(Some(target)).as_bool() {
                debug!("target window gone — detaching overlay");
                self.target = None;
                self.hide();
                return;
            }
            if IsIconic(target).as_bool() || !IsWindowVisible(target).as_bool() {
                self.hide();
                return;
            }

            // Only show while the capture target (or one of its children) is
            // the foreground window.
            let fg = GetForegroundWindow();
            if !is_target_in_foreground(target, fg) {
                self.hide();
                return;
            }

            // Align to **client area** (matches cropped capture frames).
            let Some((x, y, client_w, client_h)) = client_screen_rect(target) else {
                return;
            };

            if self.blocks.is_empty() || self.content_w == 0 || self.content_h == 0 {
                self.hide();
                return;
            }

            // Paint in capture/OCR pixel space (1:1 with bboxes), then stretch to
            // the live client rect. Avoids DPI / DWM size drift mis-mapping boxes.
            let paint_w = self.content_w as i32;
            let paint_h = self.content_h as i32;
            let size_changed = paint_w != self.surface_w || paint_h != self.surface_h;
            if size_changed {
                self.surface_w = paint_w;
                self.surface_h = paint_h;
                self.dirty = true;
            }

            if self.dirty {
                if let Err(e) = self.repaint() {
                    warn!(error = %e, "overlay repaint failed");
                    return;
                }
                self.dirty = false;
            }

            // TOPMOST only while target is focused — otherwise other apps get covered.
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), x, y, client_w, client_h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);

            if let Err(e) = self.present(x, y, client_w, client_h) {
                warn!(error = %e, "UpdateLayeredWindow failed");
            }
        }
    }

    fn tick_picker(&mut self) {
        let Some(target) = self.target else {
            self.finish_picker(PickerEnd::Cancel);
            return;
        };

        unsafe {
            if !IsWindow(Some(target)).as_bool() {
                debug!("target window gone — cancelling region picker");
                self.target = None;
                self.finish_picker(PickerEnd::Cancel);
                return;
            }
            if IsIconic(target).as_bool() || !IsWindowVisible(target).as_bool() {
                self.abort_picker_drag();
                self.hide();
                return;
            }

            // Same rule as the translation overlay: only cover the target
            // while it (or a child) is the foreground window. The picker
            // itself must also count — a clickable TOPMOST layer often
            // becomes GetForegroundWindow despite WS_EX_NOACTIVATE, and
            // hiding on that would abort the drag. Keep picker state so
            // Dashboard Done/Cancel/Clear still apply after hide.
            let dragging = self.picker.as_ref().is_some_and(RegionPicker::is_dragging);
            let fg = GetForegroundWindow();
            if !is_picker_allowed_foreground(target, self.hwnd, fg) && !dragging {
                self.abort_picker_drag();
                self.hide();
                return;
            }

            let Some((x, y, client_w, client_h)) = client_screen_rect(target) else {
                return;
            };

            if let Some(p) = self.picker.as_mut() {
                p.set_client_size(client_w, client_h);
            }

            self.surface_w = client_w;
            self.surface_h = client_h;

            if let Err(e) = self.repaint_picker() {
                warn!(error = %e, "picker repaint failed");
                return;
            }

            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), x, y, client_w, client_h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);

            if let Err(e) = self.present(x, y, client_w, client_h) {
                warn!(error = %e, "UpdateLayeredWindow failed (picker)");
            }
        }
    }

    fn hide(&mut self) {
        unsafe {
            // Drop topmost so we never stay above unrelated apps after hide.
            let _ = SetWindowPos(self.hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW);
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    fn abort_picker_drag(&mut self) {
        if let Some(p) = self.picker.as_mut() {
            p.cancel_drag();
        }
        unsafe {
            let _ = ReleaseCapture();
        }
    }

    fn dispatch_picker_msg(&mut self, msg: &MSG) {
        let (px, py) = mouse_pos(msg.lParam);
        match msg.message {
            WM_LBUTTONDOWN => {
                if let Some(p) = self.picker.as_mut() {
                    p.on_left_down(px, py);
                    unsafe {
                        let _ = SetCapture(self.hwnd);
                    }
                    self.dirty = true;
                }
            }
            WM_MOUSEMOVE => {
                if let Some(p) = self.picker.as_mut() {
                    let hit = p.on_move(px, py);
                    set_picker_cursor(hit.cursor());
                    self.dirty = true;
                }
            }
            WM_LBUTTONUP => {
                unsafe {
                    let _ = ReleaseCapture();
                }
                let action = self.picker.as_mut().map(|p| p.on_left_up(px, py)).unwrap_or(PickerAction::None);
                self.apply_picker_action(action);
            }
            WM_RBUTTONUP => {
                let action = self.picker.as_mut().map(|p| p.on_right_up(px, py)).unwrap_or(PickerAction::None);
                self.apply_picker_action(action);
            }
            _ => {}
        }
    }

    fn apply_picker_action(&mut self, action: PickerAction) {
        match action {
            PickerAction::None => self.dirty = true,
            PickerAction::RegionsChanged => {
                if let Some(p) = self.picker.as_ref() {
                    let _ = self.event_tx.send(OverlayEvent::RegionSelectUpdated(p.regions.clone()));
                }
                self.dirty = true;
            }
        }
    }

    fn repaint_picker(&mut self) -> Result<(), OverlayError> {
        let w = self.surface_w.max(1);
        let h = self.surface_h.max(1);
        self.ensure_bitmap(w, h)?;

        let len = (w as usize) * (h as usize) * 4;
        let buf = unsafe { std::slice::from_raw_parts_mut(self.bits, len) };
        draw::clear(buf);

        let surface = draw::SurfaceSize::new(w, h);
        // UpdateLayeredWindow hit-tests per-pixel alpha *before* WM_NCHITTEST.
        // Alpha 0 pixels are click-through, so the whole client must have a
        // non-zero veil or empty areas cannot start a drag.
        draw::fill_rect(buf, surface, SurfaceRect { x: 0, y: 0, w, h }, Rgba::new(6, 14, 24, 20));
        let bounds = Rgba::new(0, 200, 255, 220);
        let region_stroke = Rgba::new(80, 220, 255, 230);
        let selected_stroke = Rgba::new(255, 210, 60, 255);
        let fill = Rgba::new(80, 220, 255, 24);
        let handle = Rgba::new(255, 255, 255, 240);
        let text = Rgba::new(255, 255, 255, 255);

        // Selectable area = full client; inset so the stroke is not clipped.
        draw::stroke_rect(
            buf,
            surface,
            SurfaceRect {
                x: 2,
                y: 2,
                w: (w - 4).max(1),
                h: (h - 4).max(1),
            },
            bounds,
            2,
        );

        let (rects, band) = {
            let Some(picker) = self.picker.as_ref() else {
                return Ok(());
            };
            (picker.live_pixel_rects(), picker.rubber_band())
        };

        for (i, (pr, selected)) in rects.into_iter().enumerate() {
            let stroke = if selected { selected_stroke } else { region_stroke };
            let thick = if selected { 3 } else { 2 };
            draw::fill_rect(buf, surface, pr.to_surface(), fill);
            draw::stroke_rect(buf, surface, pr.to_surface(), stroke, thick);
            for (hx, hy) in [(pr.x, pr.y), (pr.x + pr.w, pr.y), (pr.x, pr.y + pr.h), (pr.x + pr.w, pr.y + pr.h)] {
                draw::fill_rect(
                    buf,
                    surface,
                    SurfaceRect {
                        x: hx - HANDLE_SIZE / 2,
                        y: hy - HANDLE_SIZE / 2,
                        w: HANDLE_SIZE,
                        h: HANDLE_SIZE,
                    },
                    handle,
                );
            }
            let label = format!("{}", i + 1);
            let label_rect = SurfaceRect {
                x: pr.x + 4,
                y: pr.y + 4,
                w: 22,
                h: 18,
            };
            draw::fill_rect(buf, surface, label_rect, Rgba::new(0, 0, 0, 160));
            self.draw_text_label(buf, surface, label_rect, &label, LabelStyle { font_px: 13, color: text })?;
        }

        if let Some(band) = band {
            draw::stroke_rect(buf, surface, band.to_surface(), selected_stroke, 2);
        }
        Ok(())
    }

    fn ensure_bitmap(&mut self, w: i32, h: i32) -> Result<(), OverlayError> {
        if w <= 0 || h <= 0 {
            return Err(OverlayError::Other("invalid bitmap size".into()));
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
                .map_err(|e| OverlayError::Other(format!("CreateDIBSection: {e}")))?;

            if hbmp.is_invalid() || bits.is_null() {
                return Err(OverlayError::Other("CreateDIBSection returned null".into()));
            }

            let _ = SelectObject(self.hdc_mem, HGDIOBJ(hbmp.0));
            self.hbmp = hbmp;
            self.bits = bits.cast();
            self.bmp_w = w;
            self.bmp_h = h;
        }
        Ok(())
    }

    fn repaint(&mut self) -> Result<(), OverlayError> {
        let w = self.surface_w.max(1);
        let h = self.surface_h.max(1);
        self.ensure_bitmap(w, h)?;

        let len = (w as usize) * (h as usize) * 4;
        let buf = unsafe { std::slice::from_raw_parts_mut(self.bits, len) };
        draw::clear(buf);

        let bg = draw::background_rgba(&self.config);
        let fg = draw::text_rgba(&self.config);
        let surface = draw::SurfaceSize::new(w, h);

        // Layout labels first so paint order is stable.
        let content_w = self.content_w;
        let content_h = self.content_h;
        let pending: Vec<(SurfaceRect, String, u32)> = self
            .blocks
            .iter()
            .filter_map(|block| {
                let text = block.translation.trim();
                if text.is_empty() {
                    return None;
                }
                let base = draw::map_rect_to_surface(block.bbox, content_w, content_h, w, h)?;
                Some((base, text.to_string(), block.source_lines.max(1)))
            })
            .collect();

        // (rect, text, font_px) — single-line sources may shrink/widen; merged
        // paragraphs keep the OCR column width.
        let mut labels: Vec<(SurfaceRect, String, i32)> = Vec::with_capacity(pending.len());
        for (base, text, source_lines) in pending {
            let (expanded, font_px) = self.layout_label(base, &text, source_lines, surface)?;
            labels.push((expanded, text, font_px));
        }

        for (rect, _, _) in &labels {
            draw::fill_rect(buf, surface, *rect, bg);
        }
        for (rect, text, font_px) in &labels {
            self.draw_text_label(buf, surface, *rect, text, LabelStyle {
                font_px: *font_px,
                color: fg,
            })?;
        }
        Ok(())
    }

    /// Pick CreateFont height so the GDI cell fits inside the OCR glyph box.
    ///
    /// Fixed ratios still overshoot (Segoe UI cell > requested height; vertical
    /// OCR width is padded). Shrink until ascent+descent ≤ ~90% of short side.
    fn fit_font_to_source_box(&mut self, base: SurfaceRect) -> Result<i32, OverlayError> {
        let target = draw::char_box_px(base);
        // Leave a little air so ClearType stems don't look larger than source ink.
        let max_cell = ((target as f32) * 0.90).round().max(8.0) as i32;
        let mut px = draw::font_height_for(base);

        for _ in 0..6 {
            self.ensure_font(px)?;
            let cell = unsafe {
                let _ = SelectObject(self.hdc_mem, HGDIOBJ(self.hfont.0));
                let mut tm = TEXTMETRICW::default();
                if GetTextMetricsW(self.hdc_mem, &mut tm).as_bool() {
                    (tm.tmAscent + tm.tmDescent).max(1)
                } else {
                    px
                }
            };
            if cell <= max_cell {
                break;
            }
            let next = ((px as f32) * (max_cell as f32) / (cell as f32)).floor().max(8.0) as i32;
            if next >= px {
                px = (px - 1).max(8);
            } else {
                px = next;
            }
        }
        Ok(px)
    }

    /// Layout one overlay label.
    ///
    /// - Merged paragraph (`source_lines > 1`): lock width to OCR column; wrap.
    /// - Single line: shrink font (down to ~70%) to fit source width, then allow
    ///   width growth up to 1.75x if the translation is still longer.
    fn layout_label(
        &mut self,
        base: SurfaceRect,
        text: &str,
        source_lines: u32,
        surface: draw::SurfaceSize,
    ) -> Result<(SurfaceRect, i32), OverlayError> {
        let max_w = (surface.width - base.x).max(1);
        let source_w = base.w.clamp(1, max_w);
        let mut font_px = self.fit_font_to_source_box(base)?;

        if source_lines <= 1 {
            // 1) Shrink font so the translation can stay one line inside source_w.
            let min_font = ((font_px as f32) * 0.70).round().max(8.0) as i32;
            loop {
                let pad = label_pad(font_px);
                let natural = self.measure_single_line(text, font_px)?;
                let need_w = natural.0 + pad * 2;
                if need_w <= source_w || font_px <= min_font {
                    break;
                }
                font_px = (font_px - 1).max(min_font);
            }

            // 2) If still wider than source at min font, grow width (cap 1.75×).
            let pad = label_pad(font_px);
            let natural = self.measure_single_line(text, font_px)?;
            let need_w = (natural.0 + pad * 2).max(1);
            let expand_cap = ((source_w as f32) * 1.75).round() as i32;
            let box_w = if need_w <= source_w {
                source_w
            } else {
                need_w.min(expand_cap).min(max_w).max(source_w)
            };

            // Height: one line if it fits; otherwise wrap within the chosen width.
            let text_h = if need_w <= box_w {
                natural.1
            } else {
                self.measure_wrapped(text, font_px, (box_w - pad * 2).max(8))?.1
            };
            let box_h = (text_h + pad * 2).max(font_px + pad * 2);
            return Ok((place_label(base, box_w, box_h, surface), font_px));
        }

        // Multi-line source: keep OCR column width; wrap height only.
        let pad = label_pad(font_px);
        let box_w = source_w;
        let text_h = self.measure_wrapped(text, font_px, (box_w - pad * 2).max(8))?.1;
        let box_h = (text_h + pad * 2).max(font_px + pad * 2);
        Ok((place_label(base, box_w, box_h, surface), font_px))
    }

    /// Unwrapped single-line extent (width, height) in pixels.
    fn measure_single_line(&mut self, text: &str, font_px: i32) -> Result<(i32, i32), OverlayError> {
        self.ensure_font(font_px)?;
        unsafe {
            let _ = SelectObject(self.hdc_mem, HGDIOBJ(self.hfont.0));
            let mut calc = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let flags = DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX | DT_CALCRECT;
            let measured_h = DrawTextW(self.hdc_mem, &mut wide, &mut calc, flags);
            let w = (calc.right - calc.left).max(1);
            let h = if measured_h > 0 { measured_h } else { font_px + 2 };
            Ok((w, h))
        }
    }

    /// Word-wrapped extent for a fixed text area width.
    fn measure_wrapped(&mut self, text: &str, font_px: i32, text_area_w: i32) -> Result<(i32, i32), OverlayError> {
        self.ensure_font(font_px)?;
        unsafe {
            let _ = SelectObject(self.hdc_mem, HGDIOBJ(self.hfont.0));
            let mut calc = RECT {
                left: 0,
                top: 0,
                right: text_area_w.max(8),
                bottom: 0,
            };
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let flags = DT_LEFT | DT_TOP | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX | DT_CALCRECT;
            let measured_h = DrawTextW(self.hdc_mem, &mut wide, &mut calc, flags);
            let w = (calc.right - calc.left).max(1);
            let h = if measured_h > 0 { measured_h } else { font_px + 2 };
            Ok((w, h))
        }
    }

    fn ensure_font(&mut self, font_px: i32) -> Result<(), OverlayError> {
        if font_px == self.font_px && !self.hfont.is_invalid() {
            return Ok(());
        }
        unsafe {
            if !self.hfont.is_invalid() {
                let _ = DeleteObject(self.hfont.into());
            }
            self.hfont = text::create_segoe_font(font_px)?;
            self.font_px = font_px;
        }
        Ok(())
    }

    fn draw_text_label(
        &mut self,
        buf: &mut [u8],
        surface: draw::SurfaceSize,
        rect: SurfaceRect,
        text: &str,
        style: LabelStyle,
    ) -> Result<(), OverlayError> {
        self.ensure_font(style.font_px)?;
        crate::text::draw_text_label(self.hdc_mem, self.hfont, buf, surface, rect, text, style)
    }

    fn present(&mut self, x: i32, y: i32, client_w: i32, client_h: i32) -> Result<(), OverlayError> {
        if self.bits.is_null() || self.bmp_w <= 0 || self.bmp_h <= 0 {
            return Err(OverlayError::Other("paint bitmap missing".into()));
        }
        if client_w <= 0 || client_h <= 0 {
            return Err(OverlayError::Other("invalid client size".into()));
        }

        unsafe {
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let ppt_dst = POINT { x, y };
            let psize = SIZE {
                cx: client_w,
                cy: client_h,
            };
            let ppt_src = POINT { x: 0, y: 0 };

            // Same size: present paint buffer directly (common path).
            if self.bmp_w == client_w && self.bmp_h == client_h {
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
                .map_err(|e| OverlayError::Other(format!("UpdateLayeredWindow: {e}")))?;
                return Ok(());
            }

            // Stretch content-space paint into client-sized present buffer.
            self.ensure_present_bitmap(client_w, client_h)?;
            let _ = SetStretchBltMode(self.hdc_present, HALFTONE);
            let ok = StretchBlt(
                self.hdc_present,
                0,
                0,
                client_w,
                client_h,
                Some(self.hdc_mem),
                0,
                0,
                self.bmp_w,
                self.bmp_h,
                windows::Win32::Graphics::Gdi::SRCCOPY,
            );
            if !ok.as_bool() {
                return Err(OverlayError::Other("StretchBlt failed".into()));
            }

            UpdateLayeredWindow(
                self.hwnd,
                Some(self.hdc_screen),
                Some(&ppt_dst),
                Some(&psize),
                Some(self.hdc_present),
                Some(&ppt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
            .map_err(|e| OverlayError::Other(format!("UpdateLayeredWindow: {e}")))?;
        }
        Ok(())
    }

    fn ensure_present_bitmap(&mut self, w: i32, h: i32) -> Result<(), OverlayError> {
        if w <= 0 || h <= 0 {
            return Err(OverlayError::Other("invalid present size".into()));
        }
        if w == self.present_w && h == self.present_h && !self.hbmp_present.is_invalid() {
            return Ok(());
        }

        unsafe {
            if !self.hbmp_present.is_invalid() {
                let _ = SelectObject(self.hdc_present, HGDIOBJ::default());
                let _ = DeleteObject(self.hbmp_present.into());
                self.hbmp_present = HBITMAP::default();
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
            let hbmp = CreateDIBSection(Some(self.hdc_present), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                .map_err(|e| OverlayError::Other(format!("CreateDIBSection(present): {e}")))?;

            if hbmp.is_invalid() || bits.is_null() {
                return Err(OverlayError::Other("CreateDIBSection(present) returned null".into()));
            }

            let _ = SelectObject(self.hdc_present, HGDIOBJ(hbmp.0));
            self.hbmp_present = hbmp;
            self.present_w = w;
            self.present_h = h;
        }
        Ok(())
    }

    fn teardown(&mut self) {
        FOLLOW_TARGET.store(0, Ordering::Release);
        FOLLOW_THREAD.store(0, Ordering::Release);
        FOLLOW_SYNC.store(false, Ordering::Release);
        uninstall_follow_hooks(&mut self.follow_hooks);
        if let Some(mut reader) = self.reader.take() {
            reader.teardown();
        }
        unsafe {
            if !self.hwnd.is_invalid() {
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
            if !self.hbmp_present.is_invalid() {
                let _ = DeleteObject(self.hbmp_present.into());
                self.hbmp_present = HBITMAP::default();
            }
            if !self.hdc_mem.is_invalid() {
                let _ = DeleteDC(self.hdc_mem);
                self.hdc_mem = HDC::default();
            }
            if !self.hdc_present.is_invalid() {
                let _ = DeleteDC(self.hdc_present);
                self.hdc_present = HDC::default();
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
}

impl Drop for OverlayHost {
    fn drop(&mut self) {
        self.teardown();
    }
}

pub(crate) fn wake_overlay_thread() {
    let thread_id = FOLLOW_THREAD.load(Ordering::Relaxed);
    if thread_id != 0 {
        let _ = unsafe { PostThreadMessageW(thread_id, WM_APP, WPARAM(0), LPARAM(0)) };
    }
}

fn request_follow_sync() {
    if !FOLLOW_SYNC.swap(true, Ordering::AcqRel) {
        wake_overlay_thread();
    }
}

fn is_follow_target(hwnd: HWND) -> bool {
    let target = FOLLOW_TARGET.load(Ordering::Relaxed);
    target != 0 && hwnd.0 as isize == target
}

fn install_follow_hooks() -> [HWINEVENTHOOK; 3] {
    // SAFETY: callback only touches atomics and may `PostThreadMessageW` to this thread.
    unsafe {
        let foreground =
            SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT);
        let minimize =
            SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT);
        let location = SetWinEventHook(
            EVENT_OBJECT_LOCATIONCHANGE,
            EVENT_OBJECT_LOCATIONCHANGE,
            None,
            Some(on_follow_event),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        if foreground.is_invalid() || minimize.is_invalid() || location.is_invalid() {
            warn!("overlay follow WinEvent hooks failed to install");
        }
        [foreground, minimize, location]
    }
}

fn uninstall_follow_hooks(hooks: &mut [HWINEVENTHOOK; 3]) {
    for hook in hooks {
        if !hook.is_invalid() {
            let _ = unsafe { UnhookWinEvent(*hook) };
            *hook = HWINEVENTHOOK::default();
        }
    }
}

unsafe extern "system" fn on_follow_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    match event {
        EVENT_SYSTEM_FOREGROUND => request_follow_sync(),
        EVENT_SYSTEM_MINIMIZESTART | EVENT_SYSTEM_MINIMIZEEND if is_follow_target(hwnd) => request_follow_sync(),
        EVENT_OBJECT_LOCATIONCHANGE if id_object == OBJID_WINDOW.0 && is_follow_target(hwnd) => request_follow_sync(),
        _ => {}
    }
}

/// True when the picker may stay visible: target focused, or the picker
/// window itself (clicks on the layer must not hide it).
fn is_picker_allowed_foreground(target: HWND, overlay: HWND, fg: HWND) -> bool {
    if is_target_in_foreground(target, fg) {
        return true;
    }
    !overlay.is_invalid() && !fg.is_invalid() && fg == overlay
}

/// True when `fg` is the capture target or a child / same top-level tree.
fn is_target_in_foreground(target: HWND, fg: HWND) -> bool {
    unsafe {
        if fg.is_invalid() || target.is_invalid() {
            return false;
        }
        if fg == target {
            return true;
        }
        // Focused child control inside the target window.
        if IsChild(target, fg).as_bool() {
            return true;
        }
        // Same top-level root (e.g. owned popups under the target).
        let fg_root = GetAncestor(fg, GA_ROOT);
        if !fg_root.is_invalid() && fg_root == target {
            return true;
        }
        false
    }
}

/// Client-area rectangle in screen coordinates (left, top, width, height).
fn client_screen_rect(target: HWND) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let mut client = RECT::default();
        if GetClientRect(target, &mut client).is_err() {
            return None;
        }
        let mut tl = POINT {
            x: client.left,
            y: client.top,
        };
        let mut br = POINT {
            x: client.right,
            y: client.bottom,
        };
        if !ClientToScreen(target, &mut tl).as_bool() || !ClientToScreen(target, &mut br).as_bool() {
            return None;
        }
        let w = (br.x - tl.x).max(1);
        let h = (br.y - tl.y).max(1);
        Some((tl.x, tl.y, w, h))
    }
}

enum PickerEnd {
    Confirm,
    Cancel,
}

fn is_picker_message(msg: u32) -> bool {
    matches!(msg, WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEMOVE | WM_RBUTTONUP)
}

fn mouse_pos(lparam: LPARAM) -> (i32, i32) {
    let v = lparam.0 as u32;
    let x = (v & 0xFFFF) as i16 as i32;
    let y = ((v >> 16) & 0xFFFF) as i16 as i32;
    (x, y)
}

fn picker_cursor_code(kind: PickerCursor) -> u8 {
    match kind {
        PickerCursor::Cross => 0,
        PickerCursor::SizeAll => 1,
        PickerCursor::SizeNs => 2,
        PickerCursor::SizeWe => 3,
        PickerCursor::SizeNwse => 4,
        PickerCursor::SizeNesw => 5,
    }
}

fn picker_cursor_from_code(code: u8) -> PickerCursor {
    match code {
        1 => PickerCursor::SizeAll,
        2 => PickerCursor::SizeNs,
        3 => PickerCursor::SizeWe,
        4 => PickerCursor::SizeNwse,
        5 => PickerCursor::SizeNesw,
        _ => PickerCursor::Cross,
    }
}

fn apply_picker_cursor(kind: PickerCursor) {
    unsafe {
        let id = match kind {
            PickerCursor::Cross => IDC_CROSS,
            PickerCursor::SizeAll => IDC_SIZEALL,
            PickerCursor::SizeNs => IDC_SIZENS,
            PickerCursor::SizeWe => IDC_SIZEWE,
            PickerCursor::SizeNwse => IDC_SIZENWSE,
            PickerCursor::SizeNesw => IDC_SIZENESW,
        };
        if let Ok(cur) = LoadCursorW(None, id) {
            let _ = SetCursor(Some(cur));
        }
    }
}

fn set_picker_cursor(kind: PickerCursor) {
    PICKER_CURSOR.store(picker_cursor_code(kind), Ordering::Relaxed);
    apply_picker_cursor(kind);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_NCHITTEST => {
                if PICKER_HIT_TEST.load(Ordering::Relaxed) {
                    LRESULT(HTCLIENT as isize)
                } else {
                    LRESULT(HTTRANSPARENT as isize)
                }
            }
            WM_SETCURSOR => {
                if PICKER_HIT_TEST.load(Ordering::Relaxed) {
                    apply_picker_cursor(picker_cursor_from_code(PICKER_CURSOR.load(Ordering::Relaxed)));
                    LRESULT(1)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }
            // Picker is WS_EX_NOACTIVATE, but a TOPMOST layered window can
            // still be activated on click. Refuse activation so the target
            // stays foreground while the user draws boxes.
            WM_MOUSEACTIVATE => {
                if PICKER_HIT_TEST.load(Ordering::Relaxed) {
                    LRESULT(MA_NOACTIVATE as isize)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
