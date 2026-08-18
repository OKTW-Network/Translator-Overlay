//! Overlay layered-window procedure and picker hit-test / cursor flags.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
        UI::WindowsAndMessaging::{
            DefWindowProcW, HTCLIENT, HTTRANSPARENT, IDC_CROSS, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE,
            LoadCursorW, MA_NOACTIVATE, PostQuitMessage, SetCursor, WM_DESTROY, WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_NCHITTEST, WM_PAINT,
            WM_SETCURSOR,
        },
    },
    core::{PCWSTR, w},
};

use crate::picker::PickerCursor;

pub(crate) const CLASS_NAME: PCWSTR = w!("TranslatorOverlayLayer.v1");

/// `wnd_proc` cannot reach `OverlayHost`; picker hit-testing is a process-wide flag
/// because this crate hosts a single overlay window.
pub(crate) static PICKER_HIT_TEST: AtomicBool = AtomicBool::new(false);
/// Last picker cursor. `WM_SETCURSOR` is sent (not posted) and hits `wnd_proc`
/// inside `PeekMessage` / `WaitMessage`, so the Peek-loop swallow cannot win.
static PICKER_CURSOR: AtomicU8 = AtomicU8::new(0);

fn picker_cursor_code(kind: PickerCursor) -> u8 {
    match kind {
        PickerCursor::Cross => 0,
        PickerCursor::SizeAll => 1,
        PickerCursor::SizeNs => 2,
        PickerCursor::SizeWe => 3,
        PickerCursor::SizeNwse => 4,
        PickerCursor::SizeNesw => 5,
    }
}

fn picker_cursor_from_code(code: u8) -> PickerCursor {
    match code {
        1 => PickerCursor::SizeAll,
        2 => PickerCursor::SizeNs,
        3 => PickerCursor::SizeWe,
        4 => PickerCursor::SizeNwse,
        5 => PickerCursor::SizeNesw,
        _ => PickerCursor::Cross,
    }
}

fn apply_picker_cursor(kind: PickerCursor) {
    let id = match kind {
        PickerCursor::Cross => IDC_CROSS,
        PickerCursor::SizeAll => IDC_SIZEALL,
        PickerCursor::SizeNs => IDC_SIZENS,
        PickerCursor::SizeWe => IDC_SIZEWE,
        PickerCursor::SizeNwse => IDC_SIZENWSE,
        PickerCursor::SizeNesw => IDC_SIZENESW,
    };
    if let Ok(cur) = unsafe { LoadCursorW(None, id) } {
        let _ = unsafe { SetCursor(Some(cur)) };
    }
}

pub(crate) fn set_picker_cursor(kind: PickerCursor) {
    PICKER_CURSOR.store(picker_cursor_code(kind), Ordering::Relaxed);
    apply_picker_cursor(kind);
}

pub(crate) unsafe extern "system" fn overlay_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => {
            if PICKER_HIT_TEST.load(Ordering::Relaxed) {
                LRESULT(HTCLIENT as isize)
            } else {
                LRESULT(HTTRANSPARENT as isize)
            }
        }
        WM_SETCURSOR => {
            if PICKER_HIT_TEST.load(Ordering::Relaxed) {
                apply_picker_cursor(picker_cursor_from_code(PICKER_CURSOR.load(Ordering::Relaxed)));
                LRESULT(1)
            } else {
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }
        // Picker is WS_EX_NOACTIVATE, but a TOPMOST layered window can
        // still be activated on click. Refuse activation so the target
        // stays foreground while the user draws boxes.
        WM_MOUSEACTIVATE => {
            if PICKER_HIT_TEST.load(Ordering::Relaxed) {
                LRESULT(MA_NOACTIVATE as isize)
            } else {
                unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
            }
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            if !hdc.is_invalid() {
                let _ = unsafe { EndPaint(hwnd, &ps) };
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
