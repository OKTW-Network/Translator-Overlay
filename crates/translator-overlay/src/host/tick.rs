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
        win32::{
            ClientRect, PlacementGeometry, client_screen_rect, place_overlay_above_target, position_overlay_preserving_z_order,
            show_overlay,
        },
    },
    picker::PickerEnd,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresentationAction {
    FullPresent,
    MoveOnly,
    RestackOnly,
}

fn presentation_action(previous: Option<ClientRect>, current: ClientRect, content_changed: bool) -> PresentationAction {
    let Some(previous) = previous else {
        return PresentationAction::FullPresent;
    };
    if content_changed || previous.2 != current.2 || previous.3 != current.3 {
        PresentationAction::FullPresent
    } else if previous.0 != current.0 || previous.1 != current.1 {
        PresentationAction::MoveOnly
    } else {
        PresentationAction::RestackOnly
    }
}

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
            self.presented_rect = None;
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

        let content_changed = if self.dirty {
            if let Err(e) = self.repaint() {
                warn!(error = %e, "overlay repaint failed");
                return;
            }
            self.dirty = false;
            true
        } else {
            false
        };

        self.update_overlay_window(target, (x, y, client_w, client_h), content_changed, "captions");
    }

    pub(crate) fn tick_picker(&mut self) {
        let Some(target) = self.target else {
            self.finish_picker(PickerEnd::Cancel);
            self.hide();
            return;
        };

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            debug!("target window gone — cancelling region picker");
            self.target = None;
            FOLLOW_TARGET.store(0, Ordering::Release);
            self.presented_rect = None;
            self.finish_picker(PickerEnd::Cancel);
            self.hide();
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

        if self.surface_w != client_w || self.surface_h != client_h {
            self.surface_w = client_w;
            self.surface_h = client_h;
            self.dirty = true;
        }

        let content_changed = if self.dirty {
            if let Err(e) = self.repaint_picker() {
                warn!(error = %e, "picker repaint failed");
                return;
            }
            self.dirty = false;
            true
        } else {
            false
        };

        self.update_overlay_window(target, (x, y, client_w, client_h), content_changed, "picker");
    }

    fn update_overlay_window(
        &mut self,
        target: windows::Win32::Foundation::HWND,
        rect: ClientRect,
        content_changed: bool,
        operation: &str,
    ) {
        match presentation_action(self.presented_rect, rect, content_changed) {
            PresentationAction::FullPresent => {
                if let Err(e) = self.present_to_client(rect.0, rect.1, rect.2, rect.3) {
                    warn!(error = %e, operation, "UpdateLayeredWindow failed");
                    return;
                }
                self.presented_rect = Some(rect);
                if let Err(e) = place_overlay_above_target(self.hwnd, target, PlacementGeometry::Preserve) {
                    // The bitmap is already valid. A transient Z-order race
                    // must not keep captions hidden.
                    warn!(error = %e, operation, "overlay Z-order update failed; showing with current Z-order");
                    show_overlay(self.hwnd);
                }
            }
            PresentationAction::MoveOnly => {
                if let Err(e) = place_overlay_above_target(self.hwnd, target, PlacementGeometry::Set(rect)) {
                    warn!(error = %e, operation, "overlay Z-order update failed; moving with current Z-order");
                    if let Err(fallback_error) = position_overlay_preserving_z_order(self.hwnd, rect) {
                        warn!(error = %fallback_error, operation, "overlay fallback move failed");
                        return;
                    }
                }
                self.presented_rect = Some(rect);
            }
            PresentationAction::RestackOnly => {
                if let Err(e) = place_overlay_above_target(self.hwnd, target, PlacementGeometry::Preserve) {
                    warn!(error = %e, operation, "overlay Z-order update failed; showing with current Z-order");
                    show_overlay(self.hwnd);
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: ClientRect = (10, 20, 800, 600);

    #[test]
    fn first_frame_and_content_changes_require_a_full_present() {
        assert_eq!(presentation_action(None, RECT, false), PresentationAction::FullPresent);
        assert_eq!(presentation_action(Some(RECT), RECT, true), PresentationAction::FullPresent);
    }

    #[test]
    fn destination_resize_requires_a_full_present() {
        assert_eq!(presentation_action(Some(RECT), (10, 20, 900, 600), false), PresentationAction::FullPresent);
    }

    #[test]
    fn pure_position_change_moves_without_representing_the_bitmap() {
        assert_eq!(presentation_action(Some(RECT), (30, 40, 800, 600), false), PresentationAction::MoveOnly);
    }

    #[test]
    fn unchanged_geometry_only_needs_restacking() {
        assert_eq!(presentation_action(Some(RECT), RECT, false), PresentationAction::RestackOnly);
    }
}
