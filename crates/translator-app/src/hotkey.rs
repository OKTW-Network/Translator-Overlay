//! Global keyboard hook. Binding ignores the Win key and still records the other
//! key. A live shortcut is skipped entirely while Win is held.

use std::{
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc,
    },
    thread,
};

use parking_lot::Mutex;
use tracing::error;
use translator_core::{AppState, HotkeyAction, HotkeyChord, HotkeyConfig, is_modifier_vk, is_win_vk};
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT},
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GA_ROOT, GetAncestor, GetForegroundWindow, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_UP, MSG,
            PM_NOREMOVE, PeekMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL,
            WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

use crate::pipeline::{CmdTx, PipelineCommand, ping_ui};

const KEY_SLOTS: usize = 256;

pub(crate) struct HotkeySnap {
    pub hotkeys: HotkeyConfig,
    pub focus_hwnd: Option<isize>,
    pub can: [bool; 5],
    capturing: Option<HotkeyAction>,
}

pub(crate) struct HookInner {
    pub snap: HotkeySnap,
    listen_gen: u64,
    held: [bool; KEY_SLOTS],
    swallow: [bool; KEY_SLOTS],
}

pub(crate) struct KeyIn {
    pub vk: u16,
    pub down: bool,
    pub win: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Step {
    Pass,
    Swallow,
    Fire(HotkeyAction),
    Bind {
        action: HotkeyAction,
        chord: HotkeyChord,
        generation: u64,
    },
    CancelListen {
        generation: u64,
    },
}

impl HotkeySnap {
    fn idle() -> Self {
        Self {
            hotkeys: HotkeyConfig::default(),
            focus_hwnd: None,
            can: [false; 5],
            capturing: None,
        }
    }
}

impl HookInner {
    fn new() -> Self {
        Self {
            snap: HotkeySnap::idle(),
            listen_gen: 0,
            held: [false; KEY_SLOTS],
            swallow: [false; KEY_SLOTS],
        }
    }
}

pub(crate) fn snapshot(state: &AppState) -> HotkeySnap {
    let focus_hwnd = if state.auto_running {
        state.target_hwnd
    } else {
        state.selected_hwnd
    };
    let busy = state.capture_busy;
    let mut can = [false; 5];
    can[HotkeyAction::StartPause as usize] = !busy && (state.auto_running || state.selected_hwnd.is_some());
    can[HotkeyAction::Stop as usize] = !busy && state.auto_running;
    can[HotkeyAction::SelectRegion as usize] = state.region_select_active || focus_hwnd.is_some();
    can[HotkeyAction::Retry as usize] = state.auto_running && !state.translate_in_flight && !busy;
    can[HotkeyAction::ClearHistory as usize] = true;
    HotkeySnap {
        hotkeys: state.config.hotkeys.clone(),
        focus_hwnd,
        can,
        capturing: None,
    }
}

const CTRL_VKS: [u16; 3] = [0x11, 0xA2, 0xA3];
const ALT_VKS: [u16; 3] = [0x12, 0xA4, 0xA5];
const SHIFT_VKS: [u16; 3] = [0x10, 0xA0, 0xA1];
const WIN_VKS: [u16; 2] = [0x5B, 0x5C];

fn any_held(inner: &HookInner, vks: &[u16]) -> bool {
    vks.iter().any(|vk| inner.held[*vk as usize])
}

fn any_async(vks: &[u16]) -> bool {
    vks.iter().any(|vk| key_down(*vk))
}

/// `Ctrl+Shift+…` while a row is listening and at least one modifier is held.
///
/// Swallowed modifiers never update the system key state, so this also reads the hook's own held keys.
pub fn binding_preview() -> Option<String> {
    let async_ctrl = any_async(&CTRL_VKS);
    let async_alt = any_async(&ALT_VKS);
    let async_shift = any_async(&SHIFT_VKS);
    let inner = runtime().inner.try_lock()?;
    inner.snap.capturing?;
    let ctrl = async_ctrl || any_held(&inner, &CTRL_VKS);
    let alt = async_alt || any_held(&inner, &ALT_VKS);
    let shift = async_shift || any_held(&inner, &SHIFT_VKS);
    if !ctrl && !alt && !shift {
        return None;
    }
    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    parts.push("…");
    Some(parts.join("+"))
}

/// One keyboard event. `target_focused` is the capture / selected window.
pub(crate) fn step(inner: &mut HookInner, mut key: KeyIn, target_focused: bool) -> Step {
    if key.vk as usize >= KEY_SLOTS {
        return Step::Pass;
    }
    let vk = key.vk as usize;
    if !key.down {
        inner.held[vk] = false;
        let swallowed = inner.swallow[vk];
        inner.swallow[vk] = false;
        return if swallowed { Step::Swallow } else { Step::Pass };
    }
    let repeat = inner.held[vk];
    inner.held[vk] = true;
    if repeat {
        return if inner.swallow[vk] { Step::Swallow } else { Step::Pass };
    }
    // A swallowed modifier is not visible to GetAsyncKeyState. Count keys this hook already saw.
    key.ctrl |= any_held(inner, &CTRL_VKS);
    key.alt |= any_held(inner, &ALT_VKS);
    key.shift |= any_held(inner, &SHIFT_VKS);
    key.win |= any_held(inner, &WIN_VKS);

    let out = if inner.snap.capturing.is_some() {
        step_capture(inner, &key)
    } else {
        step_live(inner, &key, target_focused)
    };
    if !matches!(out, Step::Pass) {
        inner.swallow[vk] = true;
    }
    out
}

fn step_capture(inner: &mut HookInner, key: &KeyIn) -> Step {
    let Some(action) = inner.snap.capturing else {
        return Step::Pass;
    };
    if is_win_vk(key.vk) || is_modifier_vk(key.vk) {
        return Step::Swallow;
    }
    if key.vk == 0x1B {
        let generation = inner.listen_gen;
        inner.snap.capturing = None;
        return Step::CancelListen { generation };
    }
    let Some(chord) = HotkeyChord::from_parts(key.ctrl, key.alt, key.shift, key.vk) else {
        return Step::Swallow;
    };
    let generation = inner.listen_gen;
    inner.snap.capturing = None;
    Step::Bind { action, chord, generation }
}

fn step_live(inner: &HookInner, key: &KeyIn, target_focused: bool) -> Step {
    if !inner.snap.hotkeys.enabled || key.win || is_win_vk(key.vk) || is_modifier_vk(key.vk) {
        return Step::Pass;
    }
    let Some(chord) = HotkeyChord::from_parts(key.ctrl, key.alt, key.shift, key.vk) else {
        return Step::Pass;
    };
    let Some(action) = inner.snap.hotkeys.find(chord) else {
        return Step::Pass;
    };
    if !inner.snap.can[action as usize] {
        return Step::Pass;
    }
    if inner.snap.hotkeys.only_when_target_focused && !target_focused {
        return Step::Pass;
    }
    Step::Fire(action)
}

pub enum Captured {
    Bind {
        action: HotkeyAction,
        chord: HotkeyChord,
        generation: u64,
    },
    Cancel {
        generation: u64,
    },
}

struct Runtime {
    inner: Mutex<HookInner>,
    pending: Mutex<Option<Captured>>,
    cmd_tx: Mutex<Option<CmdTx>>,
    thread_id: AtomicU32,
    started: AtomicBool,
    join: Mutex<Option<thread::JoinHandle<()>>>,
}

fn runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| Runtime {
        inner: Mutex::new(HookInner::new()),
        pending: Mutex::new(None),
        cmd_tx: Mutex::new(None),
        thread_id: AtomicU32::new(0),
        started: AtomicBool::new(false),
        join: Mutex::new(None),
    })
}

pub fn publish(snap: HotkeySnap) {
    let mut inner = runtime().inner.lock();
    let capturing = inner.snap.capturing;
    inner.snap = snap;
    inner.snap.capturing = capturing;
}

pub fn sync_state(state: &AppState) {
    publish(snapshot(state));
}

/// Start listening for `action`, or stop when `None`. Returns the generation the hook must echo.
pub fn begin_listen(action: Option<HotkeyAction>) -> u64 {
    let rt = runtime();
    let mut inner = rt.inner.lock();
    inner.listen_gen = inner.listen_gen.wrapping_add(1);
    inner.snap.capturing = action;
    *rt.pending.lock() = None;
    inner.listen_gen
}

pub fn cancel_listen() {
    let _ = begin_listen(None);
}

pub fn take_capture() -> Option<Captured> {
    runtime().pending.lock().take()
}

pub fn install(state: &AppState, cmd_tx: CmdTx) {
    let rt = runtime();
    *rt.cmd_tx.lock() = Some(cmd_tx);
    publish(snapshot(state));
    if rt.started.swap(true, Ordering::AcqRel) {
        return;
    }
    let (ready_tx, ready_rx) = mpsc::channel();
    let handle = thread::Builder::new()
        .name("hotkey".into())
        .spawn(move || hook_thread(ready_tx))
        .expect("hotkey thread");
    let _ = ready_rx.recv();
    *rt.join.lock() = Some(handle);
}

pub fn shutdown() {
    let rt = runtime();
    let id = rt.thread_id.load(Ordering::Acquire);
    if id != 0 {
        let _ = unsafe { PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0)) };
    }
    if let Some(handle) = rt.join.lock().take() {
        let _ = handle.join();
    }
}

fn hook_thread(ready: mpsc::Sender<()>) {
    unsafe {
        let mut msg = MSG::default();
        let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
        match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), None, 0) {
            Ok(hook) => {
                runtime().thread_id.store(GetCurrentThreadId(), Ordering::Release);
                let _ = ready.send(());
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                let _ = UnhookWindowsHookEx(hook);
                runtime().thread_id.store(0, Ordering::Release);
            }
            Err(e) => {
                error!(error = %e, "keyboard hook failed");
                let _ = ready.send(());
            }
        }
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    let pass = on_hook(wparam, lparam);
    if pass {
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    } else {
        LRESULT(1)
    }
}

fn on_hook(wparam: WPARAM, lparam: LPARAM) -> bool {
    let Some(key) = read_key(wparam, lparam) else {
        return true;
    };
    let Some(mut inner) = runtime().inner.try_lock() else {
        return true;
    };
    let refresh_modifiers = (key.vk as usize) < KEY_SLOTS
        && inner.snap.capturing.is_some()
        && is_modifier_vk(key.vk)
        && (!key.down || !inner.held[key.vk as usize]);
    let focused = target_focused(inner.snap.focus_hwnd);
    let outcome = step(&mut inner, key, focused);
    drop(inner);
    if refresh_modifiers {
        ping_ui();
    }
    match outcome {
        Step::Pass => true,
        Step::Swallow => false,
        Step::Fire(action) => {
            if let Some(tx) = runtime().cmd_tx.lock().clone() {
                let _ = tx.send(PipelineCommand::Hotkey(action));
            }
            false
        }
        Step::Bind { action, chord, generation } => {
            *runtime().pending.lock() = Some(Captured::Bind { action, chord, generation });
            ping_ui();
            false
        }
        Step::CancelListen { generation } => {
            *runtime().pending.lock() = Some(Captured::Cancel { generation });
            ping_ui();
            false
        }
    }
}

fn read_key(wparam: WPARAM, lparam: LPARAM) -> Option<KeyIn> {
    let kind = wparam.0 as u32;
    if !matches!(kind, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP) {
        return None;
    }
    let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let vk = u16::try_from(info.vkCode).unwrap_or(0);
    let down = !info.flags.contains(LLKHF_UP);
    Some(KeyIn {
        vk,
        down,
        win: is_win_vk(vk) || key_down(VK_LWIN.0) || key_down(VK_RWIN.0),
        ctrl: key_down(VK_CONTROL.0),
        alt: key_down(VK_MENU.0),
        shift: key_down(VK_SHIFT.0),
    })
}

fn key_down(vk: u16) -> bool {
    (unsafe { GetAsyncKeyState(i32::from(vk)) }) as u16 & 0x8000 != 0
}

fn target_focused(target: Option<isize>) -> bool {
    let Some(target) = target else {
        return false;
    };
    let fg = unsafe { GetForegroundWindow() };
    if fg.is_invalid() {
        return false;
    }
    let root = unsafe { GetAncestor(fg, GA_ROOT) };
    root.0 as isize == target || fg.0 as isize == target
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(token: &str) -> HotkeyChord {
        HotkeyChord::parse(token).unwrap()
    }

    fn inner_with(mut snap: HotkeySnap) -> HookInner {
        snap.capturing = None;
        HookInner {
            snap,
            listen_gen: 1,
            held: [false; KEY_SLOTS],
            swallow: [false; KEY_SLOTS],
        }
    }

    fn live(token: &str) -> HookInner {
        let mut snap = HotkeySnap::idle();
        snap.hotkeys.enabled = true;
        snap.hotkeys.only_when_target_focused = true;
        snap.focus_hwnd = Some(7);
        snap.hotkeys.set(HotkeyAction::StartPause, Some(chord(token)));
        snap.can[HotkeyAction::StartPause as usize] = true;
        inner_with(snap)
    }

    fn key(vk: u16, win: bool, ctrl: bool) -> KeyIn {
        KeyIn {
            vk,
            down: true,
            win,
            ctrl,
            alt: false,
            shift: false,
        }
    }

    fn release(inner: &mut HookInner, vk: u16) {
        let _ = step(
            inner,
            KeyIn {
                vk,
                down: false,
                win: false,
                ctrl: false,
                alt: false,
                shift: false,
            },
            true,
        );
    }

    #[test]
    fn binding_ignores_win_and_keeps_the_other_key() {
        let mut inner = inner_with(HotkeySnap::idle());
        inner.snap.capturing = Some(HotkeyAction::Stop);
        inner.listen_gen = 4;
        assert!(matches!(step(&mut inner, key(0x5B, true, false), false), Step::Swallow));
        assert_eq!(inner.snap.capturing, Some(HotkeyAction::Stop));
        let f9 = 0x78;
        let stepped = step(&mut inner, key(f9, true, true), false);
        match stepped {
            Step::Bind { action, chord, generation } => {
                assert_eq!(action, HotkeyAction::Stop);
                assert_eq!(generation, 4);
                assert!(chord.ctrl);
                assert!(!is_win_vk(chord.vk));
                assert_eq!(chord.to_token(), "ctrl+f9");
            }
            other => panic!("expected bind, got {other:?}"),
        }
        assert!(inner.snap.capturing.is_none());
    }

    #[test]
    fn binding_records_modifiers_the_system_did_not_see() {
        let mut inner = inner_with(HotkeySnap::idle());
        inner.snap.capturing = Some(HotkeyAction::Stop);
        // Left Ctrl, Shift, and Alt. Flags stay false: swallowing them skips the system key state.
        assert!(matches!(step(&mut inner, key(0xA2, false, false), false), Step::Swallow));
        assert!(matches!(step(&mut inner, key(0xA0, false, false), false), Step::Swallow));
        assert!(matches!(step(&mut inner, key(0xA4, false, false), false), Step::Swallow));
        match step(&mut inner, key(0x78, false, false), false) {
            Step::Bind { chord, .. } => assert_eq!(chord.to_token(), "ctrl+alt+shift+f9"),
            other => panic!("expected bind, got {other:?}"),
        }
    }

    #[test]
    fn binding_drops_a_modifier_after_keyup() {
        let mut inner = inner_with(HotkeySnap::idle());
        inner.snap.capturing = Some(HotkeyAction::Retry);
        assert!(matches!(step(&mut inner, key(0xA2, false, false), false), Step::Swallow));
        release(&mut inner, 0xA2);
        match step(&mut inner, key(0x78, false, false), false) {
            Step::Bind { chord, .. } => assert_eq!(chord.to_token(), "f9"),
            other => panic!("expected bind, got {other:?}"),
        }
    }

    #[test]
    fn binding_ignores_mouse_and_modifiers() {
        let mut inner = inner_with(HotkeySnap::idle());
        inner.snap.capturing = Some(HotkeyAction::Retry);
        assert!(matches!(step(&mut inner, key(0x01, false, false), false), Step::Swallow));
        assert!(matches!(step(&mut inner, key(0xA2, false, true), false), Step::Swallow));
        assert_eq!(inner.snap.capturing, Some(HotkeyAction::Retry));
    }

    #[test]
    fn live_win_combo_passes_the_other_key() {
        let mut inner = live("f9");
        let f9 = 0x78;
        assert!(matches!(step(&mut inner, key(f9, true, false), true), Step::Pass));
        release(&mut inner, f9);
        assert!(matches!(step(&mut inner, key(f9, false, false), true), Step::Fire(HotkeyAction::StartPause)));
    }

    #[test]
    fn live_uses_tracked_modifiers_and_win() {
        let mut inner = live("ctrl+shift+f8");
        assert!(matches!(step(&mut inner, key(0xA2, false, false), true), Step::Pass));
        assert!(matches!(step(&mut inner, key(0xA0, false, false), true), Step::Pass));
        assert!(matches!(step(&mut inner, key(0x77, false, false), true), Step::Fire(HotkeyAction::StartPause)));

        let mut win = live("f9");
        assert!(matches!(step(&mut win, key(0x5B, false, false), true), Step::Pass));
        assert!(matches!(step(&mut win, key(0x78, false, false), true), Step::Pass));
    }

    #[test]
    fn live_respects_focus_and_the_master_switch() {
        let mut inner = live("f8");
        let f8 = 0x77;
        assert!(matches!(step(&mut inner, key(f8, false, false), false), Step::Pass));
        release(&mut inner, f8);
        inner.snap.hotkeys.only_when_target_focused = false;
        assert!(matches!(step(&mut inner, key(f8, false, false), false), Step::Fire(HotkeyAction::StartPause)));
        let mut off = live("f8");
        off.snap.hotkeys.enabled = false;
        assert!(matches!(step(&mut off, key(f8, false, false), true), Step::Pass));
    }

    #[test]
    fn unavailable_action_passes_and_repeat_does_not_fire_again() {
        let mut inner = live("f9");
        inner.snap.can[HotkeyAction::StartPause as usize] = false;
        let f9 = 0x78;
        assert!(matches!(step(&mut inner, key(f9, false, false), true), Step::Pass));
        release(&mut inner, f9);
        inner.snap.can[HotkeyAction::StartPause as usize] = true;
        assert!(matches!(step(&mut inner, key(f9, false, false), true), Step::Fire(HotkeyAction::StartPause)));
        assert!(matches!(step(&mut inner, key(f9, false, false), true), Step::Swallow));
    }

    #[test]
    fn duplicate_chords_use_the_first_action() {
        let mut inner = live("a");
        inner.snap.hotkeys.set(HotkeyAction::Stop, Some(chord("a")));
        inner.snap.can[HotkeyAction::Stop as usize] = true;
        assert!(matches!(step(&mut inner, key(u16::from(b'A'), false, false), true), Step::Fire(HotkeyAction::StartPause)));
    }

    #[test]
    fn escape_cancels_listen() {
        let mut inner = inner_with(HotkeySnap::idle());
        inner.snap.capturing = Some(HotkeyAction::Retry);
        inner.listen_gen = 9;
        assert!(matches!(step(&mut inner, key(0x1B, false, false), false), Step::CancelListen { generation: 9 }));
        assert!(inner.snap.capturing.is_none());
    }
}
