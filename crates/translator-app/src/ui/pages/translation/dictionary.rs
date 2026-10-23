//! Glossary editor as a ContentDialog on the control window.

use rust_i18n::t;
use translator_core::DictionaryPair;
use translator_winui::{
    Border, Button, Grid, HorizontalAlignment, InfoBar, InfoBarSeverity, Layout, Result, ScrollBarVisibility, ScrollViewer, StackPanel,
    TextBlock, VerticalAlignment, grid, pixels,
};

use crate::{
    children,
    ui::{
        state::{
            DICTIONARIES, NamedDialog, UiCx, UiShared, commit_named, open_dictionary_editor, select_dictionary_profile,
            start_new_dictionary,
        },
        widgets::{
            Binds, Prop, button,
            cards::SEMI_BOLD,
            column,
            dialog::Dialog,
            fields::{Combo, TextField},
            glyph_box, hs, icon_button,
            named::{NamedSnap, name_entry_row, named_confirm_inline},
            row, text,
        },
    },
};

// Pixel columns so ContentScrollViewer's infinite measure cannot stretch
// the row past the ContentDialog. Subtle + FontIcon clips if the cell is locked to 36×36; min 36 + Viewbox 16.
const FIELD: f64 = 220.0;
const BTN: f64 = 40.0;
const ROW: f64 = FIELD * 2.0 + BTN;
// Default VerticalScrollBarVisibility is Visible. Pin the list height to the rows so
// Auto does not paint a bar when ContentDialog measures it with infinite height.
const PAIR_ROW: f64 = 40.0;
const PAIR_GAP: f64 = 8.0;
const PAIR_MAX: f64 = 320.0;

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
            cell.halign(HorizontalAlignment::Stretch)?.margin(0.0, 0.0, 8.0, 0.0)?;
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
    height: Prop<u64>,
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
        list.halign(HorizontalAlignment::Stretch)?;
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
            height: Prop::default(),
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
        self.add_enabled.set(can_edit, |&e| add.SetIsEnabled(e))?;
        let n = (pairs.len() + 1) as f64;
        let content_h = n * PAIR_ROW + (n - 1.0).max(0.0) * PAIR_GAP;
        let scroll = &self.scroll;
        self.height
            .set(content_h.min(PAIR_MAX).to_bits(), |&h| scroll.height(f64::from_bits(h)).map(drop))
    }
}

pub fn dictionary_editor_dialog(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<()> {
    let mut dialog = Dialog::new({
        let cx = cx.clone();
        move |primary| {
            cx.with_mut(|ui| {
                if !ui.dictionary.editor_open {
                    return;
                }
                if primary {
                    if let Some(name) = ui.dictionary.loaded.clone() {
                        commit_named(ui, &DICTIONARIES, name);
                    }
                    if ui.dictionary.dirty() {
                        // The save failed; reopen with the edits still in the buffer.
                        ui.dictionary.close_editor();
                        open_dictionary_editor(ui);
                        return;
                    }
                } else {
                    ui.dictionary.revert();
                }
                ui.dictionary.close_editor();
            });
        }
    })?;

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
    let body = column(8.0)?;
    body.width(ROW)?.halign(HorizontalAlignment::Left)?;
    children!(body; toolbar, name_row, confirm, error_bar, headers, pairs.scroll);
    dialog.cd.SetContent(&body)?;
    dialog.labels(&t!("tr.editor_title"), &t!("action.save"), &t!("action.cancel"))?;

    let mut error = Prop::default();
    let mut can_edit_prop = Prop::default();
    b.add(move |ui, _| {
        let open = ui.dictionary.editor_open;
        if open {
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
            let cd = &dialog.cd;
            can_edit_prop.set(can_edit, |&e| cd.SetIsPrimaryButtonEnabled(e))?;
            pairs.set(&ui.dictionary.pairs, can_edit)?;
        }
        dialog.show(open)
    });
    Ok(())
}
