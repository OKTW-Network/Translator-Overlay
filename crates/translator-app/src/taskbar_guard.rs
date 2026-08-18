//! Restore the shell taskbar's topmost z-order after a WinUI 3 window activates.
//!
//! WASDK / WinUI 3 can drop `Shell_TrayWnd` out of the topmost stack on launch
//! ([microsoft-ui-xaml#11091](https://github.com/microsoft/microsoft-ui-xaml/issues/11091)).
//! Auto-hide then fails once any window covers the 1px hover strip. Re-asserting
//! `HWND_TOPMOST` on the tray is the same recovery as clicking the taskbar.

use std::mem::size_of;

use tracing::debug;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM},
        UI::{
            Shell::{ABM_WINDOWPOSCHANGED, APPBARDATA, SHAppBarMessage},
            WindowsAndMessaging::{EnumWindows, GetClassNameW, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos},
        },
    },
    core::BOOL,
};

const TRAY_CLASSES: [&str; 2] = ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

/// Re-assert `HWND_TOPMOST` on every shell taskbar window (primary + secondary).
pub fn restore_taskbar_zorder() {
    if let Err(e) = unsafe { EnumWindows(Some(enum_tray), LPARAM(0)) } {
        debug!(error = %e, "EnumWindows for taskbar restore failed");
    }
}

unsafe extern "system" fn enum_tray(hwnd: HWND, _: LPARAM) -> BOOL {
    if is_tray(hwnd) {
        bump_tray(hwnd);
    }
    true.into()
}

fn is_tray(hwnd: HWND) -> bool {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    if n <= 0 {
        return false;
    }
    let Ok(name) = String::from_utf16(&buf[..n as usize]) else {
        return false;
    };
    TRAY_CLASSES.contains(&name.as_str())
}

fn bump_tray(hwnd: HWND) {
    if let Err(e) = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) } {
        debug!(error = %e, "SetWindowPos(HWND_TOPMOST) on taskbar failed");
        return;
    }
    let mut data = APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    };
    let _ = unsafe { SHAppBarMessage(ABM_WINDOWPOSCHANGED, &mut data) };
}
