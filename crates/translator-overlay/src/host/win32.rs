//! Overlay ownership, client geometry, and Z-order placement.
//!
//! Captions own the capture target and use capture/DWM client metrics so boxes
//! stay aligned with OCR frames. The region picker stays unowned and inserts
//! above the target (topmost while the target is foreground); pure moves update
//! geometry with `SetWindowPos` only. If UIPI denies insert-after, the overlay
//! falls back to `HWND_TOPMOST` while the target is foreground.

use tracing::warn;
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::{
        GA_ROOT, GA_ROOTOWNER, GW_HWNDPREV, GW_OWNER, GWL_EXSTYLE, GWLP_HWNDPARENT, GetAncestor, GetClientRect, GetForegroundWindow,
        GetWindow, GetWindowLongPtrW, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        WINDOW_EX_STYLE, WS_EX_TOPMOST,
    },
};

pub(crate) type ClientRect = (i32, i32, i32, i32);

/// Whether the overlay should be owned by the capture target.
///
/// Owned captions ride the target's Z-order group (`SWP_NOOWNERZORDER`).
/// The picker must stay unowned so insert-above + hit-testing work. While the
/// target is foreground, an unowned picker sits in the topmost band so the
/// newly activated target cannot cover it.
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

fn target_is_foreground(target: HWND) -> bool {
    if target.is_invalid() {
        return false;
    }
    let fg = unsafe { GetForegroundWindow() };
    if fg.is_invalid() {
        return false;
    }
    if fg == target {
        return true;
    }
    let root = unsafe { GetAncestor(fg, GA_ROOT) };
    if !root.is_invalid() && root == target {
        return true;
    }
    let owner_root = unsafe { GetAncestor(fg, GA_ROOTOWNER) };
    !owner_root.is_invalid() && owner_root == target
}

fn overlay_wants_topmost(ownership: OverlayOwnership, target_topmost: bool, target_foreground: bool) -> bool {
    target_topmost || (ownership == OverlayOwnership::Unowned && target_foreground)
}

fn choose_z_order_anchor(overlay: isize, above_target: Option<isize>, target_topmost: bool) -> ZOrderAnchor {
    match above_target {
        Some(hwnd) if hwnd == overlay => ZOrderAnchor::Preserve,
        Some(hwnd) => ZOrderAnchor::After(hwnd),
        None if target_topmost => ZOrderAnchor::Topmost,
        None => ZOrderAnchor::Top,
    }
}

fn set_window_pos(overlay: HWND, insert_after: Option<HWND>, no_owner_zorder: bool) -> windows::core::Result<()> {
    let mut flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW;
    if no_owner_zorder {
        flags |= SWP_NOOWNERZORDER;
    }
    unsafe { SetWindowPos(overlay, insert_after, 0, 0, 0, 0, flags) }
}

fn set_topmost_band(overlay: HWND, want_topmost: bool) -> windows::core::Result<()> {
    if window_is_topmost(overlay) == want_topmost {
        let _ = unsafe { ShowWindow(overlay, SW_SHOWNOACTIVATE) };
        return Ok(());
    }
    let band = if want_topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
    unsafe { SetWindowPos(overlay, Some(band), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW) }
}

fn fallback_to_topmost(overlay: HWND, force_topmost: &mut bool, error: windows::core::Error) -> windows::core::Result<()> {
    warn!(error = %error, "overlay Z-order update failed; falling back to HWND_TOPMOST");
    *force_topmost = true;
    set_overlay_owner(overlay, None);
    set_topmost_band(overlay, true)
}

/// Move the overlay to a screen position without a Z-order or ULW pass.
pub(crate) fn move_overlay_position(overlay: HWND, x: i32, y: i32) -> windows::core::Result<()> {
    if overlay.is_invalid() {
        return Ok(());
    }
    unsafe { SetWindowPos(overlay, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER) }
}

/// Whether `place_overlay_above_target` will `SetWindowPos` (needs a prior ULW).
pub(crate) fn overlay_needs_restack(overlay: HWND, target: HWND, ownership: OverlayOwnership, force_topmost: bool) -> bool {
    let target_topmost = window_is_topmost(target);
    let ownership = if force_topmost { OverlayOwnership::Unowned } else { ownership };
    let want_topmost = overlay_wants_topmost(ownership, target_topmost, target_is_foreground(target));
    if window_is_topmost(overlay) != want_topmost {
        return true;
    }
    if force_topmost || (want_topmost && !target_topmost) {
        return false;
    }
    let above_target = unsafe { GetWindow(target, GW_HWNDPREV) }.ok().map(|hwnd| hwnd.0 as isize);
    !matches!(choose_z_order_anchor(overlay.0 as isize, above_target, target_topmost), ZOrderAnchor::Preserve)
}

/// Keep `overlay` immediately above `target` without raising a normal target globally.
///
/// If insert-after / band sync is denied (UIPI), latch `force_topmost` and sit in
/// the topmost band while the target is foreground.
pub(crate) fn place_overlay_above_target(
    overlay: HWND,
    target: HWND,
    ownership: OverlayOwnership,
    force_topmost: &mut bool,
) -> windows::core::Result<()> {
    if *force_topmost {
        set_overlay_owner(overlay, None);
        let topmost = overlay_wants_topmost(OverlayOwnership::Unowned, window_is_topmost(target), target_is_foreground(target));
        return set_topmost_band(overlay, topmost);
    }

    match ownership {
        OverlayOwnership::OwnedByTarget => set_overlay_owner(overlay, Some(target)),
        OverlayOwnership::Unowned => set_overlay_owner(overlay, None),
    }
    let no_owner_zorder = ownership == OverlayOwnership::OwnedByTarget;
    let target_topmost = window_is_topmost(target);
    let want_topmost = overlay_wants_topmost(ownership, target_topmost, target_is_foreground(target));

    if let Err(e) = set_topmost_band(overlay, want_topmost) {
        return fallback_to_topmost(overlay, force_topmost, e);
    }

    // A topmost picker above a normal target must not insert-after a non-topmost
    // predecessor — that `SetWindowPos` drops it out of the topmost band.
    if want_topmost && !target_topmost {
        return Ok(());
    }

    // Re-read after synchronizing the topmost band: that operation itself may
    // have changed which window is immediately above the target.
    let above_target = unsafe { GetWindow(target, GW_HWNDPREV) }.ok().map(|hwnd| hwnd.0 as isize);
    match choose_z_order_anchor(overlay.0 as isize, above_target, target_topmost) {
        ZOrderAnchor::Preserve => {
            let _ = unsafe { ShowWindow(overlay, SW_SHOWNOACTIVATE) };
            Ok(())
        }
        ZOrderAnchor::After(hwnd) => set_window_pos(overlay, Some(HWND(hwnd as *mut _)), no_owner_zorder),
        ZOrderAnchor::Top => set_window_pos(overlay, Some(HWND_TOP), no_owner_zorder),
        ZOrderAnchor::Topmost => set_window_pos(overlay, Some(HWND_TOPMOST), no_owner_zorder),
    }
    .or_else(|e| fallback_to_topmost(overlay, force_topmost, e))
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
    fn unowned_picker_is_topmost_only_while_the_target_is_foreground() {
        assert!(overlay_wants_topmost(OverlayOwnership::Unowned, false, true));
        assert!(!overlay_wants_topmost(OverlayOwnership::Unowned, false, false));
        assert!(overlay_wants_topmost(OverlayOwnership::Unowned, true, false));
    }

    #[test]
    fn owned_captions_follow_the_target_topmost_band_only() {
        assert!(!overlay_wants_topmost(OverlayOwnership::OwnedByTarget, false, true));
        assert!(overlay_wants_topmost(OverlayOwnership::OwnedByTarget, true, false));
    }
}
