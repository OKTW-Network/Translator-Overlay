//! Overlay visibility: follow target, hide when unfocused, present.

use tracing::{debug, warn};
use windows::Win32::UI::{
    Input::KeyboardAndMouse::ReleaseCapture,
    WindowsAndMessaging::{
        GetForegroundWindow, HWND_NOTOPMOST, HWND_TOPMOST, IsIconic, IsWindow, IsWindowVisible, SW_HIDE, SWP_HIDEWINDOW, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow,
    },
};

use crate::{
    host::{
        OverlayHost,
        win32::{client_screen_rect, is_picker_allowed_foreground, is_target_in_foreground},
    },
    picker::PickerEnd,
};

impl OverlayHost {
    pub(crate) fn tick(&mut self) {
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

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            debug!("target window gone — detaching overlay");
            self.target = None;
            self.hide();
            return;
        }
        if unsafe { IsIconic(target) }.as_bool() || !unsafe { IsWindowVisible(target) }.as_bool() {
            self.hide();
            return;
        }

        // Only show while the capture target (or one of its children) is
        // the foreground window.
        let fg = unsafe { GetForegroundWindow() };
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
            self.last_present = None;
        }

        // TOPMOST without move/size — position comes from UpdateLayeredWindow.
        let _ =
            unsafe { SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW) };

        if let Err(e) = self.present_to_client(x, y, client_w, client_h) {
            warn!(error = %e, "UpdateLayeredWindow failed");
        }
    }

    pub(crate) fn tick_picker(&mut self) {
        let Some(target) = self.target else {
            self.finish_picker(PickerEnd::Cancel);
            return;
        };

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            debug!("target window gone — cancelling region picker");
            self.target = None;
            self.finish_picker(PickerEnd::Cancel);
            return;
        }
        if unsafe { IsIconic(target) }.as_bool() || !unsafe { IsWindowVisible(target) }.as_bool() {
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
        let dragging = self.picker.as_ref().is_some_and(crate::picker::RegionPicker::is_dragging);
        let fg = unsafe { GetForegroundWindow() };
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
        self.last_present = None;

        let _ =
            unsafe { SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW) };

        if let Err(e) = self.present_to_client(x, y, client_w, client_h) {
            warn!(error = %e, "UpdateLayeredWindow failed (picker)");
        }
    }

    pub(crate) fn hide(&mut self) {
        self.last_present = None;
        // Drop topmost so we never stay above unrelated apps after hide.
        let _ =
            unsafe { SetWindowPos(self.hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW) };
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    pub(crate) fn abort_picker_drag(&mut self) {
        if let Some(p) = self.picker.as_mut() {
            p.cancel_drag();
        }
        let _ = unsafe { ReleaseCapture() };
    }
}
