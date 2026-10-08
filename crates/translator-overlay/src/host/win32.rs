//! Overlay ownership, client geometry, and Z-order placement.
//!
//! The caption overlay is owned by the capture target and uses the capture's DWM
//! client metrics, so boxes stay aligned with OCR frames. The region picker stays
//! unowned and inserts itself above the target, and it is topmost only while the
//! target is in the foreground. If User Interface Privilege Isolation (UIPI) denies
//! the insert-after, the overlay latches `HWND_TOPMOST` while the target is in the
//! foreground. When the target loses focus, the overlay leaves that band and moves
//! back next to the target, either directly or by climbing up from `HWND_BOTTOM`.
//! That way a denied insert-after on the new foreground window cannot leave the
//! overlay covering other windows.

use tracing::{debug, warn};
use windows::Win32::{
    Foundation::{HWND, LPARAM, POINT, RECT},
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
/// Owned captions move with the target's Z-order group (`SWP_NOOWNERZORDER`).
/// The picker stays unowned so inserting above the target and hit-testing work.
/// While the target is in the foreground, the unowned picker sits in the topmost
/// band so the newly activated target cannot cover it.
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

/// Focus and raise `target`. Restacking the overlay is left to `place_overlay_above_target`.
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
        // Hidden but already in the right band, as after a minimize and restore. Show it without restacking.
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
    // Do not move to HWND_NOTOPMOST before parking. That lands at the top of the normal band.
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

/// Move next to `target` without an insert-after on the foreground window, which UIPI may protect.
///
/// `HWND_NOTOPMOST` alone lands at the top of the normal band and covers the
/// foreground window. Sitting above the target, or climbing up from `HWND_BOTTOM`,
/// anchors on the target's predecessor instead. If that predecessor is the
/// protected foreground window, the slot between them cannot be reached. Then the
/// overlay stays covering instead of being buried under the target.
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
    // The picker must stay unowned. Giving it a foreign owner HWND hides this layered popup.
    set_overlay_owner(overlay, None);
    let fg = foreground_top_level();
    let fg_ok = !fg.is_invalid() && fg != overlay && fg != target;

    // When the foreground window allows insert-after, leave the band and tuck under it
    // in one call. UIPI often denies this with ACCESS_DENIED, and then the code below
    // drops to NOTOPMOST and moves next to the target.
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

/// Move the overlay to a screen position without a Z-order or `UpdateLayeredWindow` pass.
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

/// Whether `place_overlay_above_target` will call `SetWindowPos`, which needs a prior `UpdateLayeredWindow`.
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
/// If UIPI denies the insert-after or the band change, latch `force_topmost` and sit
/// in the topmost band while the target is in the foreground. When the target loses
/// focus, leave that band and move next to the target without an insert-after on the
/// new foreground window.
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

    // Park before any HWND_NOTOPMOST call, which by itself covers the new foreground window.
    if !want_topmost && ownership == OverlayOwnership::Unowned {
        return park_unfocused(overlay, target);
    }

    if let Err(e) = set_topmost_band(overlay, want_topmost) {
        return fallback_to_topmost(overlay, target, force_topmost, e);
    }

    // A topmost picker above a normal target must not insert-after a non-topmost
    // predecessor. That `SetWindowPos` would drop it out of the topmost band.
    if want_topmost && !target_topmost {
        return Ok(());
    }

    // Read the predecessor again after the band change, which may have changed
    // the window right above the target.
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
/// Uses `GetClientRect` and `ClientToScreen` so the geometry stays current while the
/// target is dragged. The capture's DWM metrics can briefly fail or lag during a move.
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

/// Signed x and y from a mouse-message `LPARAM`, like `GET_X_LPARAM` and `GET_Y_LPARAM`.
pub(crate) fn lparam_point(lparam: LPARAM) -> (i32, i32) {
    let packed = lparam.0 as u32;
    ((packed & 0xFFFF) as i16 as i32, (packed >> 16) as i16 as i32)
}

/// cargo test runs tests on many threads, and USER32 window state is process-wide.
#[cfg(test)]
pub(crate) fn lock_hwnd_tests() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // A failed smoke test poisons the lock; later tests still need exclusive USER32 state.
    LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_order_anchor_and_topmost_band() {
        assert_eq!(choose_z_order_anchor(20, Some(20), false), ZOrderAnchor::Preserve);
        assert_eq!(choose_z_order_anchor(20, Some(30), false), ZOrderAnchor::After(30));
        assert_eq!(choose_z_order_anchor(20, None, false), ZOrderAnchor::Top);
        assert_eq!(choose_z_order_anchor(20, None, true), ZOrderAnchor::Topmost);
        assert!(overlay_wants_topmost(OverlayOwnership::Unowned, false, true));
        assert!(!overlay_wants_topmost(OverlayOwnership::Unowned, false, false));
        assert!(overlay_wants_topmost(OverlayOwnership::Unowned, true, false));
        assert!(!overlay_wants_topmost(OverlayOwnership::OwnedByTarget, false, true));
        assert!(overlay_wants_topmost(OverlayOwnership::OwnedByTarget, true, false));
    }

    #[test]
    fn unowned_unfocus_does_not_cover_the_new_foreground() -> Result<(), String> {
        for (start_force_topmost, via_fallback) in [(true, false), (false, false), (false, true)] {
            z_order_hwnd_smoke(start_force_topmost, via_fallback)
                .map_err(|e| format!("force={start_force_topmost} fallback={via_fallback}: {e}"))?;
        }
        Ok(())
    }

    struct ZOrderWindows {
        target: HWND,
        overlay: HWND,
        other: HWND,
        _lock: std::sync::MutexGuard<'static, ()>,
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

        let lock = lock_hwnd_tests();
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
        Ok(ZOrderWindows {
            target,
            overlay,
            other,
            _lock: lock,
        })
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

        // A hide like the one from minimize and restore must not stick while the target is focused.
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

        // Parking must not lose topmost for good. Focusing the target again restores the band.
        refocus_target_restores_topmost(&w, &mut force_topmost)?;

        let _ = unsafe { SetForegroundWindow(w.other) };
        place_unowned(w.overlay, w.target, &mut force_topmost, "re-park after refocus")?;
        assert_parked_beside_target(w.overlay, w.target, w.other).map_err(|e| format!("re-park: {e}"))?;

        let _ = unsafe { SetWindowPos(w.overlay, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
        place_unowned(w.overlay, w.target, &mut force_topmost, "place after bury")?;
        assert_parked_beside_target(w.overlay, w.target, w.other).map_err(|e| format!("after HWND_BOTTOM climb: {e}"))?;

        refocus_target_restores_topmost(&w, &mut force_topmost).map_err(|e| format!("refocus after bury climb: {e}"))
    }

    /// Simulates the UIPI path. The insert-after on the foreground window fails, and
    /// HWND_NOTOPMOST alone lands at the top of the normal band, covering the new foreground window.
    #[test]
    fn park_pulls_beside_target_after_notopmost_covers_foreground() -> Result<(), String> {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

        let w = create_z_order_windows()?;
        let _ = unsafe { SetForegroundWindow(w.target) };
        let mut force_topmost = false;
        place_unowned(w.overlay, w.target, &mut force_topmost, "place while focused")?;
        assert_visible_topmost(w.overlay, w.target, "initial focus")?;

        let _ = unsafe { SetForegroundWindow(w.other) };
        // What UIPI leaves behind is a band change with no tuck below the foreground window.
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
        // Regression test. Parking by hiding the window used to lose topmost on the next focus.
        refocus_target_restores_topmost(&w, &mut force_topmost)
    }

    /// When the new foreground window is right above the target, parking must not bury
    /// the overlay under the target, which an earlier Hide and HWND_BOTTOM attempt did.
    #[test]
    fn park_with_fg_as_target_predecessor_stays_above_target() -> Result<(), String> {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

        let w = create_z_order_windows()?;
        let _ = unsafe { SetForegroundWindow(w.target) };
        let mut force_topmost = false;
        place_unowned(w.overlay, w.target, &mut force_topmost, "place while focused")?;
        assert_visible_topmost(w.overlay, w.target, "initial focus")?;

        // Once `other` is in the foreground, it must be the target's predecessor.
        // SetForegroundWindow raises `other`, and then the target is inserted right
        // under it so other desktop windows on CI cannot sit between them.
        let _ = unsafe { SetForegroundWindow(w.other) };
        let _ = unsafe { SetWindowPos(w.target, Some(w.other), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
        if target_is_foreground(w.other) && predecessor_hwnd(w.target) != Some(w.other) {
            return Err("failed to place FG immediately above target".into());
        }

        place_unowned(w.overlay, w.target, &mut force_topmost, "park with FG as predecessor")?;
        assert_parked_beside_target(w.overlay, w.target, w.other)?;
        refocus_target_restores_topmost(&w, &mut force_topmost)
    }

    #[test]
    fn raise_target_window_steals_foreground_from_another_window() -> Result<(), String> {
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
