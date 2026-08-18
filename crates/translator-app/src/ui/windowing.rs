//! Win32 guard for a WinUI bug that can leave the Taskbar below normal windows.

use anyhow::{Context, Result, anyhow};
use tracing::{debug, warn};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::Threading::GetCurrentThreadId,
        UI::{
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{
                EnumThreadWindows, EnumWindows, GWL_EXSTYLE, GetClassNameW, GetWindowLongPtrW, GetWindowTextW, HWND_NOTOPMOST,
                HWND_TOPMOST, PostMessageW, RegisterWindowMessageW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
                SetWindowPos, WM_ACTIVATEAPP, WM_NCDESTROY, WS_EX_TOPMOST,
            },
        },
    },
    core::{BOOL, Error, w},
};

use crate::APP_TITLE;

const MAIN_WINDOW_CLASS: &str = "WinUIDesktopWin32WindowClass";
const PRIMARY_TASKBAR_CLASS: &str = "Shell_TrayWnd";
const SECONDARY_TASKBAR_CLASS: &str = "Shell_SecondaryTrayWnd";
const TASKBAR_Z_ORDER_SUBCLASS_ID: usize = 0x544f_5a4f;
const RESTORE_MESSAGE_NAME: windows::core::PCWSTR = w!("TranslatorOverlay.RestoreTaskbarZOrder");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MessageAction {
    Delegate,
    ScheduleRestore,
    RestoreTaskbars,
    RemoveSubclass,
}

#[derive(Default)]
struct MainWindowSearch {
    hwnd: Option<HWND>,
}

#[derive(Default)]
struct RestackStats {
    matched: u32,
    restored_topmost: u32,
    restored_non_topmost: u32,
    restored: u32,
    failed: u32,
}

/// Installs the workaround on the current WinUI control window.
pub(crate) fn install_taskbar_z_order_guard() -> Result<()> {
    let hwnd = find_main_window()?;
    let restore_message = unsafe { RegisterWindowMessageW(RESTORE_MESSAGE_NAME) };
    if restore_message == 0 {
        return Err(anyhow!("RegisterWindowMessageW failed: {}", Error::from_thread()));
    }

    let installed =
        unsafe { SetWindowSubclass(hwnd, Some(main_window_subclass_proc), TASKBAR_Z_ORDER_SUBCLASS_ID, restore_message as usize) };
    if !installed.as_bool() {
        return Err(anyhow!("SetWindowSubclass failed: {}", Error::from_thread()));
    }

    if let Err(error) = post_restore_message(hwnd, restore_message) {
        unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(main_window_subclass_proc), TASKBAR_Z_ORDER_SUBCLASS_ID);
        }
        return Err(error).context("failed to schedule initial Taskbar z-order restore");
    }

    debug!(?hwnd, "installed Taskbar z-order guard");
    Ok(())
}

fn find_main_window() -> Result<HWND> {
    let mut search = MainWindowSearch::default();
    let context = LPARAM((&raw mut search).cast::<()>() as isize);

    unsafe {
        let _ = EnumThreadWindows(GetCurrentThreadId(), Some(find_main_window_callback), context);
    }

    search
        .hwnd
        .ok_or_else(|| anyhow!("WinUI main window HWND was not found on the UI thread"))
}

unsafe extern "system" fn find_main_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let search = unsafe { &mut *(lparam.0 as *mut MainWindowSearch) };
    if window_class_matches(hwnd, MAIN_WINDOW_CLASS) && window_title_matches(hwnd, APP_TITLE) {
        search.hwnd = Some(hwnd);
        return BOOL(0);
    }

    BOOL(1)
}

unsafe extern "system" fn main_window_subclass_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass_id: usize,
    restore_message: usize,
) -> LRESULT {
    let restore_message = restore_message as u32;
    match classify_message(message, wparam.0, restore_message) {
        MessageAction::Delegate => unsafe { DefSubclassProc(hwnd, message, wparam, lparam) },
        MessageAction::ScheduleRestore => {
            // Let WinUI finish its activation handling before queueing the repair.
            let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
            if let Err(error) = post_restore_message(hwnd, restore_message) {
                warn!(%error, "failed to schedule Taskbar z-order restore");
            }
            result
        }
        MessageAction::RestoreTaskbars => {
            restore_taskbar_z_order();
            LRESULT(0)
        }
        MessageAction::RemoveSubclass => {
            let removed = unsafe { RemoveWindowSubclass(hwnd, Some(main_window_subclass_proc), subclass_id) };
            if !removed.as_bool() {
                warn!("failed to remove Taskbar z-order window subclass");
            }
            unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
        }
    }
}

fn classify_message(message: u32, wparam: usize, restore_message: u32) -> MessageAction {
    if message == restore_message {
        MessageAction::RestoreTaskbars
    } else if message == WM_ACTIVATEAPP && wparam != 0 {
        MessageAction::ScheduleRestore
    } else if message == WM_NCDESTROY {
        MessageAction::RemoveSubclass
    } else {
        MessageAction::Delegate
    }
}

fn post_restore_message(hwnd: HWND, restore_message: u32) -> windows::core::Result<()> {
    unsafe { PostMessageW(Some(hwnd), restore_message, WPARAM(0), LPARAM(0)) }
}

fn restore_taskbar_z_order() {
    let mut stats = RestackStats::default();
    let context = LPARAM((&raw mut stats).cast::<()>() as isize);
    if let Err(error) = unsafe { EnumWindows(Some(restack_taskbar_callback), context) } {
        warn!(%error, "failed to enumerate Taskbar windows");
        return;
    }

    if stats.failed != 0 {
        warn!(
            matched = stats.matched,
            restored = stats.restored,
            failed = stats.failed,
            "failed to restore one or more Taskbar windows to the topmost band"
        );
    } else {
        debug!(
            matched = stats.matched,
            restored = stats.restored,
            restored_topmost = stats.restored_topmost,
            restored_non_topmost = stats.restored_non_topmost,
            "restored Taskbar z-order"
        );
    }
}

unsafe extern "system" fn restack_taskbar_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut class_name = [0_u16; 64];
    let class_name_length = unsafe { GetClassNameW(hwnd, &mut class_name) };
    if class_name_length <= 0 || !is_taskbar_class(&class_name[..class_name_length as usize]) {
        return BOOL(1);
    }

    let stats = unsafe { &mut *(lparam.0 as *mut RestackStats) };
    stats.matched += 1;

    let extended_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    // Current Windows 11 builds may keep the standard Taskbar in the normal
    // band without WS_EX_TOPMOST. A TOPMOST -> NOTOPMOST round trip restores
    // it to the front of that band while preserving its original style. When
    // the shell does expose TOPMOST, only reassert that existing state.
    let is_topmost = extended_style & WS_EX_TOPMOST.0 != 0;
    let result = restack_taskbar(hwnd, is_topmost);
    if result.is_ok() {
        stats.restored += 1;
        if is_topmost {
            stats.restored_topmost += 1;
        } else {
            stats.restored_non_topmost += 1;
        }
    } else {
        stats.failed += 1;
    }

    BOOL(1)
}

fn restack_taskbar(hwnd: HWND, was_topmost: bool) -> windows::core::Result<()> {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER;
    unsafe {
        SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags)?;
        if !was_topmost {
            SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags)?;
        }
    }
    Ok(())
}

fn window_class_matches(hwnd: HWND, expected: &str) -> bool {
    let mut class_name = [0_u16; 64];
    let length = unsafe { GetClassNameW(hwnd, &mut class_name) };
    length > 0 && utf16_matches(&class_name[..length as usize], expected)
}

fn window_title_matches(hwnd: HWND, expected: &str) -> bool {
    let mut title = [0_u16; 128];
    let length = unsafe { GetWindowTextW(hwnd, &mut title) };
    length > 0 && utf16_matches(&title[..length as usize], expected)
}

fn is_taskbar_class(class_name: &[u16]) -> bool {
    utf16_matches(class_name, PRIMARY_TASKBAR_CLASS) || utf16_matches(class_name, SECONDARY_TASKBAR_CLASS)
}

fn utf16_matches(value: &[u16], expected: &str) -> bool {
    value.iter().copied().eq(expected.encode_utf16())
}

#[cfg(test)]
mod tests {
    use windows::Win32::UI::WindowsAndMessaging::{WM_ACTIVATEAPP, WM_NCDESTROY};

    use crate::ui::windowing::{MessageAction, classify_message, is_taskbar_class};

    const TEST_RESTORE_MESSAGE: u32 = 0xc123;

    #[test]
    fn schedules_restore_only_for_app_activation() {
        assert_eq!(classify_message(WM_ACTIVATEAPP, 1, TEST_RESTORE_MESSAGE), MessageAction::ScheduleRestore);
        assert_eq!(classify_message(WM_ACTIVATEAPP, 0, TEST_RESTORE_MESSAGE), MessageAction::Delegate);
        assert_eq!(classify_message(0x0006, 1, TEST_RESTORE_MESSAGE), MessageAction::Delegate);
    }

    #[test]
    fn private_message_runs_restore_and_destroy_removes_subclass() {
        assert_eq!(classify_message(TEST_RESTORE_MESSAGE, 0, TEST_RESTORE_MESSAGE), MessageAction::RestoreTaskbars);
        assert_eq!(classify_message(WM_NCDESTROY, 0, TEST_RESTORE_MESSAGE), MessageAction::RemoveSubclass);
    }

    #[test]
    fn recognizes_only_primary_and_secondary_taskbar_classes() {
        for class_name in ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"] {
            let class_name: Vec<u16> = class_name.encode_utf16().collect();
            assert!(is_taskbar_class(&class_name));
        }

        for class_name in ["shell_traywnd", "Shell_TrayWndExtra", "Progman", ""] {
            let class_name: Vec<u16> = class_name.encode_utf16().collect();
            assert!(!is_taskbar_class(&class_name));
        }
    }
}
