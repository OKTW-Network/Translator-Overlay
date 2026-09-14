//! Operator workspace: session settings above a divider, then preview | results.

use std::{mem, sync::Arc};

use parking_lot::Mutex;
use translator_capture::list_windows;
use translator_core::{HistoryEntry, NormRect, PreviewInfo, sanitize_regions};
use windows_reactor::{
    Border, Button, ButtonStyle, ChildrenControl, ComboBox, ContentControl, ContentDialog, ContentDialogResult, FontIcon, FontWeight, Grid,
    GridChildExt, GridLength, HorizontalAlignment, KeyedView, LayoutControl, LocalSender, Orientation, ScrollViewer, StackPanel, TextBlock,
    TextBox, TextWrapping, ThemeBrush, Thickness, TooltipExt, VerticalAlignment, View,
};

use crate::{
    pipeline::PipelineCommand,
    ui::{
        chrome::status_infobar,
        preview::capture_preview,
        shared::{AppMsg, ChromeSnap, PresetDialog, UiCx, UiShared, commit_pending_preset, save_region_presets, selected_preset, truncate},
    },
};

struct DashSnap {
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
    preset_names: Vec<String>,
    preset_selected_idx: i32,
    preset_name_draft: String,
    preset_dialog: PresetDialog,
}

fn take_dash(shared: &Arc<Mutex<UiShared>>) -> DashSnap {
    let ui = shared.lock();
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
        ocr_text: s.latest_ocr_blocks.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join("\n"),
        translation: s
            .latest_translated_blocks
            .iter()
            .map(|b| b.translation.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        history: s.history.iter().cloned().collect(),
        selected_idx: ui.selected_idx.filter(|&i| i < ui.windows.len()),
        selected_hwnd: ui.selected_idx.and_then(|i| ui.windows.get(i).map(|w| w.hwnd)),
        target_hwnd: s.target_hwnd,
        window_labels: ui.windows.iter().map(|w| truncate(&w.title, 72)).collect(),
        region_select_active: s.region_select_active,
        preset_names: ui.region_presets.iter().map(|p| p.name.clone()).collect(),
        preset_selected_idx: ui.preset_selected_idx,
        preset_name_draft: ui.preset_name_draft.clone(),
        preset_dialog: ui.preset_dialog.clone(),
    }
}

fn results_pair(left: impl Into<View>, right: impl Into<View>) -> View {
    StackPanel::new()
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .children((
            Grid::new()
                .columns([GridLength::Star(1.0), GridLength::Auto, GridLength::Star(1.0)])
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .children((
                    Border::new()
                        .grid_column(0)
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .vertical_alignment(VerticalAlignment::Top)
                        .margin(Thickness::new(0.0, 0.0, 8.0, 0.0))
                        .content(left),
                    Border::new()
                        .background(ThemeBrush::CardStroke)
                        .width(1.0)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Stretch)
                        .grid_column(1),
                    Border::new()
                        .grid_column(2)
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .vertical_alignment(VerticalAlignment::Top)
                        .margin(Thickness::new(8.0, 0.0, 0.0, 0.0))
                        .content(right),
                )),
            Border::new()
                .background(ThemeBrush::CardStroke)
                .height(1.0)
                .horizontal_alignment(HorizontalAlignment::Stretch),
        ))
}

fn results_table_row(source: String, translation: String, opacity: f64) -> View {
    results_pair(
        TextBlock::new()
            .text(source)
            .opacity(opacity)
            .text_wrapping(TextWrapping::Wrap)
            .is_text_selection_enabled(true),
        TextBlock::new()
            .text(translation)
            .opacity(opacity)
            .text_wrapping(TextWrapping::Wrap)
            .is_text_selection_enabled(true),
    )
}

pub fn dashboard_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let mut snap = take_dash(shared);
    let window_labels = mem::take(&mut snap.window_labels);
    let mut history = mem::take(&mut snap.history);
    let in_flight = snap.translate_in_flight;
    let ocr_time = snap.last_ocr_ms.map(|ms| format!("{ms} ms")).unwrap_or_else(|| "—".into());
    let ocr_desc = format!("{ocr_time} · {} blocks", snap.last_ocr_block_count);
    let live_src = mem::take(&mut snap.ocr_text);
    let live_dst = mem::take(&mut snap.translation);
    let has_live = !live_src.is_empty() || !live_dst.is_empty();
    if has_live && !in_flight && history.first().is_some_and(|h| h.covered_by(&live_src, &live_dst)) {
        history.remove(0);
    }

    let divider = Border::new()
        .background(ThemeBrush::CardStroke)
        .height(1.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .grid_row(1)
        .grid_column(0)
        .margin(Thickness::new(0.0, 12.0, 0.0, 16.0));

    let settings = StackPanel::new()
        .spacing(12.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .grid_row(0)
        .grid_column(0)
        .vertical_alignment(VerticalAlignment::Top)
        .children((
            TextBlock::new().text("Dashboard").font_size(28.0).font_weight(FontWeight::BOLD),
            status_infobar(chrome),
            build_window_row(&cx, &snap, window_labels),
            build_regions_pane(&cx, &snap),
            preset_confirm_dialog(&cx, &snap),
        ));

    let results_empty = !has_live && history.is_empty();
    let results_header: View = if results_empty {
        View::empty()
    } else {
        results_pair(
            TextBlock::new()
                .text("Source")
                .font_size(12.0)
                .font_weight(FontWeight::SEMI_BOLD)
                .foreground(ThemeBrush::PrimaryText)
                .opacity(0.72),
            TextBlock::new()
                .text("Translation")
                .font_size(12.0)
                .font_weight(FontWeight::SEMI_BOLD)
                .foreground(ThemeBrush::PrimaryText)
                .opacity(0.72),
        )
    };
    let results_rows: View = if results_empty {
        TextBlock::new()
            .text("(no history yet)")
            .foreground(ThemeBrush::PrimaryText)
            .opacity(0.72)
            .into()
    } else {
        StackPanel::new()
            .spacing(8.0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .keyed_children(
                has_live
                    .then(|| KeyedView::new("live", results_table_row(live_src, live_dst, 1.0)))
                    .into_iter()
                    .chain(
                        history
                            .into_iter()
                            .map(|h| KeyedView::new(h.id, results_table_row(h.source_text, h.translated_text, 0.72))),
                    ),
            )
    };

    let retry_tip = if in_flight {
        "Cancel the translation in progress first"
    } else if snap.auto_running {
        "Capture now and run OCR + translation"
    } else {
        "Start capture first"
    };

    let actions = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Left)
        .children((
            Button::new()
                .is_enabled(in_flight)
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::CancelTranslate)
                })
                .content("Cancel")
                .tooltip(if in_flight {
                    "Cancel the translation in progress"
                } else {
                    "No translation in progress"
                }),
            Button::new()
                .is_enabled(snap.auto_running && !in_flight)
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::ManualCapture)
                })
                .content("Retry")
                .tooltip(retry_tip),
            Button::new()
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::ResetConversation)
                })
                .content("Clear chat")
                .tooltip("Clear the translation model conversation history"),
        ));

    let preview_pane = StackPanel::new()
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Stretch)
        .grid_row(0)
        .grid_column(0)
        .margin(Thickness::new(0.0, 0.0, 16.0, 0.0))
        .children((
            actions,
            capture_preview(
                snap.preview.sequence,
                snap.preview.width,
                snap.preview.height,
                snap.preview.rgba.as_ref(),
                &snap.preview_regions,
            ),
        ));

    let output = Grid::new()
        .rows([GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Stretch)
        .grid_row(0)
        .grid_column(1)
        .children((
            StackPanel::new()
                .spacing(8.0)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .grid_row(0)
                .grid_column(0)
                .children((
                    Grid::new()
                        .columns([GridLength::Star(1.0), GridLength::Auto])
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .children((
                            TextBlock::new()
                                .text("OCR")
                                .font_size(14.0)
                                .font_weight(FontWeight::SEMI_BOLD)
                                .vertical_alignment(VerticalAlignment::Center)
                                .grid_column(0),
                            TextBlock::new()
                                .text(ocr_desc)
                                .font_size(12.0)
                                .foreground(ThemeBrush::PrimaryText)
                                .opacity(0.72)
                                .vertical_alignment(VerticalAlignment::Center)
                                .grid_column(1),
                        )),
                    results_header,
                )),
            ScrollViewer::new()
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .vertical_alignment(VerticalAlignment::Stretch)
                .margin(Thickness::new(0.0, 8.0, 0.0, 0.0))
                .grid_row(1)
                .grid_column(0)
                .content(results_rows),
        ));

    let workspace = Grid::new()
        .rows([GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0), GridLength::Star(1.0)])
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Stretch)
        .grid_row(2)
        .grid_column(0)
        .children((preview_pane, output));

    Grid::new()
        .rows([GridLength::Auto, GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Stretch)
        .children((settings, divider, workspace))
}

fn build_window_row(cx: &UiCx, snap: &DashSnap, window_labels: Vec<String>) -> View {
    let window_empty = window_labels.is_empty();
    let window_items: Vec<String> = if window_empty {
        vec!["(no windows — refresh)".into()]
    } else {
        window_labels
    };

    let picker = ComboBox::new()
        .items_source(window_items)
        .selected_index(snap.selected_idx)
        .placeholder_text("Select window…")
        .is_enabled(!window_empty && !snap.auto_running)
        .on_selection_changed({
            let cx = cx.clone();
            move |idx: Option<usize>| {
                let Some(i) = idx else {
                    return;
                };
                cx.with_mut(|ui| {
                    let Some(new_hwnd) = ui.windows.get(i).map(|w| w.hwnd) else {
                        return;
                    };
                    let old_hwnd = ui.selected_idx.and_then(|j| ui.windows.get(j).map(|w| w.hwnd));
                    ui.selected_idx = Some(i);
                    let had_regions = {
                        let s = ui.state.read();
                        !s.ocr_regions.is_empty() || s.region_select_active
                    };
                    if old_hwnd != Some(new_hwnd) && had_regions {
                        let _ = ui.cmd_tx.send(PipelineCommand::SetCaptureRegions { regions: Vec::new() });
                    }
                });
            }
        })
        .width(320.0)
        .min_width(280.0)
        .vertical_alignment(VerticalAlignment::Center);

    let refresh = Button::new()
        .style(ButtonStyle::Subtle)
        .min_width(40.0)
        .min_height(36.0)
        .vertical_alignment(VerticalAlignment::Center)
        .is_enabled(!snap.auto_running)
        .on_click({
            let cx = cx.clone();
            move || {
                cx.with_mut(|ui| {
                    ui.windows = list_windows().unwrap_or_default();
                    if ui.selected_idx.is_some_and(|i| i >= ui.windows.len()) {
                        ui.selected_idx = None;
                    }
                });
            }
        })
        .content(FontIcon::new().glyph("\u{E72C}"))
        .tooltip(if snap.auto_running {
            "Stop capture to change window"
        } else {
            "Refresh window list"
        });

    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Left)
        .children((
            TextBlock::new()
                .text("Window")
                .font_weight(FontWeight::SEMI_BOLD)
                .vertical_alignment(VerticalAlignment::Center),
            picker,
            refresh,
        ))
}

fn build_regions_pane(cx: &UiCx, snap: &DashSnap) -> View {
    let select_hwnd = if snap.auto_running { snap.target_hwnd } else { snap.selected_hwnd };
    let can_select = select_hwnd.is_some();
    let selecting = snap.region_select_active;
    let region_count = snap.preview_regions.len();
    let regions_label = if selecting {
        "Selecting…".to_string()
    } else if region_count == 0 {
        "Whole window".into()
    } else {
        format!("{region_count} selected")
    };
    let has_presets = !snap.preset_names.is_empty();
    let preset_selected = has_presets && snap.preset_selected_idx >= 0;
    let can_save = !snap.region_select_active && region_count > 0;

    let select_label = if selecting { "Done" } else { "Select regions" };
    let select_tip = if selecting {
        "Use the selected areas"
    } else if can_select {
        "Only recognize text in the areas you select on the window"
    } else {
        "Select a window first"
    };
    let mut select_btn = Button::new().is_enabled(selecting || can_select);
    if selecting {
        select_btn = select_btn.style(ButtonStyle::Accent);
    }
    let select_btn = select_btn
        .on_click({
            let cx = cx.clone();
            let hwnd = select_hwnd;
            move || {
                if selecting {
                    cx.send_cmd(PipelineCommand::ConfirmRegionSelect);
                } else if let Some(hwnd) = hwnd {
                    cx.send_cmd(PipelineCommand::BeginRegionSelect { hwnd });
                }
            }
        })
        .content(select_label)
        .tooltip(select_tip);

    let preset_idx = if has_presets && snap.preset_selected_idx >= 0 {
        Some(snap.preset_selected_idx as usize)
    } else {
        None
    };
    let preset_combo = ComboBox::new()
        .items_source(snap.preset_names.clone())
        .selected_index(preset_idx)
        .placeholder_text("Select preset…")
        .is_enabled(has_presets)
        .on_selection_changed({
            let cx = cx.clone();
            move |idx: Option<usize>| {
                cx.with_mut(|ui| {
                    ui.preset_selected_idx = idx.filter(|&i| i < ui.region_presets.len()).map(|i| i as i32).unwrap_or(-1);
                });
            }
        })
        .width(180.0)
        .min_width(140.0)
        .vertical_alignment(VerticalAlignment::Center);

    let sep = Border::new()
        .background(ThemeBrush::CardStroke)
        .width(1.0)
        .height(24.0)
        .vertical_alignment(VerticalAlignment::Center);

    let toolbar = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Left)
        .children((
            preset_combo,
            Button::new()
                .is_enabled(can_save)
                .on_click({
                    let cx = cx.clone();
                    move || {
                        cx.with_mut(|ui| {
                            let regions = sanitize_regions(&ui.state.read().ocr_regions);
                            if regions.is_empty() {
                                ui.state.write().set_error("Nothing to save: select at least one OCR region.");
                                return;
                            }
                            ui.pending_save_regions = regions;
                            ui.preset_name_draft = selected_preset(ui).map(|p| p.name.clone()).unwrap_or_default();
                            ui.preset_dialog = PresetDialog::SaveName;
                        });
                    }
                })
                .content("Save")
                .tooltip(if snap.region_select_active {
                    "Finish region select (Done) before saving a preset"
                } else if region_count == 0 {
                    "Select OCR regions first"
                } else {
                    "Save current OCR regions as a named preset"
                }),
            Button::new()
                .is_enabled(preset_selected)
                .on_click({
                    let cx = cx.clone();
                    move || {
                        let regions = selected_preset(&cx.shared.lock()).map(|p| p.regions.clone());
                        if let Some(regions) = regions {
                            cx.send_cmd(PipelineCommand::SetCaptureRegions { regions });
                        }
                    }
                })
                .content("Load")
                .tooltip("Apply the selected preset to the current window"),
            Button::new()
                .is_enabled(preset_selected)
                .on_click({
                    let cx = cx.clone();
                    move || {
                        cx.with_mut(|ui| {
                            let Some(name) = selected_preset(ui).map(|p| p.name.clone()) else {
                                return;
                            };
                            ui.preset_dialog = PresetDialog::Delete { name };
                        });
                    }
                })
                .content("Delete")
                .tooltip("Delete the selected preset"),
            sep,
            select_btn,
            Button::new()
                .is_enabled(selecting || region_count > 0)
                .on_click({
                    let cx = cx.clone();
                    move || {
                        if selecting {
                            cx.send_cmd(PipelineCommand::ClearRegionSelect);
                        } else {
                            cx.send_cmd(PipelineCommand::SetCaptureRegions { regions: Vec::new() });
                        }
                    }
                })
                .content("Clear")
                .tooltip("Recognize text on the whole window"),
            TextBlock::new()
                .text(regions_label)
                .font_size(12.0)
                .foreground(ThemeBrush::PrimaryText)
                .opacity(0.72)
                .vertical_alignment(VerticalAlignment::Center),
        ));

    StackPanel::new()
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .children((
            TextBlock::new().text("Regions").font_weight(FontWeight::SEMI_BOLD),
            toolbar,
            preset_name_row(cx, snap),
            TextBlock::new()
                .text("Drag on the selected window to choose what to translate — drag to move or resize, right-click to remove.")
                .font_size(12.0)
                .foreground(ThemeBrush::PrimaryText)
                .opacity(0.72)
                .text_wrapping(TextWrapping::WrapWholeWords),
        ))
}

fn preset_name_row(cx: &UiCx, snap: &DashSnap) -> View {
    if !matches!(snap.preset_dialog, PresetDialog::SaveName) {
        return View::empty();
    }

    let name_tb = TextBox::new()
        .text(snap.preset_name_draft.clone())
        .on_text_changed({
            let cx = cx.clone();
            move |text: String| {
                cx.with_mut(|ui| ui.preset_name_draft = text);
            }
        })
        .width(180.0)
        .vertical_alignment(VerticalAlignment::Center);

    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(4.0)
        .padding(Thickness::new(8.0, 4.0, 8.0, 4.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .content(
            StackPanel::new().orientation(Orientation::Horizontal).spacing(8.0).children((
                TextBlock::new()
                    .text("Name")
                    .font_weight(FontWeight::SEMI_BOLD)
                    .vertical_alignment(VerticalAlignment::Center),
                name_tb,
                Button::new()
                    .style(ButtonStyle::Accent)
                    .on_click({
                        let cx = cx.clone();
                        move || {
                            cx.with_mut(|ui| {
                                let name = ui.preset_name_draft.trim().to_string();
                                if name.is_empty() {
                                    ui.state.write().set_error("Preset name is required.");
                                    return;
                                }
                                if ui.region_presets.iter().any(|p| p.name == name) {
                                    ui.preset_dialog = PresetDialog::Overwrite { name };
                                    return;
                                }
                                if let Err(e) = commit_pending_preset(ui, name) {
                                    ui.state.write().set_error(e);
                                }
                            });
                        }
                    })
                    .content("Save"),
                Button::new()
                    .on_click({
                        let cx = cx.clone();
                        move || {
                            cx.with_mut(|ui| {
                                ui.preset_dialog = PresetDialog::None;
                                ui.pending_save_regions.clear();
                                ui.preset_name_draft.clear();
                            });
                        }
                    })
                    .content("Cancel"),
            )),
        )
}

fn preset_confirm_dialog(cx: &UiCx, snap: &DashSnap) -> View {
    let (open, title, body, primary) = match &snap.preset_dialog {
        PresetDialog::Overwrite { name } => (true, "Overwrite preset?", format!("Replace the regions saved in \"{name}\"?"), "Overwrite"),
        PresetDialog::Delete { name } => (true, "Delete preset?", format!("Delete preset \"{name}\"? This cannot be undone."), "Delete"),
        PresetDialog::None | PresetDialog::SaveName => (false, "", String::new(), "OK"),
    };

    ContentDialog::new()
        .title(title)
        .primary_button_text(primary)
        .close_button_text("Cancel")
        .is_open(open)
        .on_closed({
            let cx = cx.clone();
            move |result: ContentDialogResult| {
                cx.with_mut(|ui| {
                    let action = ui.preset_dialog.clone();
                    ui.preset_dialog = PresetDialog::None;
                    if result != ContentDialogResult::Primary {
                        if matches!(action, PresetDialog::Overwrite { .. }) {
                            ui.preset_dialog = PresetDialog::SaveName;
                        }
                        return;
                    }
                    match action {
                        PresetDialog::Overwrite { name } => {
                            if let Err(e) = commit_pending_preset(ui, name) {
                                ui.state.write().set_error(e);
                            }
                        }
                        PresetDialog::Delete { name } => {
                            ui.region_presets.retain(|p| p.name != name);
                            ui.preset_selected_idx = -1;
                            ui.preset_name_draft.clear();
                            if let Err(e) = save_region_presets(ui) {
                                ui.state.write().set_error(e);
                            }
                        }
                        PresetDialog::None | PresetDialog::SaveName => {}
                    }
                });
            }
        })
        .content(body)
}
