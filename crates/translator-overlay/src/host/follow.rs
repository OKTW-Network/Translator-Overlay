//! Target follow via out-of-context `SetWinEventHook`.

use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};

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

pub(crate) static FOLLOW_TARGET: AtomicIsize = AtomicIsize::new(0);
pub(crate) static FOLLOW_THREAD: AtomicU32 = AtomicU32::new(0);
pub(crate) const FOLLOW_EVENT_MESSAGE: u32 = WM_APP + 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FollowAction {
    Ignore,
    Sync,
}

pub(crate) fn wake_overlay_thread() {
    let thread_id = FOLLOW_THREAD.load(Ordering::Relaxed);
    if thread_id != 0 {
        let _ = unsafe { PostThreadMessageW(thread_id, WM_APP, WPARAM(0), LPARAM(0)) };
    }
}

pub(crate) fn request_follow_sync() {
    let thread_id = FOLLOW_THREAD.load(Ordering::Relaxed);
    if thread_id != 0 {
        // Do not coalesce location events: each one gets a chance to move the
        // overlay before the next target-window position is delivered.
        let _ = unsafe { PostThreadMessageW(thread_id, FOLLOW_EVENT_MESSAGE, WPARAM(0), LPARAM(0)) };
    }
}

fn is_follow_target(hwnd: HWND) -> bool {
    let target = FOLLOW_TARGET.load(Ordering::Relaxed);
    target != 0 && hwnd.0 as isize == target
}

fn classify_follow_event(event: u32, is_target: bool, is_window_object: bool) -> FollowAction {
    match event {
        EVENT_SYSTEM_FOREGROUND => FollowAction::Sync,
        // A top-level Z-order change is reported on its parent/desktop window,
        // so the event HWND does not have to be the capture target.
        EVENT_OBJECT_REORDER if is_window_object => FollowAction::Sync,
        EVENT_SYSTEM_MINIMIZESTART | EVENT_SYSTEM_MINIMIZEEND if is_target => FollowAction::Sync,
        EVENT_SYSTEM_MOVESIZESTART | EVENT_SYSTEM_MOVESIZEEND if is_target && is_window_object => FollowAction::Sync,
        EVENT_OBJECT_LOCATIONCHANGE | EVENT_OBJECT_DESTROY | EVENT_OBJECT_SHOW | EVENT_OBJECT_HIDE if is_target && is_window_object => {
            FollowAction::Sync
        }
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
    let action = classify_follow_event(event, is_follow_target(hwnd), id_object == OBJID_WINDOW.0);
    match action {
        FollowAction::Ignore => {}
        FollowAction::Sync => request_follow_sync(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_events_only_sync_the_target_window() {
        assert_eq!(classify_follow_event(EVENT_SYSTEM_MOVESIZESTART, true, true), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_SYSTEM_MOVESIZEEND, true, true), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_SYSTEM_MOVESIZESTART, false, true), FollowAction::Ignore);
        assert_eq!(classify_follow_event(EVENT_SYSTEM_MOVESIZESTART, true, false), FollowAction::Ignore);
    }

    #[test]
    fn location_and_foreground_events_request_a_sync() {
        assert_eq!(classify_follow_event(EVENT_OBJECT_LOCATIONCHANGE, true, true), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_OBJECT_LOCATIONCHANGE, false, true), FollowAction::Ignore);
        assert_eq!(classify_follow_event(EVENT_SYSTEM_FOREGROUND, false, false), FollowAction::Sync);
    }

    #[test]
    fn window_reorder_syncs_for_the_parent_container() {
        assert_eq!(classify_follow_event(EVENT_OBJECT_REORDER, false, true), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_OBJECT_REORDER, true, true), FollowAction::Sync);
        assert_eq!(classify_follow_event(EVENT_OBJECT_REORDER, false, false), FollowAction::Ignore);
    }
}
