//! Target follow via out-of-context `SetWinEventHook`.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

use tracing::warn;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::{
        Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
        WindowsAndMessaging::{
            EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_REORDER, EVENT_OBJECT_SHOW,
            EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZEEND,
            EVENT_SYSTEM_MOVESIZESTART, OBJID_WINDOW, PostThreadMessageW, WINEVENT_OUTOFCONTEXT, WM_APP,
        },
    },
};

use crate::host::win32::set_overlay_owner;

pub(crate) static FOLLOW_TARGET: AtomicIsize = AtomicIsize::new(0);
pub(crate) static FOLLOW_OVERLAY: AtomicIsize = AtomicIsize::new(0);
pub(crate) static FOLLOW_THREAD: AtomicU32 = AtomicU32::new(0);
pub(crate) static FOLLOW_SYNC: AtomicBool = AtomicBool::new(false);
pub(crate) static FOLLOW_IN_MOVESIZE: AtomicBool = AtomicBool::new(false);
pub(crate) static FOLLOW_SETTLE: AtomicBool = AtomicBool::new(false);
/// Dedicated wake for target geometry / Z-order — must not coalesce with `WM_APP`.
pub(crate) const FOLLOW_EVENT_MESSAGE: u32 = WM_APP + 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FollowAction {
    Ignore,
    Sync,
    /// Clear ownership before Windows destroys an owned overlay with its target.
    ClearDyingTarget,
}

pub(crate) fn wake_overlay_thread() {
    let thread_id = FOLLOW_THREAD.load(Ordering::Relaxed);
    if thread_id != 0 {
        let _ = unsafe { PostThreadMessageW(thread_id, WM_APP, WPARAM(0), LPARAM(0)) };
    }
}

pub(crate) fn request_follow_sync() {
    if FOLLOW_SYNC.swap(true, Ordering::AcqRel) {
        return;
    }
    let thread_id = FOLLOW_THREAD.load(Ordering::Relaxed);
    if thread_id != 0 {
        let _ = unsafe { PostThreadMessageW(thread_id, FOLLOW_EVENT_MESSAGE, WPARAM(0), LPARAM(0)) };
    }
}

pub(crate) fn clear_follow_move_state() {
    FOLLOW_IN_MOVESIZE.store(false, Ordering::Release);
    FOLLOW_SYNC.store(false, Ordering::Release);
    FOLLOW_SETTLE.store(false, Ordering::Release);
}

fn clear_owner_for_dying_target() {
    let overlay = FOLLOW_OVERLAY.load(Ordering::Relaxed);
    if overlay != 0 {
        set_overlay_owner(HWND(overlay as *mut _), None);
    }
    FOLLOW_TARGET.store(0, Ordering::Release);
    clear_follow_move_state();
    request_follow_sync();
}

fn classify_follow_event(event: u32, is_target: bool, is_overlay: bool, is_window_object: bool, in_movesize: bool) -> FollowAction {
    match event {
        EVENT_SYSTEM_FOREGROUND => FollowAction::Sync,
        // Top-level Z-order changes report on the parent/desktop, not the target.
        // Skip during interactive move/size — our own SetWindowPos also fires REORDER.
        EVENT_OBJECT_REORDER if is_window_object && !in_movesize => FollowAction::Sync,
        EVENT_SYSTEM_MINIMIZESTART | EVENT_SYSTEM_MINIMIZEEND if is_target => FollowAction::Sync,
        EVENT_SYSTEM_MOVESIZESTART | EVENT_SYSTEM_MOVESIZEEND if is_target && is_window_object => FollowAction::Sync,
        EVENT_OBJECT_LOCATIONCHANGE | EVENT_OBJECT_SHOW | EVENT_OBJECT_HIDE if is_target && is_window_object => FollowAction::Sync,
        EVENT_OBJECT_DESTROY if is_target && is_window_object => FollowAction::ClearDyingTarget,
        EVENT_OBJECT_DESTROY if is_overlay && is_window_object => FollowAction::Sync,
        _ => FollowAction::Ignore,
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
    if is_target && is_window_object {
        match event {
            EVENT_SYSTEM_MOVESIZESTART => {
                FOLLOW_IN_MOVESIZE.store(true, Ordering::Release);
                FOLLOW_SETTLE.store(false, Ordering::Release);
            }
            EVENT_SYSTEM_MOVESIZEEND => {
                FOLLOW_IN_MOVESIZE.store(false, Ordering::Release);
                FOLLOW_SETTLE.store(true, Ordering::Release);
            }
            _ => {}
        }
    }
    let action = classify_follow_event(event, is_target, is_overlay, is_window_object, FOLLOW_IN_MOVESIZE.load(Ordering::Acquire));
    match action {
        FollowAction::Ignore => {}
        FollowAction::Sync => request_follow_sync(),
        FollowAction::ClearDyingTarget => clear_owner_for_dying_target(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_events_only_sync_the_target_window() {
        assert_eq!(classify_follow_event(EVENT_SYSTEM_MOVESIZESTART, true, false, true, false), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_SYSTEM_MOVESIZEEND, true, false, true, false), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_SYSTEM_MOVESIZESTART, false, false, true, false), FollowAction::Ignore);
        assert_eq!(classify_follow_event(EVENT_SYSTEM_MOVESIZESTART, true, false, false, false), FollowAction::Ignore);
        assert_eq!(classify_follow_event(EVENT_SYSTEM_MOVESIZESTART, true, false, true, true), FollowAction::Sync);
    }

    #[test]
    fn location_and_foreground_events_request_a_sync() {
        assert_eq!(classify_follow_event(EVENT_OBJECT_LOCATIONCHANGE, true, false, true, false), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_OBJECT_LOCATIONCHANGE, false, false, true, false), FollowAction::Ignore);
        assert_eq!(classify_follow_event(EVENT_OBJECT_LOCATIONCHANGE, true, false, true, true), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_SYSTEM_FOREGROUND, false, false, false, false), FollowAction::Sync);
    }

    #[test]
    fn window_reorder_syncs_for_the_parent_container() {
        assert_eq!(classify_follow_event(EVENT_OBJECT_REORDER, false, false, true, false), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_OBJECT_REORDER, true, false, true, false), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_OBJECT_REORDER, false, false, false, false), FollowAction::Ignore);
    }

    #[test]
    fn window_reorder_is_ignored_during_movesize() {
        assert_eq!(classify_follow_event(EVENT_OBJECT_REORDER, false, false, true, true), FollowAction::Ignore);
        assert_eq!(classify_follow_event(EVENT_OBJECT_REORDER, true, false, true, true), FollowAction::Ignore);
    }

    #[test]
    fn destroy_clears_dying_target_and_resyncs_overlay() {
        assert_eq!(classify_follow_event(EVENT_OBJECT_DESTROY, true, false, true, false), FollowAction::ClearDyingTarget);
        assert_eq!(classify_follow_event(EVENT_OBJECT_DESTROY, false, true, true, false), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_OBJECT_DESTROY, false, false, true, false), FollowAction::Ignore);
    }
}
