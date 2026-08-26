//! Overlay ownership, client geometry, and Z-order placement.
//!
//! Captions own the capture target and use capture/DWM client metrics so boxes
//! stay aligned with OCR frames. The region picker stays unowned and inserts
//! above the target (topmost only while the target is foreground). If UIPI
//! denies insert-after, the overlay latches `HWND_TOPMOST` while the target is
//! foreground. On focus loss it leaves that band and pulls beside the target
//! (`sit` / `HWND_BOTTOM` climb) so a UIPI-denied insert-after on the new FG
//! cannot leave it covering other windows.

use tracing::{debug, warn};
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    System::Threading::{AttachThreadInput, GetCurrentThreadId},
    UI::WindowsAndMessaging::{
        BringWindowToTop, GA_ROOT, GA_ROOTOWNER, GW_HWNDNEXT, GW_HWNDPREV, GW_OWNER, GWL_EXSTYLE, GWLP_HWNDPARENT, GetAncestor,
        GetClientRect, GetForegroundWindow, GetWindow, GetWindowLongPtrW, GetWindowThreadProcessId, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOP,
        HWND_TOPMOST, IsWindowVisible, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
        SWP_NOZORDER, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WS_EX_TOPMOST,
    },
};

pub(crate) type ClientRect = (i32, i32, i32, i32);

/// Whether the overlay should be owned by the capture target.
///
/// Owned captions ride the target's Z-order group (`SWP_NOOWNERZORDER`).
/// The picker stays unowned so insert-above + hit-testing work. While the
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

pub(crate) fn window_is_topmost(hwnd: HWND) -> bool {
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
    if let Err(e) =
        unsafe { SetWindowPos(overlay, None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED) }
    {
        warn!(error = %e, "SetWindowPos(owner FRAMECHANGED) failed");
    }
}

pub(crate) fn target_is_foreground(target: HWND) -> bool {
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

/// Focus and raise `target`. Overlay restack stays in `place_overlay_above_target`.
///
/// Attaches to the current foreground thread so `SetForegroundWindow` can succeed
/// from the `WS_EX_NOACTIVATE` overlay thread.
pub(crate) fn raise_target_window(target: HWND) {
    if target.is_invalid() || target_is_foreground(target) {
        return;
    }
    let this_tid = unsafe { GetCurrentThreadId() };
    let fg_tid = unsafe { GetWindowThreadProcessId(GetForegroundWindow(), None) };
    let attached = fg_tid != 0 && fg_tid != this_tid && unsafe { AttachThreadInput(this_tid, fg_tid, true) }.as_bool();
    let _ = unsafe { SetForegroundWindow(target) };
    let _ = unsafe { BringWindowToTop(target) };
    if attached {
        let _ = unsafe { AttachThreadInput(this_tid, fg_tid, false) };
    }
}

pub(crate) fn overlay_wants_topmost(ownership: OverlayOwnership, target_topmost: bool, target_foreground: bool) -> bool {
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

fn z_order_is_above(upper: HWND, lower: HWND) -> bool {
    if upper.is_invalid() || lower.is_invalid() || upper == lower {
        return false;
    }
    let mut current = unsafe { GetWindow(upper, GW_HWNDNEXT) }.ok().unwrap_or_default();
    while !current.is_invalid() {
        if current == lower {
            return true;
        }
        current = unsafe { GetWindow(current, GW_HWNDNEXT) }.ok().unwrap_or_default();
    }
    false
}

fn predecessor_hwnd(target: HWND) -> Option<HWND> {
    unsafe { GetWindow(target, GW_HWNDPREV) }.ok().filter(|hwnd| !hwnd.is_invalid())
}

fn set_window_pos(overlay: HWND, insert_after: Option<HWND>, no_owner_zorder: bool) -> windows::core::Result<()> {
    let mut flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    if !unsafe { IsWindowVisible(overlay) }.as_bool() {
        flags |= SWP_SHOWWINDOW;
    }
    if no_owner_zorder {
        flags |= SWP_NOOWNERZORDER;
    }
    let result = unsafe { SetWindowPos(overlay, insert_after, 0, 0, 0, 0, flags) };
    if let Err(e) = &result {
        warn!(error = %e, insert_after = insert_after.map(|h| h.0 as isize).unwrap_or(0), "SetWindowPos failed");
    }
    result
}

fn set_topmost_band(overlay: HWND, want_topmost: bool) -> windows::core::Result<()> {
    let visible = unsafe { IsWindowVisible(overlay) }.as_bool();
    if window_is_topmost(overlay) == want_topmost {
        if visible {
            return Ok(());
        }
        // Hidden but already in-band (minimize → restore): show without restacking.
        return unsafe {
            SetWindowPos(overlay, None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW)
        };
    }
    let band = if want_topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
    set_window_pos(overlay, Some(band), false)
}

fn fallback_to_topmost(overlay: HWND, target: HWND, force_topmost: &mut bool, error: windows::core::Error) -> windows::core::Result<()> {
    warn!(
        error = %error,
        target_fg = target_is_foreground(target),
        overlay_topmost = window_is_topmost(overlay),
        "overlay Z-order update failed; latching HWND_TOPMOST fallback"
    );
    *force_topmost = true;
    place_force_topmost(overlay, target)
}

fn place_force_topmost(overlay: HWND, target: HWND) -> windows::core::Result<()> {
    set_overlay_owner(overlay, None);
    let target_fg = target_is_foreground(target);
    let want_topmost = overlay_wants_topmost(OverlayOwnership::Unowned, window_is_topmost(target), target_fg);
    // Do not HWND_NOTOPMOST before park — that parks at the top of the normal band.
    if want_topmost {
        set_topmost_band(overlay, true)
    } else {
        park_unfocused(overlay, target)
    }
}

fn foreground_top_level() -> HWND {
    let fg = unsafe { GetForegroundWindow() };
    if fg.is_invalid() {
        return fg;
    }
    let root = unsafe { GetAncestor(fg, GA_ROOT) };
    if root.is_invalid() { fg } else { root }
}

fn sit_above_target(overlay: HWND, target: HWND) -> windows::core::Result<()> {
    match predecessor_hwnd(target) {
        Some(prev) if prev == overlay => Ok(()),
        Some(prev) => set_window_pos(overlay, Some(prev), false),
        None => set_window_pos(overlay, Some(HWND_TOP), false),
    }
}

fn covering_foreground(overlay: HWND, fg: HWND, target: HWND) -> bool {
    !fg.is_invalid() && z_order_is_above(overlay, fg) && z_order_is_above(fg, target)
}

/// Pull beside `target` without insert-after on a possibly UIPI-protected FG hwnd.
///
/// `HWND_NOTOPMOST` alone parks at the top of the normal band and covers the FG.
/// Sitting above the target (or climbing from `HWND_BOTTOM`) uses the target's
/// predecessor instead. If that predecessor *is* the protected FG, the slot
/// between them is unreachable — leave covering rather than burying under target.
fn pull_beside_target(overlay: HWND, target: HWND, fg: HWND) {
    let _ = sit_above_target(overlay, target);
    if z_order_is_above(overlay, target) && !covering_foreground(overlay, fg, target) {
        return;
    }
    if predecessor_hwnd(target) == Some(fg) {
        warn!(
            overlay = overlay.0 as isize,
            fg = fg.0 as isize,
            target = target.0 as isize,
            "UIPI blocks the slot between foreground and target; not burying under target"
        );
        return;
    }
    let _ = set_window_pos(overlay, Some(HWND_BOTTOM), false);
    let _ = sit_above_target(overlay, target);
}

fn park_unfocused(overlay: HWND, target: HWND) -> windows::core::Result<()> {
    // Picker must stay unowned — owning a foreign HWND hides this layered popup.
    set_overlay_owner(overlay, None);
    let fg = foreground_top_level();
    let fg_ok = !fg.is_invalid() && fg != overlay && fg != target;

    // One-shot leave+tuck when FG allows insert-after. UIPI often denies this
    // (ACCESS_DENIED); fall through to NOTOPMOST + pull beside target.
    if fg_ok {
        let _ = set_window_pos(overlay, Some(fg), false);
    }
    if window_is_topmost(overlay) {
        let _ = set_topmost_band(overlay, false);
    }

    if !z_order_is_above(overlay, target) || (fg_ok && covering_foreground(overlay, fg, target)) {
        pull_beside_target(overlay, target, if fg_ok { fg } else { HWND::default() });
    }

    debug!(
        overlay = overlay.0 as isize,
        fg = fg.0 as isize,
        target = target.0 as isize,
        visible = unsafe { IsWindowVisible(overlay) }.as_bool(),
        above_target = z_order_is_above(overlay, target),
        above_fg = fg_ok && z_order_is_above(overlay, fg),
        topmost = window_is_topmost(overlay),
        "overlay unfocused park"
    );
    Ok(())
}

/// Move the overlay to a screen position without a Z-order or ULW pass.
pub(crate) fn move_overlay_position(overlay: HWND, x: i32, y: i32) -> windows::core::Result<()> {
    if overlay.is_invalid() {
        return Ok(());
    }
    let result = unsafe { SetWindowPos(overlay, None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER) };
    if let Err(e) = &result {
        warn!(error = %e, x, y, "SetWindowPos(move) failed");
    }
    result
}

/// Whether `place_overlay_above_target` will `SetWindowPos` (needs a prior ULW).
pub(crate) fn overlay_needs_restack(overlay: HWND, target: HWND, ownership: OverlayOwnership, force_topmost: bool) -> bool {
    let target_topmost = window_is_topmost(target);
    let ownership = if force_topmost { OverlayOwnership::Unowned } else { ownership };
    let want_topmost = overlay_wants_topmost(ownership, target_topmost, target_is_foreground(target));
    if window_is_topmost(overlay) != want_topmost {
        return true;
    }
    if want_topmost && (force_topmost || !target_topmost) {
        return false;
    }
    if !want_topmost {
        let fg = foreground_top_level();
        return !z_order_is_above(overlay, target) || covering_foreground(overlay, fg, target);
    }
    let above_target = predecessor_hwnd(target).map(|hwnd| hwnd.0 as isize);
    !matches!(choose_z_order_anchor(overlay.0 as isize, above_target, target_topmost), ZOrderAnchor::Preserve)
}

/// Keep `overlay` immediately above `target` without raising a normal target globally.
///
/// If insert-after / band sync is denied (UIPI), latch `force_topmost` and sit in
/// the topmost band while the target is foreground. On focus loss, drop that band
/// and pull beside the target without requiring insert-after on the new FG.
pub(crate) fn place_overlay_above_target(
    overlay: HWND,
    target: HWND,
    ownership: OverlayOwnership,
    force_topmost: &mut bool,
) -> windows::core::Result<()> {
    if *force_topmost {
        return place_force_topmost(overlay, target);
    }

    match ownership {
        OverlayOwnership::OwnedByTarget => set_overlay_owner(overlay, Some(target)),
        OverlayOwnership::Unowned => set_overlay_owner(overlay, None),
    }
    let no_owner_zorder = ownership == OverlayOwnership::OwnedByTarget;
    let target_topmost = window_is_topmost(target);
    let target_fg = target_is_foreground(target);
    let want_topmost = overlay_wants_topmost(ownership, target_topmost, target_fg);
    debug!(?ownership, want_topmost, target_fg, overlay_topmost = window_is_topmost(overlay), "overlay z-order place");

    // Park before HWND_NOTOPMOST — that band call covers the new FG on its own.
    if !want_topmost && ownership == OverlayOwnership::Unowned {
        return park_unfocused(overlay, target);
    }

    if let Err(e) = set_topmost_band(overlay, want_topmost) {
        return fallback_to_topmost(overlay, target, force_topmost, e);
    }

    // A topmost picker above a normal target must not insert-after a non-topmost
    // predecessor — that `SetWindowPos` drops it out of the topmost band.
    if want_topmost && !target_topmost {
        return Ok(());
    }

    // Re-read after synchronizing the topmost band: that operation itself may
    // have changed which window is immediately above the target.
    let above_target = predecessor_hwnd(target).map(|hwnd| hwnd.0 as isize);
    match choose_z_order_anchor(overlay.0 as isize, above_target, target_topmost) {
        ZOrderAnchor::Preserve => {
            let _ = unsafe { ShowWindow(overlay, SW_SHOWNOACTIVATE) };
            Ok(())
        }
        ZOrderAnchor::After(hwnd) => set_window_pos(overlay, Some(HWND(hwnd as *mut _)), no_owner_zorder),
        ZOrderAnchor::Top => set_window_pos(overlay, Some(HWND_TOP), no_owner_zorder),
        ZOrderAnchor::Topmost => set_window_pos(overlay, Some(HWND_TOPMOST), no_owner_zorder),
    }
    .or_else(|e| fallback_to_topmost(overlay, target, force_topmost, e))
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

    #[test]
    fn unowned_unfocus_does_not_cover_the_new_foreground() {
        for (start_force_topmost, via_fallback) in [(true, false), (false, false), (false, true)] {
            if let Err(e) = z_order_hwnd_smoke(start_force_topmost, via_fallback) {
                panic!("force={start_force_topmost} fallback={via_fallback}: {e}");
            }
        }
    }

    #[test]
    fn park_pulls_beside_target_after_notopmost_covers_foreground() {
        // Simulates the UIPI path: insert-after FG fails, HWND_NOTOPMOST alone
        // parks at the top of the normal band and covers the new FG.
        if let Err(e) = park_after_notopmost_cover_smoke() {
            panic!("{e}");
        }
    }

    #[test]
    fn park_with_fg_as_target_predecessor_stays_above_target() {
        // When the new FG is immediately above the target, park must not bury
        // the overlay under the target (the bad Hide / HWND_BOTTOM "fix").
        if let Err(e) = park_when_fg_is_predecessor_smoke() {
            panic!("{e}");
        }
    }

    #[test]
    fn raise_target_window_steals_foreground_from_another_window() {
        if let Err(e) = raise_target_hwnd_smoke() {
            panic!("{e}");
        }
    }

    struct ZOrderWindows {
        target: HWND,
        overlay: HWND,
        other: HWND,
    }

    impl Drop for ZOrderWindows {
        fn drop(&mut self) {
            use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
            for hwnd in [self.other, self.overlay, self.target] {
                let _ = unsafe { DestroyWindow(hwnd) };
            }
        }
    }

    fn create_z_order_windows() -> Result<ZOrderWindows, String> {
        use windows::{
            Win32::{
                System::LibraryLoader::GetModuleHandleW,
                UI::WindowsAndMessaging::{CreateWindowExW, SW_SHOW, ShowWindow, WS_OVERLAPPEDWINDOW},
            },
            core::w,
        };

        let hinstance = unsafe { GetModuleHandleW(None) }.map_err(|e| format!("GetModuleHandleW: {e}"))?;
        let create = |title, x, y| unsafe {
            CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                title,
                WS_OVERLAPPEDWINDOW,
                x,
                y,
                240,
                180,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        };
        let target = create(w!("z-order-target"), 40, 40).map_err(|e| format!("CreateWindowExW(target): {e}"))?;
        let overlay = create(w!("z-order-overlay"), 40, 40).map_err(|e| format!("CreateWindowExW(overlay): {e}"))?;
        let other = create(w!("z-order-other"), 320, 40).map_err(|e| format!("CreateWindowExW(other): {e}"))?;
        let _ = unsafe { ShowWindow(target, SW_SHOW) };
        let _ = unsafe { ShowWindow(overlay, SW_SHOW) };
        let _ = unsafe { ShowWindow(other, SW_SHOW) };
        Ok(ZOrderWindows { target, overlay, other })
    }

    fn assert_visible_topmost(overlay: HWND, target: HWND, why: &str) -> Result<(), String> {
        use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
        if !target_is_foreground(target) {
            return Ok(());
        }
        if !unsafe { IsWindowVisible(overlay) }.as_bool() {
            return Err(format!("{why}: overlay hidden while target is foreground"));
        }
        if !window_is_topmost(overlay) {
            return Err(format!("{why}: overlay not WS_EX_TOPMOST while target is foreground"));
        }
        Ok(())
    }

    fn assert_parked_beside_target(overlay: HWND, target: HWND, other: HWND) -> Result<(), String> {
        use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
        if !target_is_foreground(other) {
            return Ok(());
        }
        if window_is_topmost(overlay) || z_order_is_above(overlay, other) {
            return Err("overlay covers the new foreground window".into());
        }
        if !unsafe { IsWindowVisible(overlay) }.as_bool() {
            return Err("overlay is hidden after park".into());
        }
        if !z_order_is_above(overlay, target) {
            return Err("overlay is behind the target after park".into());
        }
        Ok(())
    }

    fn place_unowned(overlay: HWND, target: HWND, force_topmost: &mut bool, why: &str) -> Result<(), String> {
        place_overlay_above_target(overlay, target, OverlayOwnership::Unowned, force_topmost).map_err(|e| format!("{why}: {e}"))
    }

    fn refocus_target_restores_topmost(w: &ZOrderWindows, force_topmost: &mut bool) -> Result<(), String> {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        let _ = unsafe { SetForegroundWindow(w.target) };
        place_unowned(w.overlay, w.target, force_topmost, "place after refocus")?;
        assert_visible_topmost(w.overlay, w.target, "after refocus")
    }

    fn z_order_hwnd_smoke(start_force_topmost: bool, via_fallback: bool) -> Result<(), String> {
        use windows::Win32::{
            Foundation::E_ACCESSDENIED,
            UI::WindowsAndMessaging::{HWND_BOTTOM, SW_HIDE, SetForegroundWindow, ShowWindow},
        };

        let w = create_z_order_windows()?;
        let _ = unsafe { SetForegroundWindow(w.target) };

        let mut force_topmost = start_force_topmost;
        place_unowned(w.overlay, w.target, &mut force_topmost, "place while focused")?;
        assert_visible_topmost(w.overlay, w.target, "initial focus")?;

        // Minimize/restore style hide must not stick while target is focused.
        let _ = unsafe { ShowWindow(w.overlay, SW_HIDE) };
        place_unowned(w.overlay, w.target, &mut force_topmost, "place after hide")?;
        assert_visible_topmost(w.overlay, w.target, "after hide while focused")?;

        let _ = unsafe { SetForegroundWindow(w.other) };
        if via_fallback {
            fallback_to_topmost(w.overlay, w.target, &mut force_topmost, windows::core::Error::from_hresult(E_ACCESSDENIED))
                .map_err(|e| format!("fallback after focus loss: {e}"))?;
            if !force_topmost {
                return Err("fallback did not latch force_topmost".into());
            }
        } else {
            place_unowned(w.overlay, w.target, &mut force_topmost, "place after focus loss")?;
        }
        assert_parked_beside_target(w.overlay, w.target, w.other)?;

        // Park must not permanently lose topmost: refocus restores the band.
        refocus_target_restores_topmost(&w, &mut force_topmost)?;

        let _ = unsafe { SetForegroundWindow(w.other) };
        place_unowned(w.overlay, w.target, &mut force_topmost, "re-park after refocus")?;
        assert_parked_beside_target(w.overlay, w.target, w.other).map_err(|e| format!("re-park: {e}"))?;

        let _ = unsafe { SetWindowPos(w.overlay, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
        place_unowned(w.overlay, w.target, &mut force_topmost, "place after bury")?;
        assert_parked_beside_target(w.overlay, w.target, w.other).map_err(|e| format!("after HWND_BOTTOM climb: {e}"))?;

        refocus_target_restores_topmost(&w, &mut force_topmost).map_err(|e| format!("refocus after bury climb: {e}"))
    }

    fn park_after_notopmost_cover_smoke() -> Result<(), String> {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

        let w = create_z_order_windows()?;
        let _ = unsafe { SetForegroundWindow(w.target) };
        let mut force_topmost = false;
        place_unowned(w.overlay, w.target, &mut force_topmost, "place while focused")?;
        assert_visible_topmost(w.overlay, w.target, "initial focus")?;

        let _ = unsafe { SetForegroundWindow(w.other) };
        // UIPI aftermath: leave the topmost band without tucking below FG.
        set_topmost_band(w.overlay, false).map_err(|e| format!("HWND_NOTOPMOST: {e}"))?;
        if target_is_foreground(w.other) {
            if !covering_foreground(w.overlay, w.other, w.target) {
                return Err("expected HWND_NOTOPMOST alone to cover the new foreground".into());
            }
            if !overlay_needs_restack(w.overlay, w.target, OverlayOwnership::Unowned, force_topmost) {
                return Err("covering foreground should need restack".into());
            }
        }

        place_unowned(w.overlay, w.target, &mut force_topmost, "park after NOTOPMOST cover")?;
        assert_parked_beside_target(w.overlay, w.target, w.other)?;
        // Regression: Hide-based park lost topmost on the next focus.
        refocus_target_restores_topmost(&w, &mut force_topmost)
    }

    fn park_when_fg_is_predecessor_smoke() -> Result<(), String> {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

        let w = create_z_order_windows()?;
        let _ = unsafe { SetForegroundWindow(w.target) };
        let mut force_topmost = false;
        place_unowned(w.overlay, w.target, &mut force_topmost, "place while focused")?;
        assert_visible_topmost(w.overlay, w.target, "initial focus")?;

        // other precedes target → predecessor_hwnd(target) == other once other is FG.
        let _ = unsafe { SetWindowPos(w.target, Some(w.other), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
        let _ = unsafe { SetForegroundWindow(w.other) };
        if target_is_foreground(w.other) && predecessor_hwnd(w.target) != Some(w.other) {
            return Err("failed to place FG immediately above target".into());
        }

        place_unowned(w.overlay, w.target, &mut force_topmost, "park with FG as predecessor")?;
        assert_parked_beside_target(w.overlay, w.target, w.other)?;
        refocus_target_restores_topmost(&w, &mut force_topmost)
    }

    fn raise_target_hwnd_smoke() -> Result<(), String> {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

        let w = create_z_order_windows()?;
        let _ = unsafe { SetForegroundWindow(w.other) };
        let other_was_fg = target_is_foreground(w.other);
        raise_target_window(w.target);
        if other_was_fg && !target_is_foreground(w.target) {
            return Err("raise_target_window left the other window in the foreground".into());
        }
        let was_fg = target_is_foreground(w.target);
        raise_target_window(w.target);
        if was_fg && !target_is_foreground(w.target) {
            return Err("raise_target_window dropped focus from the already-foreground target".into());
        }
        Ok(())
    }
}
