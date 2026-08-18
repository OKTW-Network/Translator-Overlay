//! Out-of-process target resize via `SetWinEventHook`.
//!
//! Interactive drag uses `EVENT_SYSTEM_MOVESIZESTART` / `END` — pause frame
//! publish for the whole loop, and restart WGC only when the user releases if
//! the size changed. Maximize / snap / `SetWindowPos` do not enter that loop;
//! those go through `EVENT_OBJECT_LOCATIONCHANGE` and restart as soon as the
//! size changes.
//!
//! The capture session has no Win32 message pump, so the hooks live on a
//! dedicated thread. One session / process — the callback reads process-wide
//! atomics.

use std::{
    sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering},
    thread::JoinHandle,
};

use tracing::warn;
use windows::Win32::{
    Foundation::{HWND, LPARAM, RECT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
        WindowsAndMessaging::{
            DispatchMessageW, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZESTART, GetMessageW,
            GetWindowRect, MSG, OBJID_WINDOW, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, TranslateMessage, WINEVENT_OUTOFCONTEXT,
            WM_QUIT, WM_USER,
        },
    },
};

static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);
static PENDING: AtomicBool = AtomicBool::new(false);
pub(crate) static IN_MOVESIZE: AtomicBool = AtomicBool::new(false);
static LAST_W: AtomicU32 = AtomicU32::new(0);
static LAST_H: AtomicU32 = AtomicU32::new(0);

pub(crate) struct ResizeWatch {
    thread_id: u32,
    join: Option<JoinHandle<()>>,
}

impl ResizeWatch {
    pub(crate) fn new() -> Self {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        // SetWinEventHook callbacks run on this thread; needs GetMessageW.
        let join = std::thread::Builder::new()
            .name("capture-resize".into())
            .spawn(move || hook_thread(ready_tx))
            .ok();
        let Some(join) = join else {
            warn!("failed to spawn capture resize hook thread");
            return Self { thread_id: 0, join: None };
        };
        match ready_rx.recv() {
            Ok(thread_id) if thread_id != 0 => Self {
                thread_id,
                join: Some(join),
            },
            _ => {
                warn!("capture resize hook failed to start");
                let _ = join.join();
                Self { thread_id: 0, join: None }
            }
        }
    }

    pub(crate) fn set_target(&self, hwnd: isize) {
        TARGET_HWND.store(hwnd, Ordering::Release);
        IN_MOVESIZE.store(false, Ordering::Release);
        if let Some((w, h)) = window_size(hwnd) {
            LAST_W.store(w, Ordering::Relaxed);
            LAST_H.store(h, Ordering::Relaxed);
        } else {
            LAST_W.store(0, Ordering::Relaxed);
            LAST_H.store(0, Ordering::Relaxed);
        }
        PENDING.store(false, Ordering::Release);
    }

    pub(crate) fn take_pending(&self) -> bool {
        PENDING.swap(false, Ordering::AcqRel)
    }

    pub(crate) fn in_movesize(&self) -> bool {
        IN_MOVESIZE.load(Ordering::Acquire)
    }
}

impl Drop for ResizeWatch {
    fn drop(&mut self) {
        TARGET_HWND.store(0, Ordering::Release);
        IN_MOVESIZE.store(false, Ordering::Release);
        PENDING.store(false, Ordering::Release);
        if self.thread_id != 0 {
            // SAFETY: `thread_id` is the hook thread; `WM_QUIT` ends `GetMessageW`.
            let _ = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn window_size(hwnd: isize) -> Option<(u32, u32)> {
    if hwnd == 0 {
        return None;
    }
    let hwnd = HWND(hwnd as *mut _);
    let mut rect = RECT::default();
    // SAFETY: `hwnd` is a window handle; `rect` is a valid out-param.
    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    let width = (rect.right - rect.left).max(0) as u32;
    let height = (rect.bottom - rect.top).max(0) as u32;
    (width > 0 && height > 0).then_some((width, height))
}

fn mark_if_resized(target: isize) {
    let Some((w, h)) = window_size(target) else {
        return;
    };
    if w == LAST_W.load(Ordering::Relaxed) && h == LAST_H.load(Ordering::Relaxed) {
        return;
    }
    LAST_W.store(w, Ordering::Relaxed);
    LAST_H.store(h, Ordering::Relaxed);
    PENDING.store(true, Ordering::Release);
}

fn hook_thread(ready: std::sync::mpsc::Sender<u32>) {
    let mut msg = MSG::default();
    // Create the thread message queue before advertising `thread_id`.
    let _ = unsafe { PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE) };
    let thread_id = unsafe { GetCurrentThreadId() };
    let movesize = unsafe {
        SetWinEventHook(EVENT_SYSTEM_MOVESIZESTART, EVENT_SYSTEM_MOVESIZEEND, None, Some(on_win_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    let location = unsafe {
        SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(on_win_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    if movesize.is_invalid() || location.is_invalid() {
        if !movesize.is_invalid() {
            let _ = unsafe { UnhookWinEvent(movesize) };
        }
        if !location.is_invalid() {
            let _ = unsafe { UnhookWinEvent(location) };
        }
        let _ = ready.send(0);
        return;
    }
    let _ = ready.send(thread_id);
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        let _ = unsafe { TranslateMessage(&msg) };
        unsafe { DispatchMessageW(&msg) };
    }
    let _ = unsafe { UnhookWinEvent(movesize) };
    let _ = unsafe { UnhookWinEvent(location) };
}

unsafe extern "system" fn on_win_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if id_object != OBJID_WINDOW.0 {
        return;
    }
    let target = TARGET_HWND.load(Ordering::Relaxed);
    if target == 0 || hwnd.0 as isize != target {
        return;
    }
    match classify_resize_event(event, IN_MOVESIZE.load(Ordering::Relaxed)) {
        ResizeEventAction::BeginMovesize => IN_MOVESIZE.store(true, Ordering::Release),
        ResizeEventAction::EndMovesize => {
            IN_MOVESIZE.store(false, Ordering::Release);
            mark_if_resized(target);
        }
        ResizeEventAction::NoteLocation => mark_if_resized(target),
        ResizeEventAction::Ignore => {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeEventAction {
    Ignore,
    BeginMovesize,
    EndMovesize,
    NoteLocation,
}

fn classify_resize_event(event: u32, in_movesize: bool) -> ResizeEventAction {
    match event {
        EVENT_SYSTEM_MOVESIZESTART => ResizeEventAction::BeginMovesize,
        EVENT_SYSTEM_MOVESIZEEND => ResizeEventAction::EndMovesize,
        EVENT_OBJECT_LOCATIONCHANGE if !in_movesize => ResizeEventAction::NoteLocation,
        _ => ResizeEventAction::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::UI::WindowsAndMessaging::{EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZESTART};

    use crate::resize_watch::{ResizeEventAction, classify_resize_event};

    #[test]
    fn interactive_movesize_pauses_location_restarts() {
        assert_eq!(classify_resize_event(EVENT_SYSTEM_MOVESIZESTART, false), ResizeEventAction::BeginMovesize);
        assert_eq!(classify_resize_event(EVENT_OBJECT_LOCATIONCHANGE, true), ResizeEventAction::Ignore);
        assert_eq!(classify_resize_event(EVENT_SYSTEM_MOVESIZEEND, true), ResizeEventAction::EndMovesize);
        assert_eq!(classify_resize_event(EVENT_OBJECT_LOCATIONCHANGE, false), ResizeEventAction::NoteLocation);
    }
}
