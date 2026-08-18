//! Overlay visibility: follow the target, mirror its Z-order, and present.

use std::sync::atomic::Ordering;

use tracing::{debug, warn};
use windows::Win32::UI::{
    Input::KeyboardAndMouse::ReleaseCapture,
    WindowsAndMessaging::{IsIconic, IsWindow, IsWindowVisible, SW_HIDE, ShowWindow},
};

use crate::{
    host::{
        OverlayHost,
        follow::FOLLOW_TARGET,
        win32::{client_screen_rect, place_overlay_above_target},
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
            FOLLOW_TARGET.store(0, Ordering::Release);
            self.hide();
            return;
        }
        if unsafe { IsIconic(target) }.as_bool() || !unsafe { IsWindowVisible(target) }.as_bool() {
            self.hide();
            return;
        }

        // Align to **client area** (matches cropped capture frames).
        let Some((x, y, client_w, client_h)) = client_screen_rect(target) else {
            self.hide();
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

        if let Err(e) = place_overlay_above_target(self.hwnd, target, x, y, client_w, client_h) {
            warn!(error = %e, "overlay Z-order update failed");
            return;
        }

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
            FOLLOW_TARGET.store(0, Ordering::Release);
            self.finish_picker(PickerEnd::Cancel);
            return;
        }
        if unsafe { IsIconic(target) }.as_bool() || !unsafe { IsWindowVisible(target) }.as_bool() {
            self.abort_picker_drag();
            self.hide();
            return;
        }

        let Some((x, y, client_w, client_h)) = client_screen_rect(target) else {
            self.abort_picker_drag();
            self.hide();
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

        if let Err(e) = place_overlay_above_target(self.hwnd, target, x, y, client_w, client_h) {
            warn!(error = %e, "picker Z-order update failed");
            return;
        }

        if let Err(e) = self.present_to_client(x, y, client_w, client_h) {
            warn!(error = %e, "UpdateLayeredWindow failed (picker)");
        }
    }

    pub(crate) fn hide(&mut self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    pub(crate) fn abort_picker_drag(&mut self) {
        if let Some(p) = self.picker.as_mut() {
            p.cancel_drag();
        }
        let _ = unsafe { ReleaseCapture() };
    }
}
