//! Operator workspace: session settings above a divider, then preview | results.

pub mod preview;

use std::collections::HashMap;

use rust_i18n::t;
use translator_capture::list_windows;
use translator_core::{HistoryEntry, NormRect, PreviewInfo, sanitize_regions};
use translator_ocr::OcrEngine;
use translator_translate::blocks_to_translated_text;
use translator_winui::{
    Border, Button, Grid, HorizontalAlignment, Layout, Result, ScrollViewer, StackPanel, TextBlock, TextWrapping, VerticalAlignment, auto,
    grid, star,
};

use crate::{
    children,
    pipeline::PipelineCommand,
    ui::{
        app::raster_scale,
        chrome::status_infobar,
        pages::dashboard::preview::{Preview, PreviewInput},
        state::{ChromeSnap, NamedDialog, REGION_PRESETS, UiCx, UiShared, selected_preset, truncate},
        widgets::{
            Binds, ButtonKind, Prop, Theme, button, button_style,
            cards::{BOLD, SEMI_BOLD},
            click, column,
            fields::Combo,
            foreground, hs, icon_button, muted,
            named::{NamedSnap, name_entry_row, named_combo, named_confirm_dialog, named_delete_button},
            row, rule, text,
        },
    },
};

pub struct DashSnap {
    auto_running: bool,
    translate_in_flight: bool,
    last_ocr_ms: Option<u64>,
    last_ocr_block_count: u32,
    preview: PreviewInfo,
    preview_regions: Vec<NormRect>,
    ocr_text: String,
    translation: String,
    history: Vec<HistoryEntry>,
    selected_idx: Option<usize>,
    selected_hwnd: Option<isize>,
    target_hwnd: Option<isize>,
    window_labels: Vec<String>,
    region_select_active: bool,
    presets: NamedSnap,
}

fn take_dash(ui: &UiShared) -> DashSnap {
    let presets = NamedSnap::take(ui, &REGION_PRESETS);
    let s = ui.state.read();
    DashSnap {
        auto_running: s.auto_running,
        translate_in_flight: s.translate_in_flight,
        last_ocr_ms: s.last_ocr_ms,
        last_ocr_block_count: s.last_ocr_block_count,
        preview: s.preview.clone(),
        preview_regions: if s.region_select_active {
            s.region_select_draft.clone()
        } else {
            s.ocr_regions.clone()
        },
        ocr_text: OcrEngine::blocks_to_text(&s.latest_ocr_blocks),
        translation: blocks_to_translated_text(&s.latest_translated_blocks),
        history: s.history.iter().cloned().collect(),
        selected_idx: ui.selected_idx.filter(|&i| i < ui.windows.len()),
        selected_hwnd: ui.selected_idx.and_then(|i| ui.windows.get(i).map(|w| w.hwnd)),
        target_hwnd: s.target_hwnd,
        window_labels: ui.windows.iter().map(|w| truncate(&w.title, 72)).collect(),
        region_select_active: s.region_select_active,
        presets,
    }
}

/// Source | divider | translation, then a rule under the row.
fn results_pair(left: &TextBlock, right: &TextBlock) -> Result<StackPanel> {
    let cells = grid(&[], &[star(1.0), auto(), star(1.0)])?;
    cells.halign(HorizontalAlignment::Stretch)?;
    let left_cell = Border::new()?;
    left_cell
        .column(0)?
        .halign(HorizontalAlignment::Stretch)?
        .valign(VerticalAlignment::Top)?
        .margin(0.0, 0.0, 8.0, 0.0)?;
    left_cell.SetChild(left)?;
    let divider = rule()?;
    divider
        .width(1.0)?
        .halign(HorizontalAlignment::Center)?
        .valign(VerticalAlignment::Stretch)?
        .column(1)?;
    let right_cell = Border::new()?;
    right_cell
        .column(2)?
        .halign(HorizontalAlignment::Stretch)?
        .valign(VerticalAlignment::Top)?
        .margin(8.0, 0.0, 0.0, 0.0)?;
    right_cell.SetChild(right)?;
    children!(cells; left_cell, divider, right_cell);
    let line = rule()?;
    line.height(1.0)?.halign(HorizontalAlignment::Stretch)?;
    let pair = column(8.0)?;
    pair.halign(HorizontalAlignment::Stretch)?;
    children!(pair; cells, line);
    Ok(pair)
}

/// One source / translation row of the results list.
struct ResultRow {
    root: StackPanel,
    source: TextBlock,
    translation: TextBlock,
    shown: Prop<(String, String, u64)>,
}

impl ResultRow {
    fn new() -> Result<Self> {
        let cell = || -> Result<TextBlock> {
            let tb = TextBlock::new()?;
            tb.SetTextWrapping(TextWrapping::Wrap)?;
            tb.SetIsTextSelectionEnabled(true)?;
            Ok(tb)
        };
        let source = cell()?;
        let translation = cell()?;
        let root = results_pair(&source, &translation)?;
        Ok(Self {
            root,
            source,
            translation,
            shown: Prop::default(),
        })
    }

    fn set(&mut self, source: &str, translation: &str, opacity: f64) -> Result<()> {
        let (s, t) = (&self.source, &self.translation);
        self.shown
            .set((source.to_string(), translation.to_string(), opacity.to_bits()), |(src, dst, o)| {
                let o = f64::from_bits(*o);
                s.SetText(&hs(src))?;
                s.opacity(o)?;
                t.SetText(&hs(dst))?;
                t.opacity(o)?;
                Ok(())
            })
    }
}

/// Results list keyed by `"live"` / history id, so rows keep their element across syncs.
struct Results {
    list: StackPanel,
    rows: HashMap<String, ResultRow>,
    order: Vec<String>,
}

impl Results {
    fn set(&mut self, items: &[(String, String, String, f64)]) -> Result<()> {
        let keys: Vec<String> = items.iter().map(|(k, ..)| k.clone()).collect();
        self.rows.retain(|k, _| keys.contains(k));
        for (key, source, translation, opacity) in items {
            if !self.rows.contains_key(key) {
                self.rows.insert(key.clone(), ResultRow::new()?);
            }
            if let Some(row) = self.rows.get_mut(key) {
                row.set(source, translation, *opacity)?;
            }
        }
        if keys != self.order {
            let children = self.list.Children()?;
            children.Clear()?;
            for key in &keys {
                if let Some(row) = self.rows.get(key) {
                    children.Append(&row.root)?;
                }
            }
            self.order = keys;
        }
        Ok(())
    }
}

pub struct Dashboard {
    pub root: Grid,
    ui_binds: Binds<UiShared>,
    dash_binds: Binds<DashSnap>,
}

impl Dashboard {
    pub fn sync(&mut self, ui: &UiShared, chrome: &ChromeSnap) -> Result<()> {
        self.ui_binds.run(ui, chrome)?;
        let dash = take_dash(ui);
        self.dash_binds.run(&dash, chrome)
    }
}

pub fn dashboard_page(cx: &UiCx) -> Result<Dashboard> {
    let mut ui_binds = Binds::default();
    let mut d = Binds::<DashSnap>::default();

    let title = text(&t!("nav.dashboard"))?;
    title.SetFontSize(28.0)?;
    title.SetFontWeight(BOLD)?;
    let settings = column(12.0)?;
    settings
        .halign(HorizontalAlignment::Stretch)?
        .row(0)?
        .column(0)?
        .valign(VerticalAlignment::Top)?;
    children!(settings;
        title,
        status_infobar(&mut d)?,
        build_window_row(&mut d, cx)?,
        build_regions_pane(&mut ui_binds, &mut d, cx)?,
    );
    named_confirm_dialog(&mut ui_binds, cx, &REGION_PRESETS)?;

    let divider = rule()?;
    divider
        .height(1.0)?
        .halign(HorizontalAlignment::Stretch)?
        .row(1)?
        .column(0)?
        .margin(0.0, 12.0, 0.0, 16.0)?;

    let cancel = button(&t!("action.cancel"), {
        let cx = cx.clone();
        move || cx.send_cmd(PipelineCommand::CancelTranslate)
    })?;
    let retry = button(&t!("action.retry"), {
        let cx = cx.clone();
        move || cx.send_cmd(PipelineCommand::ManualCapture)
    })?;
    let clear_chat = button(&t!("dash.clear_chat"), {
        let cx = cx.clone();
        move || cx.send_cmd(PipelineCommand::ResetConversation)
    })?;
    clear_chat.tooltip(&t!("dash.clear_chat_tip"))?;
    let actions = row(8.0)?;
    actions.halign(HorizontalAlignment::Left)?;
    children!(actions; cancel, retry, clear_chat);
    let mut action_state = Prop::default();
    d.add(move |snap, chrome| {
        let in_flight = snap.translate_in_flight;
        let busy = chrome.capture_busy;
        let cancel_tip = if busy {
            t!("capture.waiting_stop")
        } else if in_flight {
            t!("dash.cancel_progress")
        } else {
            t!("dash.cancel_idle")
        };
        let retry_tip = if busy {
            t!("capture.waiting_stop")
        } else if in_flight {
            t!("dash.cancel_first")
        } else if snap.auto_running {
            t!("dash.capture_now")
        } else {
            t!("dash.start_first")
        };
        let state = (in_flight && !busy, snap.auto_running && !in_flight && !busy, cancel_tip.into_owned(), retry_tip.into_owned());
        action_state.set(state, |(cancel_on, retry_on, cancel_tip, retry_tip)| {
            cancel.SetIsEnabled(*cancel_on)?;
            retry.SetIsEnabled(*retry_on)?;
            cancel.tooltip(cancel_tip)?;
            retry.tooltip(retry_tip)?;
            Ok(())
        })
    });

    let mut preview = Preview::new()?;
    let preview_pane = column(8.0)?;
    preview_pane.fill()?.row(0)?.column(0)?.margin(0.0, 0.0, 16.0, 0.0)?;
    children!(preview_pane; actions, preview.none, preview.image, preview.unavailable);
    d.add(move |snap, _| {
        preview.sync(
            &PreviewInput {
                sequence: snap.preview.sequence,
                width: snap.preview.width,
                height: snap.preview.height,
                rgba: snap.preview.rgba.as_ref(),
                regions: &snap.preview_regions,
            },
            raster_scale(),
        )
    });

    let ocr_label = text(&t!("dash.ocr"))?;
    ocr_label.SetFontSize(14.0)?;
    ocr_label.SetFontWeight(SEMI_BOLD)?;
    ocr_label.valign(VerticalAlignment::Center)?.column(0)?;
    let ocr_desc = muted("")?;
    ocr_desc.valign(VerticalAlignment::Center)?.column(1)?;
    let ocr_line = grid(&[], &[star(1.0), auto()])?;
    ocr_line.halign(HorizontalAlignment::Stretch)?;
    children!(ocr_line; ocr_label, ocr_desc);
    let header_label = |key: &str| -> Result<TextBlock> {
        let tb = muted(&t!(key))?;
        tb.SetFontWeight(SEMI_BOLD)?;
        Ok(tb)
    };
    let results_header = results_pair(&header_label("dash.source")?, &header_label("dash.translation")?)?;
    let head = column(8.0)?;
    head.halign(HorizontalAlignment::Stretch)?.row(0)?.column(0)?;
    children!(head; ocr_line, results_header);

    let no_history = text(&t!("dash.no_history"))?;
    foreground(&no_history, Theme::PrimaryText)?;
    no_history.opacity(0.72)?;
    let list = column(8.0)?;
    list.halign(HorizontalAlignment::Stretch)?;
    let rows_host = column(0.0)?;
    children!(rows_host; no_history, list);
    let scroll = ScrollViewer::new()?;
    scroll.fill()?.margin(0.0, 8.0, 0.0, 0.0)?.row(1)?.column(0)?;
    scroll.SetContent(&rows_host)?;
    let mut results = Results {
        list,
        rows: HashMap::new(),
        order: Vec::new(),
    };
    let mut desc = Prop::default();
    let mut empty = Prop::default();
    d.add(move |snap, _| {
        let ocr_time = snap.last_ocr_ms.map(|ms| format!("{ms} ms")).unwrap_or_else(|| "—".into());
        desc.set(t!("dash.ocr_blocks", time = ocr_time, count = snap.last_ocr_block_count).into_owned(), |d| ocr_desc.SetText(&hs(d)))?;
        let live_src = &snap.ocr_text;
        let live_dst = &snap.translation;
        let has_live = !live_src.is_empty() || !live_dst.is_empty();
        let mut history: &[HistoryEntry] = &snap.history;
        if has_live && !snap.translate_in_flight && history.first().is_some_and(|h| h.covered_by(live_src, live_dst)) {
            history = &history[1..];
        }
        let is_empty = !has_live && history.is_empty();
        empty.set(is_empty, |&e| {
            results_header.visible(!e)?;
            no_history.visible(e)?;
            results.list.visible(!e)?;
            Ok(())
        })?;
        let items: Vec<(String, String, String, f64)> = has_live
            .then(|| ("live".to_string(), live_src.clone(), live_dst.clone(), 1.0))
            .into_iter()
            .chain(
                history
                    .iter()
                    .map(|h| (format!("h{}", h.id), h.source_text.clone(), h.translated_text.clone(), 0.72)),
            )
            .collect();
        results.set(&items)
    });

    let output = grid(&[auto(), star(1.0)], &[star(1.0)])?;
    output.fill()?.row(0)?.column(1)?;
    children!(output; head, scroll);

    let workspace = grid(&[star(1.0)], &[star(1.0), star(1.0)])?;
    workspace.fill()?.row(2)?.column(0)?;
    children!(workspace; preview_pane, output);

    let root = grid(&[auto(), auto(), star(1.0)], &[star(1.0)])?;
    root.fill()?;
    children!(root; settings, divider, workspace);
    Ok(Dashboard {
        root,
        ui_binds,
        dash_binds: d,
    })
}

fn build_window_row(d: &mut Binds<DashSnap>, cx: &UiCx) -> Result<StackPanel> {
    let mut picker = Combo::new(&t!("dash.select_window"), {
        let cx = cx.clone();
        move |idx| {
            let Some(i) = idx else {
                return;
            };
            cx.with_mut(|ui| {
                let Some(new_hwnd) = ui.windows.get(i).map(|w| w.hwnd) else {
                    return;
                };
                let old_hwnd = ui.selected_idx.and_then(|j| ui.windows.get(j).map(|w| w.hwnd));
                ui.selected_idx = Some(i);
                if let Some(w) = ui.windows.get(i).cloned() {
                    {
                        let mut s = ui.state.write();
                        s.selected_hwnd = Some(w.hwnd);
                        s.selected_title = Some(w.title.clone());
                    }
                    let _ = ui.cmd_tx.send(PipelineCommand::SetSelectedWindow);
                }
                let had_regions = {
                    let s = ui.state.read();
                    !s.ocr_regions.is_empty() || s.region_select_active
                };
                if old_hwnd != Some(new_hwnd) && had_regions {
                    let _ = ui.cmd_tx.send(PipelineCommand::SetCaptureRegions { regions: Vec::new() });
                }
            });
        }
    })?;
    picker.cb.width(320.0)?.min_width(280.0)?.valign(VerticalAlignment::Center)?;

    let refresh = icon_button("\u{E72C}", {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                ui.windows = list_windows().unwrap_or_default();
                if ui.selected_idx.is_some_and(|i| i >= ui.windows.len()) {
                    ui.selected_idx = None;
                    {
                        let mut s = ui.state.write();
                        s.selected_hwnd = None;
                        s.selected_title = None;
                    }
                    let _ = ui.cmd_tx.send(PipelineCommand::SetSelectedWindow);
                }
            });
        }
    })?;
    refresh.min_width(40.0)?;

    let label = text(&t!("dash.window"))?;
    label.SetFontWeight(SEMI_BOLD)?;
    label.valign(VerticalAlignment::Center)?;
    let line = row(8.0)?;
    line.valign(VerticalAlignment::Center)?.halign(HorizontalAlignment::Left)?;
    children!(line; label, picker.cb, refresh);

    let mut running = Prop::default();
    d.add(move |snap, _| {
        let empty = snap.window_labels.is_empty();
        let items = if empty {
            vec![t!("dash.no_windows").into_owned()]
        } else {
            snap.window_labels.clone()
        };
        picker.set(items, snap.selected_idx, !empty && !snap.auto_running)?;
        running.set(snap.auto_running, |&r| {
            refresh.SetIsEnabled(!r)?;
            refresh.tooltip(&if r {
                t!("dash.stop_to_change_window")
            } else {
                t!("dash.refresh_windows")
            })?;
            Ok(())
        })
    });
    Ok(line)
}

fn build_regions_pane(b: &mut Binds<UiShared>, d: &mut Binds<DashSnap>, cx: &UiCx) -> Result<StackPanel> {
    let preset_combo = named_combo(b, cx, &REGION_PRESETS, &t!("dash.select_preset"))?;
    let save = button(&t!("action.save"), {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                let regions = sanitize_regions(&ui.state.read().ocr_regions);
                if regions.is_empty() {
                    ui.state.write().set_error(t!("err.regions_required"));
                    return;
                }
                ui.pending_save_regions = regions;
                ui.presets.name_draft = selected_preset(ui).map(|p| p.name.clone()).unwrap_or_default();
                ui.presets.dialog = NamedDialog::SaveName;
            });
        }
    })?;
    let load = button(&t!("action.load"), {
        let cx = cx.clone();
        move || {
            let regions = selected_preset(&cx.shared.lock()).map(|p| p.regions.clone());
            if let Some(regions) = regions {
                cx.send_cmd(PipelineCommand::SetCaptureRegions { regions });
            }
        }
    })?;
    load.tooltip(&t!("dash.load_preset_tip"))?;
    let delete = named_delete_button(b, cx, &REGION_PRESETS, &t!("dash.delete_preset_tip"))?;
    let sep = rule()?;
    sep.width(1.0)?.height(24.0)?.valign(VerticalAlignment::Center)?;

    let select_label = text("")?;
    let select = Button::new()?;
    select.SetContent(&select_label)?;
    click(&select, {
        let cx = cx.clone();
        move || {
            let (selecting, hwnd) = {
                let ui = cx.shared.lock();
                let selected = ui.selected_idx.and_then(|i| ui.windows.get(i).map(|w| w.hwnd));
                let s = ui.state.read();
                (s.region_select_active, if s.auto_running { s.target_hwnd } else { selected })
            };
            if selecting {
                cx.send_cmd(PipelineCommand::ConfirmRegionSelect);
            } else if let Some(hwnd) = hwnd {
                cx.send_cmd(PipelineCommand::BeginRegionSelect { hwnd });
            }
        }
    })?;
    let clear = button(&t!("action.clear"), {
        let cx = cx.clone();
        move || {
            if cx.shared.lock().state.read().region_select_active {
                cx.send_cmd(PipelineCommand::ClearRegionSelect);
            } else {
                cx.send_cmd(PipelineCommand::SetCaptureRegions { regions: Vec::new() });
            }
        }
    })?;
    clear.tooltip(&t!("dash.clear_regions_tip"))?;
    let regions_label = muted("")?;
    regions_label.valign(VerticalAlignment::Center)?;

    let toolbar = row(8.0)?;
    toolbar.valign(VerticalAlignment::Center)?.halign(HorizontalAlignment::Left)?;
    children!(toolbar; preset_combo, save, load, delete, sep, select, clear, regions_label);

    let mut state = Prop::default();
    d.add(move |snap, _| {
        let select_hwnd = if snap.auto_running { snap.target_hwnd } else { snap.selected_hwnd };
        let can_select = select_hwnd.is_some();
        let selecting = snap.region_select_active;
        let count = snap.preview_regions.len();
        let label = if selecting {
            t!("dash.selecting")
        } else if count == 0 {
            t!("dash.whole_window")
        } else {
            t!("dash.regions_selected", count = count)
        };
        let save_tip = if selecting {
            t!("dash.finish_select")
        } else if count == 0 {
            t!("dash.select_regions_first")
        } else {
            t!("dash.save_preset_tip")
        };
        let select_tip = if selecting {
            t!("dash.use_areas")
        } else if can_select {
            t!("dash.only_recognize")
        } else {
            t!("capture.select_window_first")
        };
        let key = (
            selecting,
            can_select,
            count,
            snap.presets.selected().is_some(),
            label.into_owned(),
            save_tip.into_owned(),
            select_tip.into_owned(),
        );
        state.set(key, |(selecting, can_select, count, preset_selected, label, save_tip, select_tip)| {
            save.SetIsEnabled(!selecting && *count > 0)?;
            save.tooltip(save_tip)?;
            load.SetIsEnabled(*preset_selected)?;
            select_label.SetText(&hs(&if *selecting { t!("action.done") } else { t!("dash.select_regions") }))?;
            select.SetIsEnabled(*selecting || *can_select)?;
            button_style(&select, if *selecting { ButtonKind::Accent } else { ButtonKind::Default })?;
            select.tooltip(select_tip)?;
            clear.SetIsEnabled(*selecting || *count > 0)?;
            regions_label.SetText(&hs(label))?;
            Ok(())
        })
    });

    let heading = text(&t!("dash.regions"))?;
    heading.SetFontWeight(SEMI_BOLD)?;
    let help = muted(&t!("dash.regions_help"))?;
    help.SetTextWrapping(TextWrapping::WrapWholeWords)?;
    let pane = column(8.0)?;
    pane.halign(HorizontalAlignment::Stretch)?;
    let name_row = name_entry_row(b, cx, &REGION_PRESETS)?;
    children!(pane; heading, toolbar, name_row, help);
    Ok(pane)
}
