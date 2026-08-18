//! Target client geometry and overlay Z-order placement.

use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::{
        GW_HWNDPREV, GWL_EXSTYLE, GetClientRect, GetWindow, GetWindowLongPtrW, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST,
        SET_WINDOW_POS_FLAGS, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowPos,
        ShowWindow, WS_EX_TOPMOST,
    },
};

pub(crate) type ClientRect = (i32, i32, i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlacementGeometry {
    Preserve,
    Set(ClientRect),
}

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

fn placement_values(geometry: PlacementGeometry, preserve_z_order: bool) -> (i32, i32, i32, i32, SET_WINDOW_POS_FLAGS) {
    let (x, y, width, height, mut flags) = match geometry {
        PlacementGeometry::Preserve => (0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW),
        PlacementGeometry::Set((x, y, width, height)) => (x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW),
    };
    if preserve_z_order {
        flags |= SWP_NOZORDER;
    }
    (x, y, width, height, flags)
}

fn set_window_pos(
    overlay: HWND,
    insert_after: Option<HWND>,
    geometry: PlacementGeometry,
    preserve_z_order: bool,
) -> windows::core::Result<()> {
    let (x, y, width, height, flags) = placement_values(geometry, preserve_z_order);
    unsafe { SetWindowPos(overlay, insert_after, x, y, width, height, flags) }
}

/// Keep `overlay` immediately above `target`, without making a normal target
/// globally topmost. Windows above the target therefore cover both windows.
pub(crate) fn place_overlay_above_target(overlay: HWND, target: HWND, geometry: PlacementGeometry) -> windows::core::Result<()> {
    let target_topmost = is_topmost(target);
    if is_topmost(overlay) != target_topmost {
        let band = if target_topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
        unsafe { SetWindowPos(overlay, Some(band), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) }?;
    }

    // Re-read after synchronizing the topmost band: that operation itself may
    // have changed which window is immediately above the target.
    let above_target = unsafe { GetWindow(target, GW_HWNDPREV) }.ok().map(|hwnd| hwnd.0 as isize);
    let anchor = choose_z_order_anchor(overlay.0 as isize, above_target, target_topmost);
    match anchor {
        ZOrderAnchor::Preserve if geometry == PlacementGeometry::Preserve => {
            let _ = unsafe { ShowWindow(overlay, SW_SHOWNOACTIVATE) };
            Ok(())
        }
        ZOrderAnchor::Preserve => set_window_pos(overlay, None, geometry, true),
        ZOrderAnchor::After(hwnd) => set_window_pos(overlay, Some(HWND(hwnd as *mut _)), geometry, false),
        ZOrderAnchor::Top => set_window_pos(overlay, Some(HWND_TOP), geometry, false),
        ZOrderAnchor::Topmost => set_window_pos(overlay, Some(HWND_TOPMOST), geometry, false),
    }
}

/// Keep the current Z-order when a relative insertion races a disappearing
/// window. Visibility and geometry are more important than exact stacking.
pub(crate) fn position_overlay_preserving_z_order(overlay: HWND, rect: ClientRect) -> windows::core::Result<()> {
    set_window_pos(overlay, None, PlacementGeometry::Set(rect), true)
}

pub(crate) fn show_overlay(overlay: HWND) {
    let _ = unsafe { ShowWindow(overlay, SW_SHOWNOACTIVATE) };
}

/// Client-area rectangle in screen coordinates (left, top, width, height).
pub(crate) fn client_screen_rect(target: HWND) -> Option<ClientRect> {
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

    #[test]
    fn z_order_only_placement_never_changes_geometry() {
        let (_, _, _, _, flags) = placement_values(PlacementGeometry::Preserve, false);
        assert_ne!(flags.0 & SWP_NOMOVE.0, 0);
        assert_ne!(flags.0 & SWP_NOSIZE.0, 0);
        assert_eq!(flags.0 & SWP_NOZORDER.0, 0);
    }

    #[test]
    fn fallback_moves_once_without_changing_z_order() {
        let rect = (10, 20, 300, 200);
        let (x, y, width, height, flags) = placement_values(PlacementGeometry::Set(rect), true);
        assert_eq!((x, y, width, height), rect);
        assert_eq!(flags.0 & SWP_NOMOVE.0, 0);
        assert_eq!(flags.0 & SWP_NOSIZE.0, 0);
        assert_ne!(flags.0 & SWP_NOZORDER.0, 0);
        assert_ne!(flags.0 & SWP_SHOWWINDOW.0, 0);
    }
}
