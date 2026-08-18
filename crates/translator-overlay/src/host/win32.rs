//! Overlay ownership, client geometry, and Z-order placement.
//!
//! Captions own the capture target and use capture/DWM client metrics so boxes
//! stay aligned with OCR frames. The region picker stays unowned and inserts
//! above the target; pure moves update geometry with `SetWindowPos` only.

use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::{
        GW_HWNDPREV, GW_OWNER, GWL_EXSTYLE, GWLP_HWNDPARENT, GetClientRect, GetWindow, GetWindowLongPtrW, HWND_NOTOPMOST, HWND_TOP,
        HWND_TOPMOST, SET_WINDOW_POS_FLAGS, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
        SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WS_EX_TOPMOST,
    },
};

pub(crate) type ClientRect = (i32, i32, i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlacementGeometry {
    Preserve,
    Set(ClientRect),
}

/// Whether the overlay should be owned by the capture target.
///
/// Owned captions ride the target's Z-order group (`SWP_NOOWNERZORDER`).
/// The picker must stay unowned so insert-above + hit-testing work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayOwnership {
    OwnedByTarget,
    Unowned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZOrderAnchor {
    Preserve,
    After(isize),
    Top,
    Topmost,
}

fn window_is_topmost(hwnd: HWND) -> bool {
    if hwnd.is_invalid() {
        return false;
    }
    WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32).contains(WS_EX_TOPMOST)
}

pub(crate) fn overlay_owner(hwnd: HWND) -> HWND {
    if hwnd.is_invalid() {
        return HWND::default();
    }
    unsafe { GetWindow(hwnd, GW_OWNER) }.unwrap_or_default()
}

pub(crate) fn set_overlay_owner(overlay: HWND, owner: Option<HWND>) {
    if overlay.is_invalid() {
        return;
    }
    let desired = owner.filter(|h| !h.is_invalid()).unwrap_or_default();
    if overlay_owner(overlay) == desired {
        return;
    }
    unsafe { SetWindowLongPtrW(overlay, GWLP_HWNDPARENT, desired.0 as isize) };
    let _ = unsafe { SetWindowPos(overlay, None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED) };
}

fn choose_z_order_anchor(overlay: isize, above_target: Option<isize>, target_topmost: bool) -> ZOrderAnchor {
    match above_target {
        Some(hwnd) if hwnd == overlay => ZOrderAnchor::Preserve,
        Some(hwnd) => ZOrderAnchor::After(hwnd),
        None if target_topmost => ZOrderAnchor::Topmost,
        None => ZOrderAnchor::Top,
    }
}

fn placement_values(
    geometry: PlacementGeometry,
    preserve_z_order: bool,
    no_owner_zorder: bool,
) -> (i32, i32, i32, i32, SET_WINDOW_POS_FLAGS) {
    let (x, y, width, height, mut flags) = match geometry {
        PlacementGeometry::Preserve => (0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW),
        PlacementGeometry::Set((x, y, width, height)) => (x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW),
    };
    if preserve_z_order {
        flags |= SWP_NOZORDER;
    }
    if no_owner_zorder {
        flags |= SWP_NOOWNERZORDER;
    }
    (x, y, width, height, flags)
}

fn set_window_pos(
    overlay: HWND,
    insert_after: Option<HWND>,
    geometry: PlacementGeometry,
    preserve_z_order: bool,
    no_owner_zorder: bool,
) -> windows::core::Result<()> {
    let (x, y, width, height, flags) = placement_values(geometry, preserve_z_order, no_owner_zorder);
    unsafe { SetWindowPos(overlay, insert_after, x, y, width, height, flags) }
}

/// Keep `overlay` immediately above `target` without raising a normal target globally.
pub(crate) fn place_overlay_above_target(
    overlay: HWND,
    target: HWND,
    geometry: PlacementGeometry,
    ownership: OverlayOwnership,
) -> windows::core::Result<()> {
    match ownership {
        OverlayOwnership::OwnedByTarget => set_overlay_owner(overlay, Some(target)),
        OverlayOwnership::Unowned => set_overlay_owner(overlay, None),
    }
    let no_owner_zorder = ownership == OverlayOwnership::OwnedByTarget;

    let target_topmost = window_is_topmost(target);
    if window_is_topmost(overlay) != target_topmost {
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
        ZOrderAnchor::Preserve => set_window_pos(overlay, None, geometry, true, no_owner_zorder),
        ZOrderAnchor::After(hwnd) => set_window_pos(overlay, Some(HWND(hwnd as *mut _)), geometry, false, no_owner_zorder),
        ZOrderAnchor::Top => set_window_pos(overlay, Some(HWND_TOP), geometry, false, no_owner_zorder),
        ZOrderAnchor::Topmost => set_window_pos(overlay, Some(HWND_TOPMOST), geometry, false, no_owner_zorder),
    }
}

/// Live Win32 client rect for the region picker.
///
/// Uses `GetClientRect` + `ClientToScreen` so geometry stays current while the
/// target is dragged. Capture DWM metrics can briefly fail or lag mid-move.
pub(crate) fn live_client_screen_rect(target: HWND) -> Option<ClientRect> {
    if target.is_invalid() {
        return None;
    }
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
    fn placement_preserve_never_changes_geometry() {
        let (_, _, _, _, flags) = placement_values(PlacementGeometry::Preserve, false, false);
        assert_ne!(flags.0 & SWP_NOMOVE.0, 0);
        assert_ne!(flags.0 & SWP_NOSIZE.0, 0);
        assert_eq!(flags.0 & SWP_NOZORDER.0, 0);
    }

    #[test]
    fn owned_placement_sets_no_owner_zorder() {
        let (_, _, _, _, flags) = placement_values(PlacementGeometry::Set((1, 2, 3, 4)), false, true);
        assert_ne!(flags.0 & SWP_NOOWNERZORDER.0, 0);
    }

    #[test]
    fn fallback_moves_once_without_changing_z_order() {
        let rect = (10, 20, 300, 200);
        let (x, y, width, height, flags) = placement_values(PlacementGeometry::Set(rect), true, false);
        assert_eq!((x, y, width, height), rect);
        assert_eq!(flags.0 & SWP_NOMOVE.0, 0);
        assert_eq!(flags.0 & SWP_NOSIZE.0, 0);
        assert_ne!(flags.0 & SWP_NOZORDER.0, 0);
        assert_ne!(flags.0 & SWP_SHOWWINDOW.0, 0);
    }
}
