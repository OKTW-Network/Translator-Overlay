//! Dashboard: capture control, window picker, live OCR / translation preview.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_capture::list_windows;
use windows_reactor::*;

use crate::{
    pipeline::PipelineCommand,
    ui::{
        chrome::{app_status_strip, page_header, status_infobar},
        shared::{Snapshot, UiCx, UiShared},
    },
};

pub fn dashboard_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> Element {
    let cx = UiCx::new(shared, bump);

    let start_label = if snap.auto_running { "Stop" } else { "Start" };
    let start_tip = if snap.auto_running {
        "Stop continuous capture"
    } else {
        "Start continuous capture of the selected window"
    };
    let show_preview = snap.show_preview;
    let in_flight = snap.translate_in_flight;
    let can_retry = snap.can_retry;

    // ComboBox popup opens below the control (MenuFlyout on DropDownButton
    // defaults to Top placement and often expands upward).
    let window_items: Vec<String> = if snap.window_labels.is_empty() {
        vec!["(no windows — refresh)".into()]
    } else {
        snap.window_labels.clone()
    };
    let window_selected = if snap.window_count == 0 { -1 } else { snap.selected_window_idx };

    vstack((
        page_header("Dashboard", Some("Choose a window, capture text, and watch translations.")),
        app_status_strip(snap),
        status_infobar(snap),
        {
            let mut picker = ComboBox::new(window_items)
                .selected_index(window_selected)
                .placeholder_text("Select window…")
                .enabled(snap.window_count > 0)
                .on_selection_changed({
                    let cx = cx.clone();
                    move |idx: i32| {
                        if idx < 0 {
                            return;
                        }
                        cx.with_mut(|ui| {
                            let i = idx as usize;
                            if i < ui.windows.len() {
                                ui.selected_idx = i;
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
                .tooltip("Refresh window list")
                .on_click({
                    let cx = cx.clone();
                    move || {
                        cx.with_mut(|ui| {
                            ui.windows = list_windows().unwrap_or_default();
                            if ui.selected_idx >= ui.windows.len() && !ui.windows.is_empty() {
                                ui.selected_idx = 0;
                            }
                        });
                    }
                });

            hstack((text_block("Window").semibold().vertical_alignment(VerticalAlignment::Center), picker, refresh))
                .spacing(8.0)
                .with_key("window-picker-row")
        },
        hstack((
            button("Foreground")
                .tooltip("Capture the window that currently has focus")
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::StartForeground)
                }),
            button(start_label).tooltip(start_tip).on_click({
                let cx = cx.clone();
                move || {
                    {
                        let ui = cx.shared.lock();
                        if ui.state.read().auto_running {
                            let _ = ui.cmd_tx.send(PipelineCommand::StopCapture);
                        } else if let Some(w) = ui.windows.get(ui.selected_idx).cloned() {
                            let _ = ui.cmd_tx.send(PipelineCommand::StartCapture {
                                hwnd: w.hwnd,
                                title: w.title,
                            });
                        }
                    }
                    cx.refresh();
                }
            }),
            button("Once").tooltip("Capture once and translate immediately").on_click({
                let cx = cx.clone();
                move || cx.send_cmd(PipelineCommand::ManualCapture)
            }),
        ))
        .spacing(8.0),
        hstack((
            button(if show_preview { "Preview on" } else { "Preview off" })
                .tooltip(if show_preview {
                    "Turn off preview image"
                } else {
                    "Turn on preview image"
                })
                .on_click({
                    let cx = cx.clone();
                    move || {
                        let ui = cx.shared.lock();
                        let next = !ui.state.read().config.capture.show_preview;
                        let _ = ui.cmd_tx.send(PipelineCommand::SetShowPreview(next));
                        drop(ui);
                        cx.refresh();
                    }
                }),
            button("Clear chat")
                .tooltip("Clear the translation model conversation history")
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::ResetConversation)
                }),
            button("Stop all").tooltip("Stop capture immediately").on_click({
                let cx = cx.clone();
                move || cx.send_cmd(PipelineCommand::StopCapture)
            }),
        ))
        .spacing(8.0),
        hstack((
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
            format!("Last OCR: {ocr_time}  ·  {} blocks  ·  Preview: {}", snap.last_ocr_block_count, snap.preview)
        })
        .font_size(12.0)
        .foreground(ThemeRef::SecondaryText),
        vstack((
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
        .spacing(4.0),
    ))
    .spacing(12.0)
    .into()
}
