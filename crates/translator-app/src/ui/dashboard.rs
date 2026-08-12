//! Dashboard: capture control, window picker, live OCR / translation preview.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_capture::list_windows;
use windows_reactor::{
    ComboBox, HorizontalAlignment, KeyExt, LayoutExt, PaddingExt, StackPanel, TextStyleExt, ThemeRef, Thickness, TooltipExt, Updater,
    VerticalAlignment, button, hstack, text_block, vstack,
};

use crate::{
    pipeline::PipelineCommand,
    ui::{
        chrome::{app_status_strip, page_header, status_infobar},
        preview::capture_preview,
        shared::{Snapshot, UiCx, UiShared},
    },
};

pub fn dashboard_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);

    let window_selected = snap.selected_window_idx;
    let has_window = window_selected >= 0;
    let start_label = if snap.auto_running { "Stop" } else { "Start" };
    let start_tip = if snap.auto_running {
        "Stop continuous capture"
    } else if has_window {
        "Start continuous capture of the selected window"
    } else {
        "Select a window first"
    };
    let start_enabled = snap.auto_running || has_window;
    let once_tip = if snap.auto_running {
        "Capture once and translate immediately"
    } else {
        "Start capture first"
    };
    let in_flight = snap.translate_in_flight;
    let can_retry = snap.can_retry;

    // ComboBox popup opens below the control (MenuFlyout on DropDownButton
    // defaults to Top placement and often expands upward).
    let window_items: Vec<String> = if snap.window_labels.is_empty() {
        vec!["(no windows — refresh)".into()]
    } else {
        snap.window_labels.clone()
    };

    vstack((
        page_header("Dashboard", Some("Choose a window, capture text, and watch translations.")),
        app_status_strip(snap),
        status_infobar(snap),
        {
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
                            if i < ui.windows.len() {
                                ui.selected_idx = Some(i);
                            }
                        });
                    }
                })
                .with_key("window-combo");
            picker.modifiers.min_width = Some(280.0);
            picker.modifiers.horizontal_alignment = Some(HorizontalAlignment::Stretch);
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
                .with_key("window-picker-row")
        },
        hstack((
            {
                // Distinct keys remount so Accent style does not stick after Stop
                // (`Button::accent()` cannot be cleared via Prop Unset).
                let start_key = if snap.auto_running { "btn-stop" } else { "btn-start" };
                let mut start = button(start_label).tooltip(start_tip).enabled(start_enabled).with_key(start_key);
                if snap.auto_running {
                    start = start.accent();
                }
                start.on_click({
                    let cx = cx.clone();
                    move || {
                        {
                            let ui = cx.shared.lock();
                            if ui.state.read().auto_running {
                                let _ = ui.cmd_tx.send(PipelineCommand::StopCapture);
                            } else if let Some(w) = ui.selected_idx.and_then(|i| ui.windows.get(i)).cloned() {
                                let _ = ui.cmd_tx.send(PipelineCommand::StartCapture {
                                    hwnd: w.hwnd,
                                    title: w.title,
                                });
                            }
                        }
                        cx.refresh();
                    }
                })
            },
            button("Once").tooltip(once_tip).enabled(snap.auto_running).on_click({
                let cx = cx.clone();
                move || cx.send_cmd(PipelineCommand::ManualCapture)
            }),
            button("Clear chat")
                .tooltip("Clear the translation model conversation history")
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::ResetConversation)
                }),
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
                .tooltip("Retry the last failed translation")
                .enabled(can_retry && !in_flight)
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::RetryTranslate)
                }),
        ))
        .spacing(8.0),
        text_block(format!(
            "Model {}  ·  {} → {}  ·  OCR {}  ·  frames {}  ·  history {}",
            snap.model, snap.source_lang, snap.target_lang, snap.tier, snap.frame_count, snap.history_len
        ))
        .font_size(12.0)
        .foreground(ThemeRef::SecondaryText),
        text_block({
            let ocr_time = snap.last_ocr_ms.map(|ms| format!("{ms} ms")).unwrap_or_else(|| "—".into());
            format!("Last OCR: {ocr_time}  ·  {} blocks", snap.last_ocr_block_count)
        })
        .font_size(12.0)
        .foreground(ThemeRef::SecondaryText),
        vstack((
            text_block("Capture").semibold(),
            capture_preview(snap.preview_sequence, snap.preview_width, snap.preview_height, snap.preview_rgba.as_ref(), bump),
            text_block({
                let ocr_time = snap.last_ocr_ms.map(|ms| format!("{ms} ms")).unwrap_or_else(|| "—".into());
                format!("OCR ({ocr_time})")
            })
            .semibold(),
            text_block(snap.ocr_text.clone()).wrap().selectable(),
            text_block("Translation").semibold(),
            text_block(snap.translation.clone()).wrap().selectable(),
            text_block("Recent").semibold(),
            text_block(snap.history_preview.clone()).wrap().selectable(),
        ))
        .spacing(4.0)
        .horizontal_alignment(HorizontalAlignment::Stretch),
    ))
    .spacing(12.0)
    .horizontal_alignment(HorizontalAlignment::Stretch)
}
