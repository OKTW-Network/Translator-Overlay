//! Operator workspace: session settings above a divider, then preview | results.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_capture::list_windows;
use translator_core::sanitize_regions;
use windows_reactor::{
    BackgroundExt, ComboBox, ContentDialog, ContentDialogResult, Element, GridChildExt, GridLength, HorizontalAlignment, KeyExt, LayoutExt,
    PaddingExt, StackPanel, TextStyleExt, ThemeRef, Thickness, TooltipExt, Updater, VerticalAlignment, border, button, grid, hstack,
    scroll_viewer, text_block, text_box, vstack,
};

use crate::{
    pipeline::PipelineCommand,
    ui::{
        chrome::{page_header, status_infobar},
        preview::capture_preview,
        shared::{PresetDialog, Snapshot, UiCx, UiShared, commit_pending_preset, selected_preset},
    },
};

pub fn dashboard_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> Element {
    let cx = UiCx::new(shared, bump);
    let in_flight = snap.translate_in_flight;
    let ocr_time = snap.last_ocr_ms.map(|ms| format!("{ms} ms")).unwrap_or_else(|| "—".into());
    let ocr_desc = format!("Last OCR: {ocr_time}  ·  {} blocks", snap.last_ocr_block_count);

    let divider = border(Element::Empty)
        .background(ThemeRef::DividerStroke)
        .height(1.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .with_key("dash-divider");

    let settings = vstack((
        page_header("Dashboard", None),
        status_infobar(snap),
        build_window_row(&cx, snap),
        build_regions_pane(&cx, snap),
        preset_confirm_dialog(&cx, snap),
    ))
    .spacing(12.0)
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .with_key("dash-settings");

    let retry_tip = if in_flight {
        "Cancel the translation in progress first"
    } else if snap.auto_running {
        "Capture now and run OCR + translation"
    } else {
        "Start capture first"
    };

    let actions = hstack((
        button("Cancel")
            .tooltip(if in_flight {
                "Cancel the translation in progress"
            } else {
                "No translation in progress"
            })
            .enabled(in_flight)
            .on_click({
                let cx = cx.clone();
                move || cx.send_cmd(PipelineCommand::CancelTranslate)
            }),
        button("Retry")
            .tooltip(retry_tip)
            .enabled(snap.auto_running && !in_flight)
            .on_click({
                let cx = cx.clone();
                move || cx.send_cmd(PipelineCommand::ManualCapture)
            }),
        button("Clear chat")
            .tooltip("Clear the translation model conversation history")
            .on_click({
                let cx = cx.clone();
                move || cx.send_cmd(PipelineCommand::ResetConversation)
            }),
    ))
    .spacing(8.0)
    .horizontal_alignment(HorizontalAlignment::Left)
    .with_key("dash-actions");

    let preview_pane = vstack((
        actions,
        capture_preview(
            snap.preview_sequence,
            snap.preview_width,
            snap.preview_height,
            snap.preview_rgba.as_ref(),
            &snap.preview_regions,
            bump,
        ),
    ))
    .spacing(8.0)
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .vertical_alignment(VerticalAlignment::Stretch)
    .with_key("dash-preview-pane");

    let output = scroll_viewer(
        vstack((
            hstack((
                text_block("OCR").semibold(),
                text_block(ocr_desc)
                    .font_size(12.0)
                    .foreground(ThemeRef::SecondaryText)
                    .vertical_alignment(VerticalAlignment::Center),
            ))
            .spacing(12.0),
            text_block(snap.ocr_text.clone()).wrap().selectable(),
            text_block("Translation").semibold(),
            text_block(snap.translation.clone()).wrap().selectable(),
            text_block("Recent").semibold(),
            text_block(snap.history_preview.clone()).wrap().selectable(),
        ))
        .spacing(4.0)
        .horizontal_alignment(HorizontalAlignment::Stretch),
    )
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .vertical_alignment(VerticalAlignment::Stretch)
    .with_key("dash-output");

    let workspace = grid((
        preview_pane
            .grid_row(0)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Stretch)
            .margin(Thickness {
                left: 0.0,
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
            }),
        output
            .grid_row(0)
            .grid_column(1)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Stretch),
    ))
    .rows([GridLength::Star(1.0)])
    .columns([GridLength::Star(1.0), GridLength::Star(1.0)])
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .vertical_alignment(VerticalAlignment::Stretch)
    .with_key("dash-workspace");

    grid((
        settings
            .grid_row(0)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Top),
        divider
            .grid_row(1)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .margin(Thickness {
                left: 0.0,
                top: 12.0,
                right: 0.0,
                bottom: 16.0,
            }),
        workspace
            .grid_row(2)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Stretch),
    ))
    .rows([GridLength::Auto, GridLength::Auto, GridLength::Star(1.0)])
    .columns([GridLength::Star(1.0)])
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .vertical_alignment(VerticalAlignment::Stretch)
    .into()
}

fn build_window_row(cx: &UiCx, snap: &Snapshot) -> StackPanel {
    let window_selected = snap.selected_window_idx;
    let window_items: Vec<String> = if snap.window_labels.is_empty() {
        vec!["(no windows — refresh)".into()]
    } else {
        snap.window_labels.clone()
    };

    // ComboBox popup opens below the control (MenuFlyout on DropDownButton
    // defaults to Top placement and often expands upward).
    let mut picker = ComboBox::new(window_items)
        .selected_index(window_selected)
        .placeholder_text("Select window…")
        .enabled(snap.window_count > 0 && !snap.auto_running)
        .on_selection_changed({
            let cx = cx.clone();
            move |idx: i32| {
                if idx < 0 {
                    return;
                }
                cx.with_mut(|ui| {
                    let i = idx as usize;
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
        .with_key("window-combo");
    picker.modifiers.width = Some(320.0);
    picker.modifiers.min_width = Some(280.0);
    picker.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    let refresh = button("\u{E72C}")
        .font_family("Segoe MDL2 Assets")
        .font_size(16.0)
        .subtle()
        .padding(Thickness::uniform(8.0))
        .min_width(40.0)
        .min_height(36.0)
        .vertical_alignment(VerticalAlignment::Center)
        .tooltip(if snap.auto_running {
            "Stop capture to change window"
        } else {
            "Refresh window list"
        })
        .enabled(!snap.auto_running)
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
        });

    hstack((text_block("Window").semibold().vertical_alignment(VerticalAlignment::Center), picker, refresh))
        .spacing(8.0)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Left)
        .with_key("dash-window-row")
}

fn build_regions_pane(cx: &UiCx, snap: &Snapshot) -> StackPanel {
    let select_hwnd = if snap.auto_running { snap.target_hwnd } else { snap.selected_hwnd };
    let can_select = select_hwnd.is_some();
    let selecting = snap.region_select_active;
    let regions_label = if selecting {
        "Selecting…".to_string()
    } else if snap.ocr_region_count == 0 {
        "Whole window".into()
    } else {
        format!("{} selected", snap.ocr_region_count)
    };
    let has_presets = !snap.preset_names.is_empty();
    let preset_selected = has_presets && snap.preset_selected_idx >= 0;
    let can_save = !snap.region_select_active && snap.ocr_region_count > 0;

    // Distinct keys remount so Accent does not stick after Done
    // (`Button::accent()` cannot be cleared via Prop Unset).
    let select_label = if selecting { "Done" } else { "Select regions" };
    let select_key = if selecting { "btn-done-regions" } else { "btn-select-regions" };
    let select_tip = if selecting {
        "Use the selected areas"
    } else if can_select {
        "Only recognize text in the areas you select on the window"
    } else {
        "Select a window first"
    };
    let mut select_btn = button(select_label)
        .tooltip(select_tip)
        .enabled(selecting || can_select)
        .with_key(select_key);
    if selecting {
        select_btn = select_btn.accent();
    }
    let select_btn = select_btn.on_click({
        let cx = cx.clone();
        let hwnd = select_hwnd;
        move || {
            if selecting {
                cx.send_cmd(PipelineCommand::ConfirmRegionSelect);
            } else if let Some(hwnd) = hwnd {
                cx.send_cmd(PipelineCommand::BeginRegionSelect { hwnd });
            }
        }
    });

    let mut preset_combo = ComboBox::new(snap.preset_names.clone())
        .selected_index(if has_presets { snap.preset_selected_idx } else { -1 })
        .placeholder_text("Select preset…")
        .enabled(has_presets)
        .on_selection_changed({
            let cx = cx.clone();
            move |idx: i32| {
                cx.with_mut(|ui| {
                    ui.preset_selected_idx = if idx >= 0 && (idx as usize) < ui.region_presets.len() {
                        idx
                    } else {
                        -1
                    };
                });
            }
        })
        .with_key("preset-combo");
    preset_combo.modifiers.width = Some(180.0);
    preset_combo.modifiers.min_width = Some(140.0);
    preset_combo.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    let sep = border(Element::Empty)
        .background(ThemeRef::DividerStroke)
        .width(1.0)
        .height(24.0)
        .vertical_alignment(VerticalAlignment::Center)
        .with_key("region-row-sep");

    let toolbar = hstack((
        preset_combo,
        button("Save")
            .tooltip(if snap.region_select_active {
                "Finish region select (Done) before saving a preset"
            } else if snap.ocr_region_count == 0 {
                "Select OCR regions first"
            } else {
                "Save current OCR regions as a named preset"
            })
            .enabled(can_save)
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
            }),
        button("Load")
            .tooltip("Apply the selected preset to the current window")
            .enabled(preset_selected)
            .on_click({
                let cx = cx.clone();
                move || {
                    let regions = selected_preset(&cx.shared.lock()).map(|p| p.regions.clone());
                    if let Some(regions) = regions {
                        cx.send_cmd(PipelineCommand::SetCaptureRegions { regions });
                    }
                }
            }),
        button("Delete")
            .tooltip("Delete the selected preset")
            .enabled(preset_selected)
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
            }),
        sep,
        select_btn,
        button("Clear")
            .tooltip("Recognize text on the whole window")
            .enabled(selecting || snap.ocr_region_count > 0)
            .on_click({
                let cx = cx.clone();
                move || {
                    if selecting {
                        cx.send_cmd(PipelineCommand::ClearRegionSelect);
                    } else {
                        cx.send_cmd(PipelineCommand::SetCaptureRegions { regions: Vec::new() });
                    }
                }
            }),
        text_block(regions_label)
            .font_size(12.0)
            .foreground(ThemeRef::SecondaryText)
            .vertical_alignment(VerticalAlignment::Center),
    ))
    .spacing(8.0)
    .vertical_alignment(VerticalAlignment::Center)
    .horizontal_alignment(HorizontalAlignment::Left)
    .with_key("region-toolbar-row");

    vstack((
        text_block("Regions").semibold(),
        toolbar,
        preset_name_row(cx, snap),
        text_block("Drag on the selected window to choose what to translate — drag to move or resize, right-click to remove.")
            .font_size(12.0)
            .foreground(ThemeRef::SecondaryText)
            .wrap(),
    ))
    .spacing(8.0)
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .with_key("dash-regions-pane")
}

fn preset_name_row(cx: &UiCx, snap: &Snapshot) -> Element {
    if !matches!(snap.preset_dialog, PresetDialog::SaveName) {
        return Element::Empty;
    }

    let mut name_tb = text_box(snap.preset_name_draft.clone()).on_text_changed({
        let cx = cx.clone();
        move |text: String| {
            cx.with_mut(|ui| ui.preset_name_draft = text);
        }
    });
    name_tb.modifiers.width = Some(180.0);
    name_tb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    border(
        hstack((
            text_block("Name").semibold().vertical_alignment(VerticalAlignment::Center),
            name_tb,
            button("Save").accent().on_click({
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
            }),
            button("Cancel").on_click({
                let cx = cx.clone();
                move || {
                    cx.with_mut(|ui| {
                        ui.preset_dialog = PresetDialog::None;
                        ui.pending_save_regions.clear();
                        ui.preset_name_draft.clear();
                    });
                }
            }),
        ))
        .spacing(8.0),
    )
    .background(ThemeRef::SubtleFill)
    .border_brush(ThemeRef::CardStroke)
    .border_thickness(Thickness::uniform(1.0))
    .corner_radius(4.0)
    .padding(Thickness {
        left: 8.0,
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
    })
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .with_key("preset-name-row")
    .into()
}

fn preset_confirm_dialog(cx: &UiCx, snap: &Snapshot) -> ContentDialog {
    let (open, title, body, primary) = match &snap.preset_dialog {
        PresetDialog::Overwrite { name } => (true, "Overwrite preset?", format!("Replace the regions saved in \"{name}\"?"), "Overwrite"),
        PresetDialog::Delete { name } => (true, "Delete preset?", format!("Delete preset \"{name}\"? This cannot be undone."), "Delete"),
        PresetDialog::None | PresetDialog::SaveName => (false, "", String::new(), "OK"),
    };

    ContentDialog::new(title)
        .content(body)
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
                            // Keep the inline name row open with pending regions.
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
                            if let Err(e) = crate::ui::shared::save_region_presets(ui) {
                                ui.state.write().set_error(e);
                            }
                        }
                        PresetDialog::None | PresetDialog::SaveName => {}
                    }
                });
            }
        })
        .with_key("preset-confirm-dialog")
}
