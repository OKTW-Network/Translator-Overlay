//! Flash the control window's taskbar button when a translate API error lands
//! while the dashboard is not focused. Never steals foreground.

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

/// Flash the control-window taskbar until it is focused. No-op if already focused.
pub fn flash_control_window_taskbar() {
    // TODO: windows-reactor 0.100 `WindowRef` has no HWND (`request_close` only). GitHub master
    // exposes `context.run_window(|w| w.as_raw())` via `IWindowNative::WindowHandle` — switch
    // when that ships on crates.io instead of pid + `WinUIDesktopWin32WindowClass`.
    let mut hwnd = HWND::default();
    if unsafe { EnumWindows(Some(enum_control), LPARAM(std::ptr::from_mut(&mut hwnd) as isize)) }.is_err() || hwnd.is_invalid() {
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
