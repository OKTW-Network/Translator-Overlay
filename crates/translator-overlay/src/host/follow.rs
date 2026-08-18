//! Target follow via out-of-context `SetWinEventHook`.

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

use tracing::warn;
use windows::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    UI::{
        Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
        WindowsAndMessaging::{
            EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_SHOW, EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, OBJID_WINDOW, PostThreadMessageW, WINEVENT_OUTOFCONTEXT, WM_APP,
        },
    },
};

pub(crate) static FOLLOW_TARGET: AtomicIsize = AtomicIsize::new(0);
pub(crate) static FOLLOW_THREAD: AtomicU32 = AtomicU32::new(0);
pub(crate) static FOLLOW_SYNC: AtomicBool = AtomicBool::new(false);

pub(crate) fn wake_overlay_thread() {
    let thread_id = FOLLOW_THREAD.load(Ordering::Relaxed);
    if thread_id != 0 {
        let _ = unsafe { PostThreadMessageW(thread_id, WM_APP, WPARAM(0), LPARAM(0)) };
    }
}

pub(crate) fn request_follow_sync() {
    if !FOLLOW_SYNC.swap(true, Ordering::AcqRel) {
        wake_overlay_thread();
    }
}

fn is_follow_target(hwnd: HWND) -> bool {
    let target = FOLLOW_TARGET.load(Ordering::Relaxed);
    target != 0 && hwnd.0 as isize == target
}

pub(crate) fn install_follow_hooks() -> [HWINEVENTHOOK; 4] {
    let foreground = unsafe {
        SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    let minimize = unsafe {
        SetWinEventHook(EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    let location = unsafe {
        SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    let lifecycle =
        unsafe { SetWinEventHook(EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE, None, Some(on_follow_event), 0, 0, WINEVENT_OUTOFCONTEXT) };
    if foreground.is_invalid() || minimize.is_invalid() || location.is_invalid() || lifecycle.is_invalid() {
        warn!("overlay follow WinEvent hooks failed to install");
    }
    [foreground, minimize, location, lifecycle]
}

pub(crate) fn uninstall_follow_hooks(hooks: &mut [HWINEVENTHOOK; 4]) {
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
    match event {
        EVENT_SYSTEM_FOREGROUND => request_follow_sync(),
        EVENT_SYSTEM_MINIMIZESTART | EVENT_SYSTEM_MINIMIZEEND if is_follow_target(hwnd) => request_follow_sync(),
        EVENT_OBJECT_LOCATIONCHANGE if id_object == OBJID_WINDOW.0 && is_follow_target(hwnd) => request_follow_sync(),
        EVENT_OBJECT_DESTROY | EVENT_OBJECT_SHOW | EVENT_OBJECT_HIDE if id_object == OBJID_WINDOW.0 && is_follow_target(hwnd) => {
            request_follow_sync();
        }
        _ => {}
    }
}
