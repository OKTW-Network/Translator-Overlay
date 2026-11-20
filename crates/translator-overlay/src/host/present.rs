//! Overlay visibility and presentation.
//!
//! Captions: owned overlay (unowned where capture cannot exclude owned
//! windows) + capture/DWM client rect.
//! Picker: unowned insert-above + live client rect; pure target drags use
//! `SetWindowPos` only (no per-drag `UpdateLayeredWindow`).

use std::sync::atomic::Ordering;

use tracing::{debug, warn};
use translator_capture::{client_screen_rect, excludes_owned_windows};
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
    pub(crate) fn apply_overlay(&mut self, allow_restack: bool) {
        if !self.ensure_layer_alive() {
            return;
        }

        // Mid-drag, new captions (streaming) still repaint; drag frames alone only move the window.
        if self.in_movesize && !(self.dirty && self.picker.is_none()) {
            self.follow_move();
            return;
        }

        if self.picker.is_some() {
            self.apply_picker(allow_restack);
            return;
        }

        self.apply_captions(allow_restack);
    }

    /// Returns whether the pump should `apply_overlay` after this notice.
    pub(crate) fn on_follow_event(&mut self, event: u32, hwnd: HWND) -> bool {
        match event {
            EVENT_SYSTEM_MOVESIZESTART => self.in_movesize = true,
            EVENT_SYSTEM_MOVESIZEEND => {
                self.in_movesize = false;
                self.presented_rect = None;
            }
            // Z-order churn during a drag: the drag path moves the overlay itself.
            EVENT_OBJECT_REORDER if self.in_movesize => return false,
            EVENT_OBJECT_DESTROY if self.target == Some(hwnd) => self.drop_dead_target(),
            _ => {}
        }
        true
    }

    /// The target window is gone: detach, and cancel the picker if it is open.
    fn drop_dead_target(&mut self) {
        debug!("target window gone; detaching overlay");
        self.target = None;
        FOLLOW_TARGET.store(0, Ordering::Release);
        self.in_movesize = false;
        self.z_order_force_topmost = false;
        if self.picker.is_some() {
            self.presented_rect = None;
            self.finish_picker(PickerEnd::Cancel);
        } else {
            self.release_target();
        }
    }

    /// Interactive title-bar drag / resize: live rect + one `SetWindowPos`.
    fn follow_move(&mut self) {
        let Some(target) = self.target else {
            return;
        };

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            self.drop_dead_target();
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

    fn apply_captions(&mut self, allow_restack: bool) {
        if !self.config.enabled {
            self.hide();
            return;
        }

        let Some(target) = self.target else {
            return;
        };

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            self.drop_dead_target();
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

        let Some(rect) = client_screen_rect(target.0 as isize) else {
            return;
        };
        // An owned overlay would land in the target's own capture and feed OCR its captions.
        let ownership = if excludes_owned_windows() {
            OverlayOwnership::OwnedByTarget
        } else {
            OverlayOwnership::Unowned
        };

        if self.captions_hidden || self.blocks.is_empty() || self.content_w == 0 || self.content_h == 0 {
            let (_, _, client_w, client_h) = rect;
            if client_w <= 0 || client_h <= 0 {
                return;
            }
            if client_w != self.surface_w || client_h != self.surface_h {
                self.surface_w = client_w;
                self.surface_h = client_h;
                self.dirty = true;
            }
            let content_changed = if self.dirty {
                if let Err(e) = self.surface.ensure(client_w, client_h) {
                    warn!(error = %e, "overlay blank present failed");
                    return;
                }
                if let Some(px) = self.surface.pixels() {
                    px.fill(0);
                }
                self.dirty = false;
                true
            } else {
                false
            };
            self.update_overlay_window(target, rect, content_changed, ownership, allow_restack);
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

        self.update_overlay_window(target, rect, content_changed, ownership, allow_restack);
    }

    fn apply_picker(&mut self, allow_restack: bool) {
        let Some(target) = self.target else {
            self.finish_picker(PickerEnd::Cancel);
            return;
        };

        if !unsafe { IsWindow(Some(target)) }.as_bool() {
            self.drop_dead_target();
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

        self.update_overlay_window(target, rect, content_changed, OverlayOwnership::Unowned, allow_restack);
    }

    fn update_overlay_window(
        &mut self,
        target: HWND,
        rect: ClientRect,
        content_changed: bool,
        ownership: OverlayOwnership,
        allow_restack: bool,
    ) {
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
                if !allow_restack {
                    return;
                }
                if let Err(e) = place_overlay_above_target(self.hwnd, target, ownership, &mut self.z_order_force_topmost) {
                    warn!(error = %e, ?ownership, "overlay Z-order update failed; showing with current Z-order");
                    let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
                }
                // Band change / first-show `SetWindowPos` can drop the ULW above.
                // Bits-only: no ppt_dst/psize — those restack and can drop HWND_TOPMOST.
                if present_after_restack {
                    if let Err(e) = self.refresh_layered_bits(rect.2, rect.3) {
                        warn!(error = %e, ?ownership, "UpdateLayeredWindow failed");
                    }
                    self.reassert_z_order_after_bits(target, ownership, want_topmost);
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
                if !allow_restack {
                    return;
                }
                // Skip a no-op restack: ShowWindow on an already-visible layered
                // window without ULW can drop DWM's bitmap.
                if (needs_restack || !overlay_visible)
                    && let Err(e) = place_overlay_above_target(self.hwnd, target, ownership, &mut self.z_order_force_topmost)
                {
                    warn!(error = %e, ?ownership, "overlay Z-order update failed; showing with current Z-order");
                    let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
                }
                if present_after_restack {
                    if let Err(e) = self.refresh_layered_bits(rect.2, rect.3) {
                        warn!(error = %e, ?ownership, "UpdateLayeredWindow failed");
                        return;
                    }
                    self.presented_rect = Some(rect);
                    self.reassert_z_order_after_bits(target, ownership, want_topmost);
                }
            }
        }
    }

    fn reassert_z_order_after_bits(&mut self, target: HWND, ownership: OverlayOwnership, want_topmost: bool) {
        if window_is_topmost(self.hwnd) == want_topmost && !overlay_needs_restack(self.hwnd, target, ownership, self.z_order_force_topmost)
        {
            return;
        }
        if let Err(e) = place_overlay_above_target(self.hwnd, target, ownership, &mut self.z_order_force_topmost) {
            warn!(error = %e, ?ownership, "overlay Z-order reassert failed; showing with current Z-order");
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
        }
    }

    pub(crate) fn hide(&mut self) {
        if self.hwnd.is_invalid() || !unsafe { IsWindow(Some(self.hwnd)) }.as_bool() {
            return;
        }
        self.presented_rect = None;
        self.replay_present = false;
        self.dirty = true;
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
    use crate::host::captions::SourceSpan;

    fn overlay_ex_style(hwnd: HWND) -> windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE {
        use windows::Win32::UI::WindowsAndMessaging::{GWL_EXSTYLE, GetWindowLongPtrW, WINDOW_EX_STYLE};

        WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32)
    }

    struct OverlayTestRig {
        host: crate::host::OverlayHost,
        target: HWND,
        event_rx: tokio::sync::mpsc::UnboundedReceiver<crate::command::OverlayEvent>,
        _lock: parking_lot::MutexGuard<'static, ()>,
    }

    impl Drop for OverlayTestRig {
        fn drop(&mut self) {
            use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
            self.host.teardown();
            if !self.target.is_invalid() {
                let _ = unsafe { DestroyWindow(self.target) };
            }
        }
    }

    fn spawn_overlay_and_target() -> Result<OverlayTestRig, String> {
        use tokio::sync::mpsc;
        use translator_core::OverlayConfig;
        use windows::{
            Win32::{
                System::LibraryLoader::GetModuleHandleW,
                UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, SW_SHOW, SetForegroundWindow, ShowWindow, WS_OVERLAPPEDWINDOW},
            },
            core::w,
        };

        use crate::host::{OverlayHost, win32::lock_hwnd_tests};

        let lock = lock_hwnd_tests();
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

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        match OverlayHost::create(
            OverlayConfig {
                reader_enabled: false,
                ..OverlayConfig::default()
            },
            event_tx,
        ) {
            Ok(host) => Ok(OverlayTestRig {
                host,
                target,
                event_rx,
                _lock: lock,
            }),
            Err(e) => {
                let _ = unsafe { DestroyWindow(target) };
                Err(format!("OverlayHost::create: {e}"))
            }
        }
    }

    #[test]
    fn first_picker_session_shows_a_visible_hit_testable_layer() -> Result<(), String> {
        use std::sync::atomic::Ordering;

        use windows::Win32::UI::WindowsAndMessaging::{WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT};

        use crate::{command::OverlayCommand, host::wnd::PICKER_HIT_TEST};

        let mut rig = spawn_overlay_and_target()?;

        rig.host.handle(OverlayCommand::Attach {
            target_hwnd: rig.target.0 as isize,
        });
        rig.host.apply_overlay(true);
        rig.host.handle(OverlayCommand::BeginRegionSelect { regions: Vec::new() });
        rig.host.apply_overlay(true);
        if rig.host.replay_present {
            rig.host.apply_overlay(true);
        }

        let visible = unsafe { IsWindowVisible(rig.host.hwnd) }.as_bool();
        let ex = overlay_ex_style(rig.host.hwnd);
        let click_through = ex.contains(WS_EX_TRANSPARENT);
        let toolwindow = ex.contains(WS_EX_TOOLWINDOW);
        let topmost = ex.contains(WS_EX_TOPMOST);
        let presented = rig.host.presented_rect.is_some();
        let full_veil = rig
            .host
            .surface
            .pixels()
            .is_some_and(|px| !px.is_empty() && px.as_chunks::<4>().0.iter().all(|c| c[3] > 0));
        let target_fg = target_is_foreground(rig.target);

        rig.host.teardown();
        let hit_test_after_teardown = PICKER_HIT_TEST.load(Ordering::Acquire);

        if !visible {
            return Err("overlay HWND is not visible after first picker apply".into());
        }
        if click_through {
            return Err("picker is still WS_EX_TRANSPARENT (click-through)".into());
        }
        if toolwindow {
            return Err("picker HWND is WS_EX_TOOLWINDOW (OBS skips it)".into());
        }
        if target_fg && !topmost {
            return Err("picker is not WS_EX_TOPMOST while the target is foreground".into());
        }
        if !presented {
            return Err("presented_rect is None after first picker apply".into());
        }
        if !full_veil {
            return Err("picker DIB must paint non-zero alpha across the full veil".into());
        }
        if hit_test_after_teardown {
            return Err("teardown left PICKER_HIT_TEST enabled".into());
        }
        Ok(())
    }

    #[test]
    fn empty_captions_stay_visible_and_clear_the_dib() -> Result<(), String> {
        use translator_core::{Rect, TranslatedBlock};
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextW, WS_EX_TOOLWINDOW};

        use crate::command::OverlayCommand;

        let mut rig = spawn_overlay_and_target()?;

        rig.host.handle(OverlayCommand::Attach {
            target_hwnd: rig.target.0 as isize,
        });
        rig.host.apply_overlay(true);

        let mut title_buf = [0u16; 256];
        let n = unsafe { GetWindowTextW(rig.host.hwnd, &mut title_buf) }.max(0) as usize;
        let title = String::from_utf16_lossy(&title_buf[..n]);
        let ex = overlay_ex_style(rig.host.hwnd);
        let visible_empty = unsafe { IsWindowVisible(rig.host.hwnd) }.as_bool();
        let blank_empty = rig.host.surface.pixels().is_some_and(|px| px.iter().all(|&b| b == 0));
        let presented_empty = rig.host.presented_rect.is_some();
        let toolwindow = ex.contains(WS_EX_TOOLWINDOW);

        rig.host.handle(OverlayCommand::SetBlocks {
            blocks: vec![TranslatedBlock {
                id: 1,
                source: "src".into(),
                translation: "hello".into(),
                confidence: 1.0,
                bbox: Rect::new(20.0, 20.0, 200.0, 40.0),
                source_lines: 1,
                source_height: 40.0,
                source_width: 200.0,
            }],
            locale: "zh-TW".into(),
            content_width: 400,
            content_height: 300,
        });
        rig.host.apply_overlay(true);
        let ink = rig.host.surface.pixels().is_some_and(|px| px.iter().any(|&b| b != 0));

        rig.host.handle(OverlayCommand::Clear);
        rig.host.apply_overlay(true);
        let visible_clear = unsafe { IsWindowVisible(rig.host.hwnd) }.as_bool();
        let blank_clear = rig.host.surface.pixels().is_some_and(|px| px.iter().all(|&b| b == 0));

        if title != "Translator Overlay Captions" {
            return Err(format!("overlay title is {title:?}, expected Translator Overlay Captions"));
        }
        if toolwindow {
            return Err("overlay HWND is WS_EX_TOOLWINDOW (OBS skips it)".into());
        }
        if !visible_empty {
            return Err("empty captions hid the overlay (OBS would freeze the last frame)".into());
        }
        if !blank_empty {
            return Err("empty captions DIB is not fully transparent".into());
        }
        if !presented_empty {
            return Err("empty captions did not present a client rect".into());
        }
        if !ink {
            return Err("caption present painted no pixels".into());
        }
        if !visible_clear {
            return Err("Clear hid the overlay (OBS would freeze the last caption)".into());
        }
        if !blank_clear {
            return Err("Clear left non-zero pixels in the overlay DIB".into());
        }
        Ok(())
    }

    #[test]
    fn captions_arriving_mid_drag_are_painted() -> Result<(), String> {
        use translator_core::{Rect, TranslatedBlock};

        use crate::command::OverlayCommand;

        let mut rig = spawn_overlay_and_target()?;
        rig.host.handle(OverlayCommand::Attach {
            target_hwnd: rig.target.0 as isize,
        });
        rig.host.apply_overlay(true);

        // The user is dragging the target when a streamed caption lands.
        rig.host.in_movesize = true;
        rig.host.handle(OverlayCommand::SetBlocks {
            blocks: vec![TranslatedBlock {
                id: 1,
                source: "src".into(),
                translation: "hello".into(),
                confidence: 1.0,
                bbox: Rect::new(20.0, 20.0, 200.0, 40.0),
                source_lines: 1,
                source_height: 40.0,
                source_width: 200.0,
            }],
            locale: "zh-TW".into(),
            content_width: 400,
            content_height: 300,
        });
        rig.host.apply_overlay(true);
        let ink = rig.host.surface.pixels().is_some_and(|px| px.iter().any(|&b| b != 0));
        let still_dragging = rig.host.in_movesize;

        if !ink {
            return Err("a caption set mid-drag was not painted".into());
        }
        if !still_dragging {
            return Err("painting mid-drag ended the drag fast path".into());
        }
        Ok(())
    }

    #[test]
    fn wrapped_translation_follows_the_source_line_pitch() -> Result<(), String> {
        use translator_core::{Rect, TranslatedBlock};

        use crate::command::OverlayCommand;

        let mut rig = spawn_overlay_and_target()?;
        rig.host.handle(OverlayCommand::Attach {
            target_hwnd: rig.target.0 as isize,
        });
        // Two source lines spaced far wider than the font's own line pitch.
        rig.host.handle(OverlayCommand::SetBlocks {
            blocks: vec![TranslatedBlock {
                id: 1,
                source: "src".into(),
                translation: "Please wait right here".into(),
                confidence: 1.0,
                bbox: Rect::new(20.0, 20.0, 200.0, 40.0),
                source_lines: 2,
                source_height: 140.0,
                source_width: 200.0,
            }],
            locale: "zh-TW".into(),
            content_width: 400,
            content_height: 300,
        });
        rig.host.apply_overlay(true);
        let (w, _) = rig.host.surface.size();
        // Rows holding ink: a second colour besides the background fill.
        let inked: Vec<usize> = rig.host.surface.pixels().map_or(Vec::new(), |px| {
            px.chunks(w as usize * 4)
                .enumerate()
                .filter(|(_, row)| {
                    let mut colours = row.as_chunks::<4>().0.iter().filter(|p| p[3] != 0);
                    colours.next().is_some_and(|first| colours.any(|p| p != first))
                })
                .map(|(y, _)| y)
                .collect()
        });

        let (Some(top), Some(bottom)) = (inked.first(), inked.last()) else {
            return Err("caption was not painted".into());
        };
        let gap = inked.windows(2).map(|p| p[1] - p[0]).max().unwrap_or(0);
        if gap * 3 < bottom - top {
            return Err(format!("text lines must sit on the source lines, got ink rows {top}..{bottom} with a {gap}px gap"));
        }
        Ok(())
    }

    #[test]
    fn caption_ink_stays_on_its_background() -> Result<(), String> {
        use translator_core::{Rect, TranslatedBlock};

        use crate::{command::OverlayCommand, gfx::draw};

        let mut rig = spawn_overlay_and_target()?;
        rig.host.handle(OverlayCommand::Attach {
            target_hwnd: rig.target.0 as isize,
        });
        // Two lines merged with a much smaller third: the median line box sets a font taller
        // than the average line pitch, so the text cannot sit on the source lines.
        let text = "我完全了解，並會照著要求繼續處理這件事情，請你在這裡稍微等我一下就好。";
        let mut failures = Vec::new();
        for w in (200..=380).step_by(20) {
            rig.host.handle(OverlayCommand::SetBlocks {
                blocks: vec![TranslatedBlock {
                    id: 1,
                    source: "src".into(),
                    translation: text.into(),
                    confidence: 1.0,
                    bbox: Rect::new(10.0, 10.0, w as f32, 30.0),
                    source_lines: 3,
                    source_height: 72.0,
                    source_width: w as f32,
                }],
                locale: "zh-TW".into(),
                content_width: 400,
                content_height: 300,
            });
            rig.host.apply_overlay(true);
            let (sw, sh) = rig.host.surface.size();
            if sw != 400 || sh != 300 {
                failures.push(format!("w={w} unexpected surface size ({sw}, {sh}), expected (400, 300)"));
                continue;
            }
            let surface = draw::SurfaceSize::new(sw, sh);
            let span = SourceSpan { w, h: 72, lines: 3 };
            let (rect, px, _) = rig
                .host
                .layout_label(draw::SurfaceRect::new(10, 10, w, 30), text, span, &[], surface)
                .map_err(|e| e.to_string())?;
            let buf = rig.host.surface.pixels().ok_or("paint bitmap missing")?;
            let mut inside = 0usize;
            let mut stray = 0usize;
            for y in 0..sh {
                for x in 0..sw {
                    if buf[y as usize * surface.stride + x as usize * 4 + 3] != 0 {
                        if x >= rect.x && x < rect.x + rect.w && y >= rect.y && y < rect.y + rect.h {
                            inside += 1;
                        } else {
                            stray += 1;
                        }
                    }
                }
            }
            if inside == 0 {
                failures.push(format!("w={w} px={px} rect={rect:?} had no painted pixels inside rect"));
            }
            if stray > 0 {
                failures.push(format!("w={w} px={px} rect={rect:?} stray={stray}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!("ink outside the caption background:\n{}", failures.join("\n")))
        }
    }

    #[test]
    fn long_translation_shrinks_then_widens_then_wraps() -> Result<(), String> {
        use translator_core::{Rect, TranslatedBlock};

        use crate::{
            command::OverlayCommand,
            gfx::{
                draw::{self, SurfaceRect},
                text::LabelStyle,
            },
        };

        let mut rig = spawn_overlay_and_target()?;

        rig.host.handle(OverlayCommand::Attach {
            target_hwnd: rig.target.0 as isize,
        });
        rig.host.handle(OverlayCommand::SetBlocks {
            blocks: vec![TranslatedBlock {
                id: 1,
                source: "了解".into(),
                translation: "I understand completely and will proceed with the request as written".into(),
                confidence: 1.0,
                bbox: Rect::new(20.0, 20.0, 40.0, 28.0),
                source_lines: 1,
                source_height: 28.0,
                source_width: 40.0,
            }],
            locale: "zh-TW".into(),
            content_width: 400,
            content_height: 300,
        });
        rig.host.apply_overlay(true);

        let surface = draw::SurfaceSize::new(rig.host.surface_w.max(1), rig.host.surface_h.max(1));
        let long = "I understand completely and will proceed with the request as written";
        let (rect_h, font_h, _) = rig
            .host
            .layout_label(
                SurfaceRect {
                    x: 20,
                    y: 20,
                    w: 40,
                    h: 28,
                },
                long,
                SourceSpan { w: 40, h: 28, lines: 1 },
                &[],
                surface,
            )
            .map_err(|e| e.to_string())?;
        // Room to the right ends at x=70 and below at y=90.
        let obstacles = [
            SurfaceRect {
                x: 70,
                y: 20,
                w: 100,
                h: 28,
            },
            SurfaceRect {
                x: 0,
                y: 90,
                w: 300,
                h: 28,
            },
        ];
        let (rect_boxed, font_boxed, _) = rig
            .host
            .layout_label(
                SurfaceRect {
                    x: 20,
                    y: 20,
                    w: 40,
                    h: 28,
                },
                "I understand completely",
                SourceSpan { w: 40, h: 28, lines: 1 },
                &obstacles,
                surface,
            )
            .map_err(|e| e.to_string())?;
        // Two tightly spaced source lines: 52px of ink fits two wrapped lines without shrinking or widening.
        let two_line_base = SurfaceRect {
            x: 20,
            y: 150,
            w: 160,
            h: 28,
        };
        let (rect_two, font_two, _) = rig
            .host
            .layout_label(two_line_base, "Please wait right here", SourceSpan { w: 160, h: 52, lines: 2 }, &[], surface)
            .map_err(|e| e.to_string())?;
        // Same ink span from one tilted line: shrink or widen instead of wrapping.
        let (rect_tilt, font_tilt, _) = rig
            .host
            .layout_label(two_line_base, "Please wait right here", SourceSpan { w: 160, h: 52, lines: 1 }, &[], surface)
            .map_err(|e| e.to_string())?;
        let (_, font_short, _) = rig
            .host
            .layout_label(
                SurfaceRect {
                    x: 20,
                    y: 20,
                    w: 40,
                    h: 28,
                },
                "OK",
                SourceSpan { w: 40, h: 28, lines: 1 },
                &[],
                surface,
            )
            .map_err(|e| e.to_string())?;
        let (rect_v, _, vertical_en) = rig
            .host
            .layout_label(
                SurfaceRect {
                    x: 20,
                    y: 20,
                    w: 20,
                    h: 200,
                },
                long,
                SourceSpan { w: 20, h: 200, lines: 1 },
                &[],
                surface,
            )
            .map_err(|e| e.to_string())?;
        let column = SurfaceRect {
            x: 200,
            y: 20,
            w: 20,
            h: 120,
        };
        let (rect_cjk, _, vertical_cjk) = rig
            .host
            .layout_label(column, "我完全了解，並會照著要求繼續處理這件事情。", SourceSpan { w: 20, h: 120, lines: 1 }, &[], surface)
            .map_err(|e| e.to_string())?;
        let (_, _, vertical_short) = rig
            .host
            .layout_label(column, "嗯？", SourceSpan { w: 20, h: 120, lines: 1 }, &[], surface)
            .map_err(|e| e.to_string())?;

        // Five 16px glyphs fit one column: all ink lands at the right edge of a wide label.
        let label = SurfaceRect {
            x: 100,
            y: 20,
            w: 120,
            h: 120,
        };
        rig.host.surface.ensure(surface.width, surface.height).map_err(|e| e.to_string())?;
        rig.host.surface.pixels().ok_or("paint bitmap missing")?.fill(0);
        rig.host
            .paint_label(label, "我完全了解", LabelStyle {
                font_px: 16,
                color: draw::Rgba::from_argb(0xFFFF_FFFF),
                locale: "zh-TW",
                vertical: true,
                line_pitch: None,
            })
            .map_err(|e| e.to_string())?;
        let buf = rig.host.surface.pixels().ok_or("paint bitmap missing")?;
        let ink_x: Vec<i32> = (label.y..label.y + label.h)
            .flat_map(|y| (label.x..label.x + label.w).map(move |x| (x, y)))
            .filter(|&(x, y)| buf[y as usize * surface.stride + x as usize * 4 + 3] > 0)
            .map(|(x, _)| x)
            .collect();

        // Column source: Han text stays in columns at the source height, growing left.
        if !vertical_cjk || !vertical_short || vertical_en {
            return Err(format!("expected columns only for Han text, got cjk={vertical_cjk} short={vertical_short} en={vertical_en}"));
        }
        if rect_cjk.h != 120 || rect_cjk.x + rect_cjk.w != 220 || rect_cjk.w <= 20 {
            return Err(format!("expected extra columns to the left at 120px tall, got {rect_cjk:?}"));
        }
        if ink_x.len() < 50 || ink_x.iter().any(|&x| x < label.x + label.w - 40) {
            let min_x = ink_x.iter().min();
            return Err(format!("expected one column of ink at the right edge, got {} px from x={min_x:?}", ink_x.len()));
        }
        if rect_v.w <= 20 || rect_v.h != 200 || rect_v.x >= 20 {
            return Err(format!(
                "expected English over a 20x200 column to widen and wrap inside h=200 centered onto the canvas, got {rect_v:?}"
            ));
        }

        if rect_two.w != 160 || rect_two.h != 52 || font_two != font_short {
            return Err(format!(
                "two-line source must fit two lines in w=160 h=52 at source font {font_short}px, got {rect_two:?} at {font_two}px"
            ));
        }
        if rect_tilt.w <= 160 && font_tilt >= font_two {
            return Err(format!("one tilted source line must shrink or widen before wrapping, got {rect_tilt:?} at {font_tilt}px"));
        }

        // Too long for the 1.75× width cap: the font shrinks before wrapping.
        if font_h >= font_short {
            return Err(format!("long translation must shrink below the source font {font_short}px, got {font_h}"));
        }
        if font_boxed >= font_short {
            return Err(format!("boxed-in translation must shrink below {font_short}px, got {font_boxed}"));
        }
        if let Some(hit) = obstacles.iter().find(|o| {
            rect_boxed.x < o.x + o.w && o.x < rect_boxed.x + rect_boxed.w && rect_boxed.y < o.y + o.h && o.y < rect_boxed.y + rect_boxed.h
        }) {
            return Err(format!("caption {rect_boxed:?} overlaps neighbour {hit:?}"));
        }
        // Never clipped: past the font floor the text wraps within the width
        // cap, so the box grows taller instead of ending in "…".
        // The box hugs its widest wrapped line, which can stop a few px short of the cap.
        if !(64..=70).contains(&rect_h.w) {
            return Err(format!("expected wrapped width near the 1.75× cap (70px), got {}", rect_h.w));
        }
        if rect_h.h <= 28 {
            return Err(format!("expected wrapped height taller than the source line, got {}", rect_h.h));
        }
        Ok(())
    }

    #[test]
    fn on_follow_event_drives_movesize_and_destroy_state_machine() -> Result<(), String> {
        use std::sync::atomic::Ordering;

        use crate::{
            command::{OverlayCommand, OverlayEvent},
            host::wnd::PICKER_HIT_TEST,
        };

        let mut rig = spawn_overlay_and_target()?;
        rig.host.handle(OverlayCommand::Attach {
            target_hwnd: rig.target.0 as isize,
        });
        rig.host.apply_overlay(true);
        assert!(rig.host.presented_rect.is_some());

        assert!(rig.host.on_follow_event(EVENT_SYSTEM_MOVESIZESTART, rig.target));
        assert!(rig.host.in_movesize);
        assert!(!rig.host.on_follow_event(EVENT_OBJECT_REORDER, rig.target), "Z-order churn mid-drag must not schedule apply_overlay");

        assert!(rig.host.on_follow_event(EVENT_SYSTEM_MOVESIZEEND, rig.target));
        assert!(!rig.host.in_movesize);
        assert!(rig.host.presented_rect.is_none());
        assert!(rig.host.on_follow_event(EVENT_OBJECT_REORDER, rig.target), "Z-order events resume scheduling outside drag");

        // Unrelated HWND destroy leaves target + picker intact.
        rig.host.handle(OverlayCommand::BeginRegionSelect { regions: Vec::new() });
        let other = HWND(0xDEADisize as *mut _);
        assert!(rig.host.on_follow_event(EVENT_OBJECT_DESTROY, other));
        assert_eq!(rig.host.target, Some(rig.target));
        assert!(rig.host.picker.is_some());

        // Target HWND destroy detaches target and cancels active picker.
        assert!(rig.host.on_follow_event(EVENT_OBJECT_DESTROY, rig.target));
        assert!(rig.host.target.is_none());
        assert!(rig.host.picker.is_none());
        assert_eq!(FOLLOW_TARGET.load(Ordering::Acquire), 0);
        assert!(!PICKER_HIT_TEST.load(Ordering::Acquire));
        assert!(matches!(rig.event_rx.try_recv(), Ok(OverlayEvent::RegionSelectCancelled)));
        Ok(())
    }
}
