//! Target client geometry and foreground tests.

use translator_capture::client_screen_rect as capture_client_rect;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, IsChild},
};

/// True when the picker may stay visible: target focused, or the picker
/// window itself (clicks on the layer must not hide it).
pub(crate) fn is_picker_allowed_foreground(target: HWND, overlay: HWND, fg: HWND) -> bool {
    if is_target_in_foreground(target, fg) {
        return true;
    }
    !overlay.is_invalid() && !fg.is_invalid() && fg == overlay
}

/// True when `fg` is the capture target or a child / same top-level tree.
pub(crate) fn is_target_in_foreground(target: HWND, fg: HWND) -> bool {
    if fg.is_invalid() || target.is_invalid() {
        return false;
    }
    if fg == target {
        return true;
    }
    // Focused child control inside the target window.
    if unsafe { IsChild(target, fg) }.as_bool() {
        return true;
    }
    // Same top-level root (e.g. owned popups under the target).
    let fg_root = unsafe { GetAncestor(fg, GA_ROOT) };
    !fg_root.is_invalid() && fg_root == target
}

pub(crate) fn client_screen_rect(target: HWND) -> Option<(i32, i32, i32, i32)> {
    capture_client_rect(target.0 as isize)
}
