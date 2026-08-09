//! OCR / capture settings.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::ModelTier;
use windows_reactor::*;

use crate::ui::{
    chrome::{section_header, settings_card, settings_expander, settings_page_shell},
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

pub fn ocr_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> Element {
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
    let line_merge_body = vstack((
        row_toggle(
            "ocr-line-merge",
            "Merge lines into paragraphs",
            Some("Join stacked OCR lines that look like one paragraph (geometry only)."),
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
        row_slider_number(
            SliderNumberParams {
                key: "merge-max-gap",
                header: "Max gap (× line height)".into(),
                description: Some("Larger → more merges; too large glues menu lists.".into()),
                value: snap.merge_max_gap,
                min: 0.10,
                max: 1.20,
                step: 0.01,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.max_gap_ratio = x.max(0.05), v)
            },
        ),
        row_slider_number(
            SliderNumberParams {
                key: "merge-min-gap",
                header: "Min gap (× line height)".into(),
                description: Some("Negative allows slightly overlapping OCR boxes.".into()),
                value: snap.merge_min_gap,
                min: -1.0,
                max: 0.5,
                step: 0.01,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.min_gap_ratio = x, v)
            },
        ),
        row_slider_number(
            SliderNumberParams {
                key: "merge-gap-slack",
                header: "Leading slack".into(),
                description: Some("Adaptive max gap = typical leading × slack.".into()),
                value: snap.merge_gap_slack,
                min: 1.0,
                max: 2.5,
                step: 0.05,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.gap_slack = x.max(0.5), v)
            },
        ),
        row_slider_number(
            SliderNumberParams {
                key: "merge-list-gap-min",
                header: "List gap min (× h)".into(),
                description: Some("Gaps larger than this may be UI list spacing (not wraps).".into()),
                value: snap.merge_list_gap_min,
                min: 0.05,
                max: 0.80,
                step: 0.01,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.list_gap_min_ratio = x.max(0.0), v)
            },
        ),
        row_slider_number(
            SliderNumberParams {
                key: "merge-wrap-width",
                header: "Wrap width ratio".into(),
                description: Some("Wide line above shorter line (≥ this factor) counts as wrap.".into()),
                value: snap.merge_wrap_width,
                min: 1.1,
                max: 3.0,
                step: 0.05,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.wrap_width_ratio = x.max(1.0), v)
            },
        ),
        row_slider_number(
            SliderNumberParams {
                key: "merge-height-ratio",
                header: "Height match min".into(),
                description: Some("Lines must be similar height to merge (0–1).".into()),
                value: snap.merge_height_ratio,
                min: 0.20,
                max: 1.0,
                step: 0.01,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.height_ratio_min = x.clamp(0.1, 1.0), v)
            },
        ),
    ))
    .spacing(0.0)
    .horizontal_alignment(HorizontalAlignment::Stretch);

    let line_merge_adv_body = vstack((
        row_slider_number(
            SliderNumberParams {
                key: "merge-left-align",
                header: "Column align (× h)".into(),
                description: Some("Left-edge tolerance for same-column detection.".into()),
                value: snap.merge_left_align,
                min: 0.10,
                max: 1.5,
                step: 0.05,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.left_align_ratio = x.max(0.05), v)
            },
        ),
        row_slider_number(
            SliderNumberParams {
                key: "merge-short-max-gap",
                header: "Short–short max gap (× h)".into(),
                description: Some("Two short boxes only merge when almost touching.".into()),
                value: snap.merge_short_max_gap,
                min: 0.0,
                max: 0.80,
                step: 0.01,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.short_max_gap_ratio = x.max(0.0), v)
            },
        ),
        row_slider_number(
            SliderNumberParams {
                key: "merge-list-peers",
                header: "List min peers".into(),
                description: Some("Column needs this many lines to use list-pitch rules.".into()),
                value: snap.merge_list_min_peers,
                min: 2.0,
                max: 12.0,
                step: 1.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.list_min_peers = v.max(2.0) as u32;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        row_slider_number(
            SliderNumberParams {
                key: "merge-compact-aspect",
                header: "Compact box max w/h".into(),
                description: Some("Buttons/pills below this aspect under a wide heading.".into()),
                value: snap.merge_compact_aspect,
                min: 1.0,
                max: 4.0,
                step: 0.1,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.compact_aspect_max = x.max(0.5), v)
            },
        ),
        row_toggle(
            "merge-nameplate",
            "Keep nameplates separate",
            Some("Short narrow box above a wider line stays its own block."),
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
        row_slider_number(
            SliderNumberParams {
                key: "merge-nameplate-body",
                header: "Nameplate body width ratio".into(),
                description: Some("Body must be at least this × nameplate width.".into()),
                value: snap.merge_nameplate_body,
                min: 1.0,
                max: 3.0,
                step: 0.05,
            },
            {
                let cx = cx.clone();
                move |v| set_merge_f32(&cx, |m, x| m.nameplate_body_width_ratio = x.max(1.0), v)
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

    let line_merge_advanced = settings_expander(
        "ocr-exp-line-merge-adv",
        "Line merge · advanced",
        snap.expand_line_merge_adv,
        {
            let cx = cx.clone();
            move |open| {
                cx.with_mut(|ui| {
                    ui.expand_line_merge_adv = open;
                });
            }
        },
        line_merge_adv_body,
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

    settings_page_shell(
        shared,
        snap,
        bump,
        vstack((model, detection, line_merge, line_merge_advanced, timing))
            .spacing(8.0)
            .into(),
    )
}
