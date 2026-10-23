//! Control-window HWND helpers: taskbar flash.

use std::mem::size_of;

use tracing::debug;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FlashWindowEx, GA_ROOTOWNER, GetAncestor, GetForegroundWindow},
};

/// Flash the control-window taskbar until it is focused. No-op if already focused.
pub fn flash_control_window_taskbar() {
    let Some(hwnd) = crate::ui::control_hwnd() else {
        debug!("control window HWND not known yet — skip taskbar flash");
        return;
    };
    let hwnd = HWND(hwnd as *mut _);
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
