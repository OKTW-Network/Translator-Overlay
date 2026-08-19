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
            EVENT_OBJECT_DESTROY, EVENT_OBJECT_REORDER, EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZESTART, IsIconic, IsWindow,
            IsWindowVisible, SW_HIDE, SW_SHOWNOACTIVATE, ShowWindow,
        },
    },
};

use crate::{
    host::{
        OverlayHost,
        follow::FOLLOW_TARGET,
        win32::{
            ClientRect, OverlayOwnership, live_client_screen_rect, move_overlay_position, overlay_needs_restack, overlay_owner,
            place_overlay_above_target, set_overlay_owner,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FollowNotice {
    BeginMoveSize,
    EndMoveSize,
    Drop,
    Apply,
}

fn classify_follow_notice(event: u32, in_movesize: bool) -> FollowNotice {
    match event {
        EVENT_SYSTEM_MOVESIZESTART => FollowNotice::BeginMoveSize,
        EVENT_SYSTEM_MOVESIZEEND => FollowNotice::EndMoveSize,
        EVENT_OBJECT_REORDER if in_movesize => FollowNotice::Drop,
        _ => FollowNotice::Apply,
    }
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
    pub(crate) fn apply_overlay(&mut self) {
        if !self.ensure_layer_alive() {
            return;
        }

        if self.in_movesize {
            self.follow_move();
            return;
        }

        if self.picker.is_some() {
            self.apply_picker();
            return;
        }

        self.apply_captions();
    }

    /// Returns whether the pump should `apply_overlay` after this notice.
    pub(crate) fn on_follow_event(&mut self, event: u32, hwnd: HWND) -> bool {
        match classify_follow_notice(event, self.in_movesize) {
            FollowNotice::BeginMoveSize => {
                self.in_movesize = true;
                true
            }
            FollowNotice::EndMoveSize => {
                self.in_movesize = false;
                self.presented_rect = None;
                true
            }
            FollowNotice::Drop => false,
            FollowNotice::Apply => {
                if event == EVENT_OBJECT_DESTROY {
                    self.on_follow_destroy(hwnd);
                }
                true
            }
        }
    }

    fn on_follow_destroy(&mut self, hwnd: HWND) {
        if self.target != Some(hwnd) {
            return;
        }
        debug!("target window gone — detaching overlay");
        self.forget_target();
        if self.picker.is_some() {
            self.presented_rect = None;
            self.finish_picker(PickerEnd::Cancel);
        } else {
            self.release_target();
        }
    }

    pub(crate) fn clear_follow_move_state(&mut self) {
        self.in_movesize = false;
    }

    fn forget_target(&mut self) {
        self.target = None;
        FOLLOW_TARGET.store(0, std::sync::atomic::Ordering::Release);
        self.in_movesize = false;
    }

    /// Interactive title-bar drag / resize: live rect + one `SetWindowPos`.
    fn follow_move(&mut self) {
        let Some(target) = self.target else {
            return;
        };

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            debug!("target window gone — detaching overlay");
            self.forget_target();
            if self.picker.is_some() {
                self.presented_rect = None;
                self.finish_picker(PickerEnd::Cancel);
            } else {
                self.release_target();
            }
            return;
        }
        if unsafe { IsIconic(target) }.as_bool() {
            return;
        }

        let Some(live) = live_client_screen_rect(target) else {
            return;
        };
        let Some(presented) = self.presented_rect else {
            return;
        };
        let next = (live.0, live.1, presented.2, presented.3);
        if next.0 == presented.0 && next.1 == presented.1 {
            return;
        }
        if let Err(e) = move_overlay_position(self.hwnd, next.0, next.1) {
            warn!(error = %e, "overlay follow move failed");
            return;
        }
        self.presented_rect = Some(next);
    }

    fn apply_captions(&mut self) {
        if !self.config.enabled {
            self.hide();
            return;
        }

        let Some(target) = self.target else {
            return;
        };

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            debug!("target window gone — detaching overlay");
            self.forget_target();
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

    fn apply_picker(&mut self) {
        let Some(target) = self.target else {
            self.finish_picker(PickerEnd::Cancel);
            return;
        };

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            debug!("target window gone — cancelling region picker");
            self.forget_target();
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
                if let Err(e) = place_overlay_above_target(self.hwnd, target, ownership) {
                    warn!(error = %e, ?ownership, "overlay Z-order update failed; showing with current Z-order");
                    let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
                }
            }
            PresentationAction::MoveOnly => {
                // Position only — never re-ULW or restack, or the drag loop hitches.
                if let Err(e) = move_overlay_position(self.hwnd, rect.0, rect.1) {
                    warn!(error = %e, ?ownership, "overlay follow move failed");
                    return;
                }
                self.presented_rect = Some(rect);
            }
            PresentationAction::RestackOnly => {
                // SetWindowPos on a layered window without ULW can blank it.
                if overlay_needs_restack(self.hwnd, target, ownership) {
                    if let Err(e) = self.present_to_client(rect.0, rect.1, rect.2, rect.3) {
                        warn!(error = %e, ?ownership, "UpdateLayeredWindow failed");
                        return;
                    }
                    self.presented_rect = Some(rect);
                }
                if let Err(e) = place_overlay_above_target(self.hwnd, target, ownership) {
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

    #[test]
    fn movesize_notices_bookend_interactive_follow() {
        assert_eq!(classify_follow_notice(EVENT_SYSTEM_MOVESIZESTART, false), FollowNotice::BeginMoveSize);
        assert_eq!(classify_follow_notice(EVENT_SYSTEM_MOVESIZEEND, true), FollowNotice::EndMoveSize);
    }

    #[test]
    fn reorder_is_dropped_only_during_movesize() {
        assert_eq!(classify_follow_notice(EVENT_OBJECT_REORDER, true), FollowNotice::Drop);
        assert_eq!(classify_follow_notice(EVENT_OBJECT_REORDER, false), FollowNotice::Apply);
        assert_eq!(classify_follow_notice(EVENT_OBJECT_DESTROY, true), FollowNotice::Apply);
    }
}
