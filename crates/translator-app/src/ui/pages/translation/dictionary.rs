//! Glossary editor as a child window, modal over the control window.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use rust_i18n::t;
use translator_core::DictionaryPair;
use translator_winui::{
    Border, Button, ContentDialog, ContentDialogResult, Grid, HorizontalAlignment, InfoBar, InfoBarSeverity, Layout, MicaBackdrop,
    MicaKind, Result, ScrollBarVisibility, ScrollViewer, SizeInt32, StackPanel, TextBlock, TitleBarTheme, VerticalAlignment, Window, auto,
    boxed, grid, keep, pixels, star, window_hwnd,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::EnableWindow,
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            GWL_STYLE, GWLP_HWNDPARENT, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, ICON_BIG, ICON_SMALL, SC_MINIMIZE,
            SW_MINIMIZE, SW_PARENTOPENING, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOZORDER, SendMessageW, SetForegroundWindow,
            SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_GETICON, WM_NCDESTROY, WM_SETICON, WM_SHOWWINDOW, WM_SYSCOMMAND,
            WS_MAXIMIZEBOX, WS_THICKFRAME,
        },
    },
};

use crate::{
    children,
    ui::{
        app::{control_hwnd, quiet},
        state::{DICTIONARIES, NamedDialog, UiCx, UiShared, commit_named, select_dictionary_profile, start_new_dictionary},
        widgets::{
            Binds, ButtonKind, Prop, button, button_style,
            cards::SEMI_BOLD,
            column,
            fields::{Combo, TextField},
            glyph_box, hs, icon_button,
            named::{NamedSnap, name_entry_row, named_confirm_inline},
            row, text,
        },
    },
};

// Fixed pixel columns keep the source/target fields aligned with their headers.
// Subtle + FontIcon clips if the cell is locked to 36×36; min 36 + Viewbox 16.
const FIELD: f64 = 220.0;
const BTN: f64 = 40.0;
const ROW: f64 = FIELD * 2.0 + BTN;
const PAD: f64 = 24.0;
const PAIR_GAP: f64 = 8.0;

fn pair_grid() -> Result<Grid> {
    let g = grid(&[], &[pixels(FIELD), pixels(FIELD), pixels(BTN)])?;
    g.width(ROW)?.halign(HorizontalAlignment::Left)?;
    Ok(g)
}

fn row_button(glyph: &str, on_click: impl Fn() + Send + 'static) -> Result<(Border, Button)> {
    let b = icon_button(glyph, on_click)?;
    b.SetContent(&glyph_box(glyph)?)?;
    let cell = Border::new()?;
    cell.column(2)?
        .halign(HorizontalAlignment::Center)?
        .valign(VerticalAlignment::Center)?;
    cell.SetChild(&b)?;
    Ok((cell, b))
}

struct PairRow {
    grid: Grid,
    source: TextField,
    target: TextField,
    delete: Button,
    enabled: Prop<bool>,
}

impl PairRow {
    fn new(cx: &UiCx, i: usize) -> Result<Self> {
        let field = |placeholder: &str, id: String, set: fn(&mut DictionaryPair, String)| -> Result<(Border, TextField)> {
            let cx = cx.clone();
            let f = TextField::new(placeholder, move |text| {
                cx.with_mut(|ui| {
                    if let Some(row) = ui.dictionary.pairs.get_mut(i) {
                        set(row, text);
                    }
                });
            })?;
            f.tb.automation_id(&id)?.halign(HorizontalAlignment::Stretch)?;
            let cell = Border::new()?;
            cell.margin(0.0, 0.0, 8.0, 0.0)?;
            cell.SetChild(&f.tb)?;
            Ok((cell, f))
        };
        let (source_cell, source) = field(&t!("tr.term_source"), format!("txt-dictionary-source-{i}"), |row, text| row.source = text)?;
        let (target_cell, target) = field(&t!("tr.term_target"), format!("txt-dictionary-target-{i}"), |row, text| row.target = text)?;
        source_cell.column(0)?;
        target_cell.column(1)?;
        let (delete_cell, delete) = row_button("\u{E74D}", {
            let cx = cx.clone();
            move || {
                cx.with_mut(|ui| {
                    if i < ui.dictionary.pairs.len() {
                        ui.dictionary.pairs.remove(i);
                    }
                });
            }
        })?;
        delete.automation_name(&t!("action.delete"))?;
        let grid = pair_grid()?;
        children!(grid; source_cell, target_cell, delete_cell);
        Ok(Self {
            grid,
            source,
            target,
            delete,
            enabled: Prop::default(),
        })
    }

    fn set(&mut self, pair: &DictionaryPair, can_edit: bool) -> Result<()> {
        self.source.set(&pair.source)?;
        self.target.set(&pair.target)?;
        let (source, target, delete) = (&self.source.tb, &self.target.tb, &self.delete);
        self.enabled.set(can_edit, |&e| {
            source.SetIsEnabled(e)?;
            target.SetIsEnabled(e)?;
            delete.SetIsEnabled(e)
        })
    }
}

struct PairList {
    cx: UiCx,
    scroll: ScrollViewer,
    list: StackPanel,
    rows: Vec<PairRow>,
    add: Button,
    add_enabled: Prop<bool>,
}

impl PairList {
    fn new(cx: &UiCx) -> Result<Self> {
        let (add_cell, add) = row_button("\u{E710}", {
            let cx = cx.clone();
            move || cx.with_mut(|ui| ui.dictionary.pairs.push(DictionaryPair::default()))
        })?;
        add.automation_id("btn-dictionary-add")?.automation_name(&t!("action.add"))?;
        let spacer = |col: i32| -> Result<Border> {
            let b = Border::new()?;
            b.height(BTN)?.column(col)?;
            Ok(b)
        };
        let add_row = pair_grid()?;
        children!(add_row; spacer(0)?, spacer(1)?, add_cell);
        let list = column(PAIR_GAP)?;
        children!(list; add_row);
        let scroll = ScrollViewer::new()?;
        scroll.width(ROW)?;
        scroll.SetHorizontalScrollBarVisibility(ScrollBarVisibility::Disabled)?;
        scroll.SetVerticalScrollBarVisibility(ScrollBarVisibility::Auto)?;
        scroll.SetContent(&list)?;
        Ok(Self {
            cx: cx.clone(),
            scroll,
            list,
            rows: Vec::new(),
            add,
            add_enabled: Prop::default(),
        })
    }

    fn set(&mut self, pairs: &[DictionaryPair], can_edit: bool) -> Result<()> {
        let children = self.list.Children()?;
        while self.rows.len() > pairs.len() {
            self.rows.pop();
            children.RemoveAt(self.rows.len() as u32)?;
        }
        while self.rows.len() < pairs.len() {
            let row = PairRow::new(&self.cx, self.rows.len())?;
            children.InsertAt(self.rows.len() as u32, &row.grid)?;
            self.rows.push(row);
        }
        for (row, pair) in self.rows.iter_mut().zip(pairs) {
            row.set(pair, can_edit)?;
        }
        let add = &self.add;
        self.add_enabled.set(can_edit, |&e| add.SetIsEnabled(e))
    }
}

/// Shows the glossary editor window while `editor_open` is set.
pub fn dictionary_editor_window(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<()> {
    let cx = cx.clone();
    let mut editor: Option<Editor> = None;
    b.add(move |ui, chrome| {
        if !ui.dictionary.editor_open {
            editor = None;
            return Ok(());
        }
        let editor = match &mut editor {
            Some(editor) => editor,
            None => editor.insert(Editor::new(&cx)?),
        };
        editor.binds.run(ui, chrome)
    });
    Ok(())
}

/// Editor window procedure: minimizing the editor minimizes the control window (`owner`) instead.
/// A minimized owned window has no taskbar button, so it could not be brought back; Windows hides
/// owned windows with their owner and shows them again when the owner is restored.
unsafe extern "system" fn minimize_owner(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, id: usize, owner: usize) -> LRESULT {
    let owner = HWND(owner as *mut _);
    match msg {
        WM_SYSCOMMAND if (wparam.0 & 0xFFF0) as u32 == SC_MINIMIZE => {
            // Alt+Tab leaves a disabled minimized window minimized, so keep it enabled until it is restored.
            // SAFETY: `owner` is the live control window, which outlives the editor.
            let _ = unsafe { EnableWindow(owner, true) };
            // SAFETY: `owner` is the live control window, which outlives the editor.
            let _ = unsafe { ShowWindow(owner, SW_MINIMIZE) };
            return LRESULT(0);
        }
        WM_SHOWWINDOW if wparam.0 != 0 && lparam.0 as u32 == SW_PARENTOPENING.0 => {
            // SAFETY: `owner` is the live control window, which outlives the editor.
            let _ = unsafe { EnableWindow(owner, false) };
            // SAFETY: `hwnd` is the live editor window this procedure was called for.
            let _ = unsafe { SetForegroundWindow(hwnd) };
        }
        WM_NCDESTROY => {
            // SAFETY: removes the subclass installed on `hwnd` with this procedure and `id`.
            let _ = unsafe { RemoveWindowSubclass(hwnd, Some(minimize_owner), id) };
        }
        _ => {}
    }
    // SAFETY: passes comctl32's own arguments for the live editor window to the next procedure.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// Re-enables the control window and hands it the focus the editor had. Call while the editor still
/// exists: once it is gone, Windows activates another app instead of the owner.
fn release_owner(owner: HWND, editor: HWND) {
    // SAFETY: `owner` is the control window, which outlives the editor.
    let _ = unsafe { EnableWindow(owner, true) };
    // SAFETY: takes no arguments; only reads the foreground window handle.
    if unsafe { GetForegroundWindow() } == editor {
        // SAFETY: `owner` is the live control window on this thread, and this process is foreground.
        let _ = unsafe { SetForegroundWindow(owner) };
    }
}

/// A closed WinUI window cannot be shown again, so each open builds a new one.
/// It disables the control window while it lives, like a modal dialog.
struct Editor {
    window: Window,
    hwnd: HWND,
    owner: HWND,
    binds: Binds<UiShared>,
}

impl Drop for Editor {
    fn drop(&mut self) {
        release_owner(self.owner, self.hwnd);
        let _ = self.window.Close();
    }
}

impl Editor {
    fn new(cx: &UiCx) -> Result<Self> {
        let mut binds = Binds::default();
        let b = &mut binds;
        let save = |ui: &mut UiShared| {
            if let Some(name) = ui.dictionary.loaded.clone() {
                commit_named(ui, &DICTIONARIES, name);
            }
            // A failed save keeps the window open with the edits still in the buffer.
            if !ui.dictionary.dirty() {
                ui.dictionary.close_editor();
            }
        };
        let cancel = |ui: &mut UiShared| {
            ui.dictionary.revert();
            ui.dictionary.close_editor();
        };

        let mut combo = Combo::new(&t!("tr.select_dictionary"), {
            let cx = cx.clone();
            move |idx| cx.with_mut(|ui| select_dictionary_profile(ui, idx))
        })?;
        combo
            .cb
            .automation_id("cmb-dictionary-editor")?
            .width(180.0)?
            .min_width(140.0)?
            .valign(VerticalAlignment::Center)?;
        let new_btn = button(&t!("action.new"), {
            let cx = cx.clone();
            move || cx.with_mut(start_new_dictionary)
        })?;
        new_btn.automation_id("btn-dictionary-new")?;
        let delete_btn = button(&t!("action.delete"), {
            let cx = cx.clone();
            move || {
                cx.with_mut(|ui| {
                    if let Some(name) = ui.dictionary.selected_name() {
                        ui.dictionary.pick.dialog = NamedDialog::Delete { name };
                    }
                });
            }
        })?;
        let toolbar = row(8.0)?;
        toolbar.valign(VerticalAlignment::Center)?.halign(HorizontalAlignment::Left)?;
        children!(toolbar; combo.cb, new_btn, delete_btn);

        let error_bar = InfoBar::new()?;
        error_bar.SetSeverity(InfoBarSeverity::Error)?;
        error_bar.SetIsClosable(false)?;

        let header = |key: &str, col: i32| -> Result<TextBlock> {
            let tb = text(&t!(key))?;
            tb.SetFontWeight(SEMI_BOLD)?;
            tb.column(col)?.margin(0.0, 0.0, 8.0, 0.0)?;
            Ok(tb)
        };
        let headers = pair_grid()?;
        let header_gap = Border::new()?;
        header_gap.width(BTN)?.column(2)?;
        children!(headers; header("tr.term_source", 0)?, header("tr.term_target", 1)?, header_gap);

        let mut pairs = PairList::new(cx)?;
        let name_row = name_entry_row(b, cx, &DICTIONARIES)?;
        let confirm = named_confirm_inline(b, cx, &DICTIONARIES)?;
        let top = column(8.0)?;
        children!(top; toolbar, name_row, confirm, error_bar, headers);
        pairs.scroll.row(1)?.margin(0.0, PAIR_GAP, 0.0, 0.0)?;

        let body = grid(&[auto(), star(1.0), auto()], &[star(1.0)])?;
        body.width(ROW)?.halign(HorizontalAlignment::Left)?.margin(PAD, PAD, PAD, PAD)?;

        let ask = ContentDialog::new()?;
        ask.SetTitle(&boxed(&t!("tr.unsaved_dictionary_title")))?;
        ask.SetPrimaryButtonText(&hs(&t!("action.save")))?;
        ask.SetSecondaryButtonText(&hs(&t!("action.dont_save")))?;
        ask.SetCloseButtonText(&hs(&t!("action.cancel")))?;
        let asking = Arc::new(AtomicBool::new(false));
        keep(ask.Closed({
            let (cx, asking) = (cx.clone(), asking.clone());
            move |_, args| {
                asking.store(false, Ordering::Relaxed);
                match args.as_ref().and_then(|a| a.Result().ok()) {
                    Some(ContentDialogResult::Primary) => cx.with_mut(save),
                    Some(ContentDialogResult::Secondary) => cx.with_mut(cancel),
                    _ => {}
                }
            }
        }))?;
        // Shows `ask` when closing would drop unsaved edits; `Ok(false)` means close now.
        let ask_unsaved = {
            let (cx, ask, body) = (cx.clone(), ask.clone(), body.clone());
            move || -> Result<bool> {
                let name = {
                    let ui = cx.shared.lock();
                    ui.dictionary.loaded.clone().filter(|_| ui.dictionary.dirty())
                };
                let Some(name) = name else {
                    return Ok(false);
                };
                // Leave an open question alone: a second ShowAsync fails and the dialog then never closes.
                if asking.swap(true, Ordering::Relaxed) {
                    return Ok(true);
                }
                let shown = (|| -> Result<()> {
                    ask.SetContent(&text(&t!("tr.unsaved_dictionary_body", name = name))?)?;
                    ask.SetXamlRoot(&body.XamlRoot()?)?;
                    // WinUI may return S_OK without an async handle, which still means shown.
                    ask.ShowAsync()
                        .map(drop)
                        .or_else(|e| if e.code().is_ok() { Ok(()) } else { Err(e) })
                })();
                if shown.is_err() {
                    asking.store(false, Ordering::Relaxed);
                }
                shown.map(|()| true)
            }
        };

        let save_btn = button(&t!("action.save"), {
            let cx = cx.clone();
            move || cx.with_mut(save)
        })?;
        button_style(&save_btn, ButtonKind::Accent)?;
        let cancel_btn = button(&t!("action.cancel"), {
            let (cx, ask_unsaved) = (cx.clone(), ask_unsaved.clone());
            move || {
                if matches!(ask_unsaved(), Ok(false)) {
                    cx.with_mut(cancel);
                }
            }
        })?;
        let footer = row(8.0)?;
        footer.row(2)?.halign(HorizontalAlignment::Right)?.margin(0.0, 16.0, 0.0, 0.0)?;
        children!(footer; save_btn, cancel_btn);
        children!(body; top, pairs.scroll, footer);

        let window = Window::new()?;
        window.SetContent(&body)?;
        window.SetTitle(&hs(&t!("tr.editor_title")))?;
        let mica = MicaBackdrop::new()?;
        mica.SetKind(MicaKind::Base)?;
        window.SetSystemBackdrop(&mica)?;
        let owner = control_hwnd().unwrap_or_default();
        let hwnd = HWND(window_hwnd(&window)? as *mut _);
        let editor = hwnd.0 as isize;
        keep(window.Closed({
            let cx = cx.clone();
            move |_, args| {
                // A sync dropping the editor closes it with the state lock held, and the state already says closed.
                if !quiet() {
                    if !matches!(ask_unsaved(), Ok(false)) {
                        // Stay open until the user answers.
                        if let Some(args) = args.as_ref() {
                            let _ = args.SetHandled(true);
                        }
                        return;
                    }
                    cx.with_mut(|ui| {
                        if ui.dictionary.editor_open {
                            cancel(ui);
                        }
                    });
                }
                release_owner(HWND(owner as *mut _), HWND(editor as *mut _));
            }
        }))?;

        let owner_hwnd = HWND(owner as *mut _);
        // SAFETY: `owner_hwnd` is the live control window.
        let dpi = unsafe { GetDpiForWindow(owner_hwnd) }.max(96);
        let px = |dips: f64| (dips * f64::from(dpi) / 96.0).round() as i32;
        let app_window = window.AppWindow()?;
        app_window.TitleBar()?.SetPreferredTheme(TitleBarTheme::UseDefaultAppMode)?;
        // Fixed size and no maximize. Before `ResizeClient`, which sizes for the frame.
        // SAFETY: `hwnd` is the live editor window.
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } & !((WS_THICKFRAME.0 | WS_MAXIMIZEBOX.0) as isize);
        // SAFETY: `hwnd` is the live editor window.
        unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, style) };
        // SAFETY: `hwnd` is the live editor window; `owner` is the live control window on the same thread.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner) };
        // SAFETY: `hwnd` is the live editor window; `minimize_owner` removes itself on `WM_NCDESTROY`.
        let _ = unsafe { SetWindowSubclass(hwnd, Some(minimize_owner), 0, owner as usize) };
        // Borrow the control window's icons; it keeps them for the life of the process.
        for kind in [ICON_SMALL, ICON_BIG] {
            // SAFETY: `owner_hwnd` is the live control window.
            let icon = unsafe { SendMessageW(owner_hwnd, WM_GETICON, Some(WPARAM(kind as usize)), None) };
            // SAFETY: `hwnd` is the live editor window; the icon outlives it.
            unsafe { SendMessageW(hwnd, WM_SETICON, Some(WPARAM(kind as usize)), Some(LPARAM(icon.0))) };
        }
        app_window.ResizeClient(SizeInt32 {
            Width: px(ROW + PAD * 2.0),
            Height: px(600.0),
        })?;
        let (mut outer, mut own) = (RECT::default(), RECT::default());
        let mut monitor = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: `owner_hwnd` is the live control window.
        let screen = unsafe { MonitorFromWindow(owner_hwnd, MONITOR_DEFAULTTONEAREST) };
        // SAFETY: `owner_hwnd` is the live control window; `outer` outlives the call.
        let outer_ok = unsafe { GetWindowRect(owner_hwnd, &mut outer) }.is_ok();
        // SAFETY: `hwnd` is the live editor window; `own` outlives the call.
        let own_ok = unsafe { GetWindowRect(hwnd, &mut own) }.is_ok();
        // SAFETY: `screen` came from `MonitorFromWindow`; `monitor` has `cbSize` set and outlives the call.
        let monitor_ok = unsafe { GetMonitorInfoW(screen, &mut monitor) }.as_bool();
        if outer_ok && own_ok && monitor_ok {
            // The window cannot be resized, so keep all of it, title bar and footer, inside the work area.
            let work = monitor.rcWork;
            let w = (own.right - own.left).min(work.right - work.left);
            let h = (own.bottom - own.top).min(work.bottom - work.top);
            let x = ((outer.left + outer.right - w) / 2).clamp(work.left, work.right - w);
            let y = ((outer.top + outer.bottom - h) / 2).clamp(work.top, work.bottom - h);
            // SAFETY: `hwnd` is the live editor window.
            let _ = unsafe { SetWindowPos(hwnd, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED) };
        }
        // SAFETY: `owner_hwnd` is the live control window.
        let _ = unsafe { EnableWindow(owner_hwnd, false) };
        window.Activate()?;

        let mut error = Prop::default();
        let mut can_edit_prop = Prop::default();
        b.add(move |ui, _| {
            let snap = NamedSnap::take(ui, &DICTIONARIES);
            let can_edit = ui.dictionary.loaded.is_some();
            let enabled = !snap.names.is_empty();
            let selected = snap.selected();
            delete_btn.SetIsEnabled(selected.is_some())?;
            combo.set(snap.names, selected, enabled)?;
            let last_error = ui.state.read().last_error.clone().unwrap_or_default();
            error.set(last_error, |e| {
                error_bar.SetTitle(&hs(e))?;
                error_bar.SetIsOpen(!e.is_empty())
            })?;
            can_edit_prop.set(can_edit, |&e| save_btn.SetIsEnabled(e))?;
            pairs.set(&ui.dictionary.pairs, can_edit)
        });
        Ok(Self {
            window,
            hwnd,
            owner: owner_hwnd,
            binds,
        })
    }
}
