//! Flash the control window's taskbar button when a translate API error arrives
//! while the dashboard is not focused. It never takes the foreground.

use std::mem::size_of;

use tracing::debug;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FlashWindowEx, GA_ROOTOWNER, GetAncestor, GetClassNameW,
            GetForegroundWindow, GetWindowThreadProcessId,
        },
    },
    core::{BOOL, w},
};

/// Flash the control window's taskbar button until it is focused. Does nothing if it already is.
pub fn flash_control_window_taskbar() {
    // TODO: In windows-reactor 0.100, `WindowRef` has no HWND, only `request_close`. GitHub master
    // exposes `context.run_window(|w| w.as_raw())` through `IWindowNative::WindowHandle`. When that
    // ships on crates.io, use it instead of matching the pid and `WinUIDesktopWin32WindowClass`.
    let mut hwnd = HWND::default();
    // The callback returns FALSE to stop the walk, and windows-rs turns that into Err even after a match.
    let _ = unsafe { EnumWindows(Some(enum_control), LPARAM(std::ptr::from_mut(&mut hwnd) as isize)) };
    if hwnd.is_invalid() {
        debug!("control window HWND not found — skip taskbar flash");
        return;
    }
    if unsafe { GetAncestor(GetForegroundWindow(), GA_ROOTOWNER) == hwnd } {
        return;
    }
    let info = FLASHWINFO {
        cbSize: size_of::<FLASHWINFO>() as u32,
        hwnd,
        dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG,
        uCount: 0,
        dwTimeout: 0,
    };
    let _ = unsafe { FlashWindowEx(&info) };
}

unsafe extern "system" fn enum_control(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is `&mut HWND` for the duration of `EnumWindows`.
    let found = unsafe { &mut *(lparam.0 as *mut HWND) };
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(std::ptr::from_mut(&mut pid))) };
    if pid != std::process::id() {
        return true.into();
    }
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    let class = w!("WinUIDesktopWin32WindowClass");
    if n > 0 && buf.get(..n as usize) == Some(unsafe { class.as_wide() }) {
        *found = hwnd;
        return false.into();
    }
    true.into()
}
