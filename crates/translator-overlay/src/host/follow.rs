//! Target follow via out-of-context `SetWinEventHook`.
//!
//! The hook cannot borrow `OverlayHost` (it re-enters inside `PeekMessage` /
//! `WaitMessage`). It only reads `FOLLOW_TARGET` / `FOLLOW_OVERLAY`, posts a
//! thread notice, and clears ownership before a dying target takes the overlay.

use std::sync::atomic::{AtomicIsize, Ordering};

use tracing::warn;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::{
        Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
        WindowsAndMessaging::{
            EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_REORDER, EVENT_OBJECT_SHOW,
            EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZEEND,
            EVENT_SYSTEM_MOVESIZESTART, GetWindowThreadProcessId, OBJID_WINDOW, PostThreadMessageW, WINEVENT_OUTOFCONTEXT, WM_APP,
        },
    },
};

use crate::host::win32::set_overlay_owner;

pub(crate) static FOLLOW_TARGET: AtomicIsize = AtomicIsize::new(0);
pub(crate) static FOLLOW_OVERLAY: AtomicIsize = AtomicIsize::new(0);
/// Dedicated wake for target geometry / Z-order — must not coalesce with `WM_APP`.
pub(crate) const FOLLOW_EVENT_MESSAGE: u32 = WM_APP + 1;

fn overlay_thread_id(fallback: HWND) -> u32 {
    let overlay = FOLLOW_OVERLAY.load(Ordering::Relaxed);
    let hwnd = if overlay != 0 { HWND(overlay as *mut _) } else { fallback };
    if hwnd.is_invalid() {
        return 0;
    }
    unsafe { GetWindowThreadProcessId(hwnd, None) }
}

pub(crate) fn wake_overlay_thread() {
    let thread_id = overlay_thread_id(HWND::default());
    if thread_id != 0 {
        let _ = unsafe { PostThreadMessageW(thread_id, WM_APP, WPARAM(0), LPARAM(0)) };
    }
}

fn post_follow(event: u32, hwnd: HWND) {
    let thread_id = overlay_thread_id(hwnd);
    if thread_id != 0 {
        let _ = unsafe { PostThreadMessageW(thread_id, FOLLOW_EVENT_MESSAGE, WPARAM(event as usize), LPARAM(hwnd.0 as isize)) };
    }
}

fn clear_owner_for_dying_target() {
    let overlay = FOLLOW_OVERLAY.load(Ordering::Relaxed);
    if overlay != 0 {
        set_overlay_owner(HWND(overlay as *mut _), None);
    }
    FOLLOW_TARGET.store(0, Ordering::Release);
}

fn should_forward(event: u32, is_target: bool, is_overlay: bool, is_window_object: bool) -> bool {
    match event {
        EVENT_SYSTEM_FOREGROUND => true,
        // Top-level Z-order changes report on the parent/desktop, not the target.
        EVENT_OBJECT_REORDER if is_window_object => true,
        EVENT_SYSTEM_MINIMIZESTART | EVENT_SYSTEM_MINIMIZEEND if is_target => true,
        EVENT_SYSTEM_MOVESIZESTART | EVENT_SYSTEM_MOVESIZEEND if is_target && is_window_object => true,
        EVENT_OBJECT_LOCATIONCHANGE | EVENT_OBJECT_SHOW | EVENT_OBJECT_HIDE if is_target && is_window_object => true,
        EVENT_OBJECT_DESTROY if (is_target || is_overlay) && is_window_object => true,
        _ => false,
    }
}

pub(crate) fn install_follow_hooks() -> [HWINEVENTHOOK; 5] {
    let foreground = unsafe {
        SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    let minimize = unsafe {
        SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    let movesize = unsafe {
        SetWinEventHook(EVENT_SYSTEM_MOVESIZESTART, EVENT_SYSTEM_MOVESIZEEND, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    let location = unsafe {
        SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    let object_changes =
        unsafe { SetWinEventHook(EVENT_OBJECT_DESTROY, EVENT_OBJECT_REORDER, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT) };
    if foreground.is_invalid() || minimize.is_invalid() || movesize.is_invalid() || location.is_invalid() || object_changes.is_invalid() {
        warn!("overlay follow WinEvent hooks failed to install");
    }
    [foreground, minimize, movesize, location, object_changes]
}

pub(crate) fn uninstall_follow_hooks(hooks: &mut [HWINEVENTHOOK; 5]) {
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
    let overlay = FOLLOW_OVERLAY.load(Ordering::Relaxed);
    let is_overlay = overlay != 0 && hwnd.0 as isize == overlay;
    let target = FOLLOW_TARGET.load(Ordering::Relaxed);
    let is_target = target != 0 && !hwnd.is_invalid() && hwnd.0 as isize == target;
    let is_window_object = id_object == OBJID_WINDOW.0;
    if is_target && is_window_object && event == EVENT_OBJECT_DESTROY {
        clear_owner_for_dying_target();
    }
    if should_forward(event, is_target, is_overlay, is_window_object) {
        post_follow(event, hwnd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_events_only_forward_the_target_window() {
        assert!(should_forward(EVENT_SYSTEM_MOVESIZESTART, true, false, true));
        assert!(should_forward(EVENT_SYSTEM_MOVESIZEEND, true, false, true));
        assert!(!should_forward(EVENT_SYSTEM_MOVESIZESTART, false, false, true));
        assert!(!should_forward(EVENT_SYSTEM_MOVESIZESTART, true, false, false));
    }

    #[test]
    fn location_and_foreground_events_are_forwarded() {
        assert!(should_forward(EVENT_OBJECT_LOCATIONCHANGE, true, false, true));
        assert!(!should_forward(EVENT_OBJECT_LOCATIONCHANGE, false, false, true));
        assert!(should_forward(EVENT_SYSTEM_FOREGROUND, false, false, false));
    }

    #[test]
    fn window_reorder_forwards_for_the_parent_container() {
        assert!(should_forward(EVENT_OBJECT_REORDER, false, false, true));
        assert!(should_forward(EVENT_OBJECT_REORDER, true, false, true));
        assert!(!should_forward(EVENT_OBJECT_REORDER, false, false, false));
    }

    #[test]
    fn destroy_forwards_target_and_overlay_windows() {
        assert!(should_forward(EVENT_OBJECT_DESTROY, true, false, true));
        assert!(should_forward(EVENT_OBJECT_DESTROY, false, true, true));
        assert!(!should_forward(EVENT_OBJECT_DESTROY, false, false, true));
    }
}
