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
            overlay_wants_topmost, place_overlay_above_target, set_overlay_owner, target_is_foreground, window_is_topmost,
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
        self.z_order_force_topmost = false;
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
        let effective = if self.z_order_force_topmost {
            OverlayOwnership::Unowned
        } else {
            ownership
        };
        let overlay_topmost = window_is_topmost(self.hwnd);
        let overlay_visible = unsafe { IsWindowVisible(self.hwnd) }.as_bool();
        let target_fg = target_is_foreground(target);
        if self.replay_present {
            self.replay_present = false;
            self.presented_rect = None;
        }
        let want_topmost = overlay_wants_topmost(effective, window_is_topmost(target), target_fg);
        let needs_restack = overlay_needs_restack(self.hwnd, target, ownership, self.z_order_force_topmost);
        let present_after_restack = !overlay_visible || overlay_topmost != want_topmost || needs_restack;

        match presentation_action(self.presented_rect, rect, content_changed) {
            PresentationAction::FullPresent => {
                if let Err(e) = self.present_to_client(rect.0, rect.1, rect.2, rect.3) {
                    warn!(error = %e, ?ownership, "UpdateLayeredWindow failed");
                    return;
                }
                self.presented_rect = Some(rect);
                if let Err(e) = place_overlay_above_target(self.hwnd, target, ownership, &mut self.z_order_force_topmost) {
                    warn!(error = %e, ?ownership, "overlay Z-order update failed; showing with current Z-order");
                    let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
                }
                // Band change / first-show `SetWindowPos` can drop the ULW above.
                if present_after_restack && let Err(e) = self.present_to_client(rect.0, rect.1, rect.2, rect.3) {
                    warn!(error = %e, ?ownership, "UpdateLayeredWindow failed");
                }
                // First Show can leave DWM blank; replay a FullPresent on the next apply.
                if !overlay_visible && self.picker.is_some() {
                    self.replay_present = true;
                    self.dirty = true;
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
                // Skip a no-op restack: ShowWindow on an already-visible layered
                // window without ULW can drop DWM's bitmap.
                if (needs_restack || !overlay_visible)
                    && let Err(e) = place_overlay_above_target(self.hwnd, target, ownership, &mut self.z_order_force_topmost)
                {
                    warn!(error = %e, ?ownership, "overlay Z-order update failed; showing with current Z-order");
                    let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
                }
                if present_after_restack {
                    if let Err(e) = self.present_to_client(rect.0, rect.1, rect.2, rect.3) {
                        warn!(error = %e, ?ownership, "UpdateLayeredWindow failed");
                        return;
                    }
                    self.presented_rect = Some(rect);
                }
            }
        }
    }

    pub(crate) fn hide(&mut self) {
        if self.hwnd.is_invalid() || !unsafe { IsWindow(Some(self.hwnd)) }.as_bool() {
            return;
        }
        self.presented_rect = None;
        self.replay_present = false;
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

    #[test]
    fn first_picker_session_shows_a_visible_hit_testable_layer() {
        if let Err(e) = first_picker_hwnd_smoke() {
            panic!("{e}");
        }
    }

    fn first_picker_hwnd_smoke() -> Result<(), String> {
        use tokio::sync::mpsc;
        use translator_core::OverlayConfig;
        use windows::{
            Win32::{
                System::LibraryLoader::GetModuleHandleW,
                UI::WindowsAndMessaging::{
                    CreateWindowExW, DestroyWindow, GWL_EXSTYLE, GetWindowLongPtrW, SW_SHOW, SetForegroundWindow, ShowWindow,
                    WINDOW_EX_STYLE, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_OVERLAPPEDWINDOW,
                },
            },
            core::w,
        };

        use crate::{command::OverlayCommand, host::OverlayHost};

        let hinstance = unsafe { GetModuleHandleW(None) }.map_err(|e| format!("GetModuleHandleW: {e}"))?;
        let target = unsafe {
            CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                w!("picker-target"),
                WS_OVERLAPPEDWINDOW,
                80,
                80,
                480,
                360,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .map_err(|e| format!("CreateWindowExW(target): {e}"))?;
        let _ = unsafe { ShowWindow(target, SW_SHOW) };
        let _ = unsafe { SetForegroundWindow(target) };

        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let mut host = OverlayHost::create(
            OverlayConfig {
                reader_enabled: false,
                ..OverlayConfig::default()
            },
            event_tx,
        )
        .map_err(|e| format!("OverlayHost::create: {e}"))?;

        // Two-wake cold start: attach (empty captions → hide) then first picker apply.
        host.handle(OverlayCommand::Attach {
            target_hwnd: target.0 as isize,
        });
        host.apply_overlay();
        host.handle(OverlayCommand::BeginRegionSelect { regions: Vec::new() });
        host.apply_overlay();
        if !host.replay_present {
            return Err("first picker show did not schedule a replay present".into());
        }
        host.apply_overlay();

        let visible = unsafe { IsWindowVisible(host.hwnd) }.as_bool();
        let ex = WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(host.hwnd, GWL_EXSTYLE) } as u32);
        let click_through = ex.contains(WS_EX_TRANSPARENT);
        let topmost = ex.contains(WS_EX_TOPMOST);
        let presented = host.presented_rect.is_some();
        let veil = host.surface.pixels().is_some_and(|px| px.iter().any(|&b| b != 0));
        let target_fg = target_is_foreground(target);

        host.teardown();
        let _ = unsafe { DestroyWindow(target) };

        if !visible {
            return Err("overlay HWND is not visible after first picker apply".into());
        }
        if click_through {
            return Err("picker is still WS_EX_TRANSPARENT (click-through)".into());
        }
        if target_fg && !topmost {
            return Err("picker is not WS_EX_TOPMOST while the target is foreground".into());
        }
        if !presented {
            return Err("presented_rect is None after first picker apply".into());
        }
        if !veil {
            return Err("picker DIB has no non-zero alpha (blank veil)".into());
        }
        Ok(())
    }
}
