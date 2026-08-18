//! Overlay visibility and presentation.
//!
//! Captions: owned overlay + capture/DWM client rect.
//! Picker: unowned insert-above + live client rect; pure target drags use
//! `SetWindowPos` only (no per-drag `UpdateLayeredWindow`).

use tracing::{debug, warn};
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::ReleaseCapture,
        WindowsAndMessaging::{
            IsIconic, IsWindow, IsWindowVisible, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowPos,
            ShowWindow,
        },
    },
};

use crate::{
    host::{
        OverlayHost,
        follow::FOLLOW_TARGET,
        win32::{
            ClientRect, OverlayOwnership, PlacementGeometry, live_client_screen_rect, overlay_owner, place_overlay_above_target,
            set_overlay_owner,
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
        if !self.ensure_layer_alive() {
            return;
        }

        if self.picker.is_some() {
            self.tick_picker();
            return;
        }

        self.tick_captions();
    }

    fn tick_captions(&mut self) {
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
            FOLLOW_TARGET.store(0, std::sync::atomic::Ordering::Release);
            self.release_target();
            return;
        }
        if unsafe { IsIconic(target) }.as_bool() {
            // Owned overlay is hidden/shown with the owner; do not SW_HIDE or restore is suppressed.
            if overlay_owner(self.hwnd) != target {
                self.hide();
            }
            return;
        }
        if !unsafe { IsWindowVisible(target) }.as_bool() {
            self.hide();
            return;
        }

        let Some(rect) = translator_capture::client_screen_rect(target.0 as isize) else {
            return;
        };

        if self.blocks.is_empty() || self.content_w == 0 || self.content_h == 0 {
            self.hide();
            return;
        }

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

        self.update_overlay_window(target, rect, content_changed, OverlayOwnership::OwnedByTarget);
    }

    pub(crate) fn tick_picker(&mut self) {
        let Some(target) = self.target else {
            self.finish_picker(PickerEnd::Cancel);
            return;
        };

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            debug!("target window gone — cancelling region picker");
            self.target = None;
            FOLLOW_TARGET.store(0, std::sync::atomic::Ordering::Release);
            self.presented_rect = None;
            self.finish_picker(PickerEnd::Cancel);
            return;
        }
        if unsafe { IsIconic(target) }.as_bool() {
            self.abort_picker_drag();
            self.hide();
            return;
        }
        // Do not SW_HIDE on transient !visible / geometry gaps while dragging.
        if !unsafe { IsWindowVisible(target) }.as_bool() {
            self.abort_picker_drag();
            return;
        }

        let Some(rect) = live_client_screen_rect(target) else {
            return;
        };
        let (_, _, client_w, client_h) = rect;

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

        self.update_overlay_window(target, rect, content_changed, OverlayOwnership::Unowned);
    }

    fn update_overlay_window(&mut self, target: HWND, rect: ClientRect, content_changed: bool, ownership: OverlayOwnership) {
        match presentation_action(self.presented_rect, rect, content_changed) {
            PresentationAction::FullPresent => {
                // ULW first, then restack with Preserve so a Z-order race cannot
                // leave a blank/hidden layered window after present.
                if let Err(e) = self.present_to_client(rect.0, rect.1, rect.2, rect.3) {
                    warn!(error = %e, ?ownership, "UpdateLayeredWindow failed");
                    return;
                }
                self.presented_rect = Some(rect);
                if let Err(e) = place_overlay_above_target(self.hwnd, target, PlacementGeometry::Preserve, ownership) {
                    warn!(error = %e, ?ownership, "overlay Z-order update failed; showing with current Z-order");
                    let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
                }
            }
            PresentationAction::MoveOnly => {
                // Pure drag: SetWindowPos only — never re-ULW or the layer can vanish.
                if let Err(e) = place_overlay_above_target(self.hwnd, target, PlacementGeometry::Set(rect), ownership) {
                    warn!(error = %e, ?ownership, "overlay Z-order update failed; moving with current Z-order");
                    if let Err(fallback_error) = unsafe {
                        SetWindowPos(self.hwnd, None, rect.0, rect.1, rect.2, rect.3, SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW)
                    } {
                        warn!(error = %fallback_error, ?ownership, "overlay fallback move failed");
                        return;
                    }
                }
                self.presented_rect = Some(rect);
            }
            PresentationAction::RestackOnly => {
                if let Err(e) = place_overlay_above_target(self.hwnd, target, PlacementGeometry::Preserve, ownership) {
                    warn!(error = %e, ?ownership, "overlay Z-order update failed; showing with current Z-order");
                    let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
                }
            }
        }
    }

    pub(crate) fn hide(&mut self) {
        if self.hwnd.is_invalid() || !unsafe { IsWindow(Some(self.hwnd)) }.as_bool() {
            return;
        }
        self.presented_rect = None;
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    pub(crate) fn release_target(&mut self) {
        if !self.hwnd.is_invalid() && unsafe { IsWindow(Some(self.hwnd)) }.as_bool() {
            set_overlay_owner(self.hwnd, None);
        }
        self.hide();
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
