//! Target client geometry and overlay Z-order placement.

use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::{
        GW_HWNDPREV, GWL_EXSTYLE, GetClientRect, GetWindow, GetWindowLongPtrW, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowPos, WS_EX_TOPMOST,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZOrderAnchor {
    Preserve,
    After(isize),
    Top,
    Topmost,
}

fn choose_z_order_anchor(overlay: isize, above_target: Option<isize>, target_topmost: bool) -> ZOrderAnchor {
    match above_target {
        Some(hwnd) if hwnd == overlay => ZOrderAnchor::Preserve,
        Some(hwnd) => ZOrderAnchor::After(hwnd),
        None if target_topmost => ZOrderAnchor::Topmost,
        None => ZOrderAnchor::Top,
    }
}

fn is_topmost(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    style & WS_EX_TOPMOST.0 != 0
}

/// Keep `overlay` immediately above `target`, without making a normal target
/// globally topmost. Windows above the target therefore cover both windows.
pub(crate) fn place_overlay_above_target(
    overlay: HWND,
    target: HWND,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> windows::core::Result<()> {
    let target_topmost = is_topmost(target);
    if is_topmost(overlay) != target_topmost {
        let band = if target_topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
        unsafe { SetWindowPos(overlay, Some(band), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) }?;
    }

    // Re-read after synchronizing the topmost band: that operation itself may
    // have changed which window is immediately above the target.
    let above_target = unsafe { GetWindow(target, GW_HWNDPREV) }.ok().map(|hwnd| hwnd.0 as isize);
    let anchor = choose_z_order_anchor(overlay.0 as isize, above_target, target_topmost);
    let flags = SWP_NOACTIVATE | SWP_SHOWWINDOW;
    match anchor {
        ZOrderAnchor::Preserve => unsafe { SetWindowPos(overlay, None, x, y, width, height, flags | SWP_NOZORDER) },
        ZOrderAnchor::After(hwnd) => unsafe { SetWindowPos(overlay, Some(HWND(hwnd as *mut _)), x, y, width, height, flags) },
        ZOrderAnchor::Top => unsafe { SetWindowPos(overlay, Some(HWND_TOP), x, y, width, height, flags) },
        ZOrderAnchor::Topmost => unsafe { SetWindowPos(overlay, Some(HWND_TOPMOST), x, y, width, height, flags) },
    }
}

/// Client-area rectangle in screen coordinates (left, top, width, height).
pub(crate) fn client_screen_rect(target: HWND) -> Option<(i32, i32, i32, i32)> {
    let mut client = RECT::default();
    if unsafe { GetClientRect(target, &mut client) }.is_err() {
        return None;
    }
    let mut tl = POINT {
        x: client.left,
        y: client.top,
    };
    let mut br = POINT {
        x: client.right,
        y: client.bottom,
    };
    if !unsafe { ClientToScreen(target, &mut tl) }.as_bool() || !unsafe { ClientToScreen(target, &mut br) }.as_bool() {
        return None;
    }
    let w = (br.x - tl.x).max(1);
    let h = (br.y - tl.y).max(1);
    Some((tl.x, tl.y, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_order_preserves_when_overlay_is_already_above_target() {
        assert_eq!(choose_z_order_anchor(20, Some(20), false), ZOrderAnchor::Preserve);
    }

    #[test]
    fn z_order_inserts_after_existing_predecessor() {
        assert_eq!(choose_z_order_anchor(20, Some(30), false), ZOrderAnchor::After(30));
    }

    #[test]
    fn z_order_uses_top_for_normal_target_at_front() {
        assert_eq!(choose_z_order_anchor(20, None, false), ZOrderAnchor::Top);
    }

    #[test]
    fn z_order_uses_topmost_only_for_topmost_target_at_front() {
        assert_eq!(choose_z_order_anchor(20, None, true), ZOrderAnchor::Topmost);
    }
}
