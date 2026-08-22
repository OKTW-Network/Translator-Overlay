//! Region-picker session on the overlay HWND (input + click-through).

use std::sync::atomic::Ordering;

use translator_core::NormRect;
use windows::Win32::{
    Foundation::LPARAM,
    UI::{
        Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
        WindowsAndMessaging::{
            GWL_EXSTYLE, GetWindowLongPtrW, MSG, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
            SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, WINDOW_EX_STYLE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
            WM_RBUTTONUP, WS_EX_TRANSPARENT,
        },
    },
};

use crate::{
    command::OverlayEvent,
    host::{
        OverlayHost,
        win32::{live_client_screen_rect, set_overlay_owner},
        wnd::{PICKER_HIT_TEST, set_picker_cursor},
    },
    picker::{PickerAction, PickerCursor, RegionPicker},
};

#[derive(Clone, Copy)]
pub(crate) enum PickerEnd {
    Confirm,
    Cancel,
}

pub(crate) fn is_picker_message(msg: u32) -> bool {
    matches!(msg, WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEMOVE | WM_RBUTTONUP)
}

fn mouse_pos(lparam: LPARAM) -> (i32, i32) {
    let v = lparam.0 as u32;
    let x = (v & 0xFFFF) as i16 as i32;
    let y = ((v >> 16) & 0xFFFF) as i16 as i32;
    (x, y)
}

impl OverlayHost {
    pub(crate) fn begin_picker(&mut self, regions: Vec<NormRect>) {
        let (cw, ch) = self
            .target
            .and_then(live_client_screen_rect)
            .map(|(_, _, w, h)| (w, h))
            .unwrap_or((800, 600));
        self.picker = Some(RegionPicker::new(regions, cw, ch));
        self.presented_rect = None;
        // Captions own the target; picker insert-above requires an unowned overlay.
        set_overlay_owner(self.hwnd, None);
        self.set_click_through(false);
        PICKER_HIT_TEST.store(true, Ordering::Relaxed);
        set_picker_cursor(PickerCursor::Cross);
        self.dirty = true;
        // Dashboard just received the click, so this process may set foreground.
        // Raise the target before apply so unowned-topmost-while-focused is true.
        if let Some(target) = self.target {
            let _ = unsafe { SetForegroundWindow(target) };
        }
    }

    pub(crate) fn finish_picker(&mut self, end: PickerEnd) {
        let Some(picker) = self.picker.take() else {
            return;
        };
        self.presented_rect = None;
        self.set_click_through(true);
        PICKER_HIT_TEST.store(false, Ordering::Relaxed);
        let _ = unsafe { ReleaseCapture() };
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
        let raw = unsafe { GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) } as u32;
        let style = if through {
            WINDOW_EX_STYLE(raw | WS_EX_TRANSPARENT.0)
        } else {
            WINDOW_EX_STYLE(raw & !WS_EX_TRANSPARENT.0)
        };
        unsafe { SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, style.0 as isize) };
        let _ = unsafe {
            SetWindowPos(self.hwnd, None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED)
        };
    }

    pub(crate) fn dispatch_picker_msg(&mut self, msg: &MSG) {
        let (px, py) = mouse_pos(msg.lParam);
        match msg.message {
            WM_LBUTTONDOWN => {
                if let Some(p) = self.picker.as_mut() {
                    p.on_left_down(px, py);
                    let _ = unsafe { SetCapture(self.hwnd) };
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
                let _ = unsafe { ReleaseCapture() };
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
}

#[cfg(test)]
mod tests {
    use windows::Win32::UI::WindowsAndMessaging::WM_KEYDOWN;

    use super::*;

    #[test]
    fn picker_mouse_messages_are_all_intercepted() {
        for message in [WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONUP] {
            assert!(is_picker_message(message));
        }
        assert!(!is_picker_message(WM_KEYDOWN));
    }
}
