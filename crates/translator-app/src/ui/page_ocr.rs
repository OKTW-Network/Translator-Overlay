//! OCR / capture settings.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{LineMergeOrder, ModelTier};
use windows_reactor::{HorizontalAlignment, LayoutExt, RadioButton, StackPanel, Updater, VerticalAlignment, hstack, vstack};

use crate::ui::{
    chrome::{section_header, settings_card, settings_expander, settings_page_shell, settings_row},
    controls::{SliderNumberParams, card_slider_number, card_toggle, row_slider_number, row_toggle},
    shared::{Snapshot, UiCx, UiShared, mark_dirty},
};

/// Bind a line-merge f32 field from a slider/number value.
fn set_merge_f32(cx: &UiCx, set: impl FnOnce(&mut translator_core::LineMergeConfig, f32), v: f64) {
    cx.with_mut(|ui| {
        set(&mut ui.draft.ocr.line_merge, v as f32);
        mark_dirty(ui);
    });
}

pub fn ocr_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);

    let model = vstack((
        section_header("Model"),
        // Compact but readable: default RadioButton MinWidth (~120) spreads
        // short labels too far; zero padding/min-width crushes circle+text.
        // Cap width near content size and space items with hstack only.
        settings_card("ocr-tier", "Model size", Some("Smaller is faster; larger is more accurate. Reloads on Save."), {
            let idx = snap.model_tier_idx;
            let cx_tier = cx.clone();
            let pick = move |choice: i32| {
                let cx = cx_tier.clone();
                move || {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.model_tier = match choice {
                            0 => ModelTier::Tiny,
                            2 => ModelTier::Medium,
                            _ => ModelTier::Small,
                        };
                        mark_dirty(ui);
                    });
                }
            };
            // Width ≈ glyph + label; leave template padding for circle↔text.
            let radio = |label: &str, width: f64, checked: bool, on: Box<dyn Fn() + 'static>| {
                let mut rb = RadioButton::new(label).group("ocr-model-tier").checked(checked).on_checked(on);
                rb.modifiers.min_width = Some(width);
                rb.modifiers.width = Some(width);
                rb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);
                rb
            };
            hstack((
                radio("tiny", 64.0, idx == 0, Box::new(pick(0))),
                radio("small", 72.0, idx == 1, Box::new(pick(1))),
                radio("medium", 84.0, idx == 2, Box::new(pick(2))),
            ))
            .spacing(12.0)
            .vertical_alignment(VerticalAlignment::Center)
        }),
    ))
    .spacing(4.0);

    let detection = vstack((
        section_header("Detection"),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-confidence",
                header: "Min confidence".into(),
                description: Some("Ignore text below this score (0–1).".into()),
                value: snap.confidence,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.confidence_threshold = v.clamp(0.0, 1.0) as f32;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_toggle("ocr-filter-single", "Ignore single characters", Some("Drop lone single-character detections."), snap.filter_single, {
            let cx = cx.clone();
            move |v| {
                cx.with_mut(|ui| {
                    ui.draft.ocr.filter_single_char = v;
                    mark_dirty(ui);
                });
            }
        }),
    ))
    .spacing(4.0);

    // Flat rows inside Expander (SettingsExpander.Items style) — no nested cards.
    let order_idx = snap.merge_order_idx;
    let cx_order = cx.clone();
    let pick_order = move |choice: i32| {
        let cx = cx_order.clone();
        move || {
            cx.with_mut(|ui| {
                ui.draft.ocr.line_merge.order = match choice {
                    1 => LineMergeOrder::LeftToRightTopToBottom,
                    _ => LineMergeOrder::TopToBottomLeftToRight,
                };
                mark_dirty(ui);
            });
        }
    };
    let order_radio = |label: &str, width: f64, checked: bool, on: Box<dyn Fn() + 'static>| {
        let mut rb = RadioButton::new(label).group("ocr-merge-order").checked(checked).on_checked(on);
        rb.modifiers.min_width = Some(width);
        rb.modifiers.width = Some(width);
        rb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);
        rb
    };

    let line_merge_body = vstack((
        row_toggle(
            "ocr-line-merge",
            "Merge lines",
            Some("Join stacked OCR lines that look like a wrapped paragraph (upper line longer than the one below)."),
            snap.merge_enabled,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.enabled = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        row_toggle(
            "ocr-merge-whole-region",
            "Merge entire selected region",
            Some("When OCR regions are set, join every line inside each region. Ignored for whole-window OCR."),
            snap.merge_whole_region,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.merge_whole_region = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        settings_row(
            "ocr-merge-order",
            "Merge order",
            Some("Reading order when joining lines inside a merged block."),
            vstack((
                order_radio("Top to bottom, then left to right", 280.0, order_idx == 0, Box::new(pick_order(0))),
                order_radio("Left to right, then top to bottom", 280.0, order_idx == 1, Box::new(pick_order(1))),
            ))
            .spacing(4.0),
        ),
        row_slider_number(
            SliderNumberParams {
                key: "merge-max-gap",
                header: "Max gap (% of window height)".into(),
                description: Some("Only used by rule-based merge. Larger → more merges.".into()),
                value: snap.merge_max_gap_pct,
                min: 0.5,
                max: 8.0,
                step: 0.1,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.max_gap_ratio = (x / 100.0).clamp(0.001, 0.20), v)
            },
        ),
        row_toggle(
            "merge-nameplate",
            "Keep nameplates separate",
            Some("On: speaker tag stays its own block. Off: join it into the line below."),
            snap.merge_keep_nameplate,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.keep_speaker_separate = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ))
    .spacing(0.0)
    .horizontal_alignment(HorizontalAlignment::Stretch);

    let line_merge = settings_expander(
        "ocr-exp-line-merge",
        "Line merge",
        snap.expand_line_merge,
        {
            let cx = cx.clone();
            move |open| {
                cx.with_mut(|ui| {
                    ui.expand_line_merge = open;
                });
            }
        },
        line_merge_body,
    );

    let timing = vstack((
        section_header("Timing"),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-stable-ms",
                header: "Stable wait (ms)".into(),
                description: Some("Wait until text stops changing, then translate.".into()),
                value: snap.stable_ms,
                min: 0.0,
                max: 10_000.0,
                step: 50.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.stable_duration_ms = v.max(0.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-max-unstable-ms",
                header: "Force translate (ms)".into(),
                description: Some("If OCR keeps changing, translate anyway after this long. 0 = off.".into()),
                value: snap.max_unstable_ms,
                min: 0.0,
                max: 15_000.0,
                step: 100.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.max_unstable_ms = v.max(0.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-interval-ms",
                header: "Capture interval (ms)".into(),
                description: Some("How often to grab a new frame.".into()),
                value: snap.interval_ms,
                min: 50.0,
                max: 5_000.0,
                step: 50.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.capture.min_interval_ms = v.max(50.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-persist-ms",
                header: "Keep after gone (ms)".into(),
                description: Some("Keep text on overlay after it disappears. 0 = off.".into()),
                value: snap.persist_ms,
                min: 0.0,
                max: 5_000.0,
                step: 50.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.block_persist_ms = v.max(0.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-miss-ms",
                header: "Drop after miss (ms)".into(),
                description: Some("Remove text if not seen again within this time.".into()),
                value: snap.max_miss_ms,
                min: 0.0,
                max: 10_000.0,
                step: 50.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.block_max_miss_ms = v.max(0.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    settings_page_shell(shared, snap, bump, vstack((model, detection, line_merge, timing)).spacing(8.0))
}
