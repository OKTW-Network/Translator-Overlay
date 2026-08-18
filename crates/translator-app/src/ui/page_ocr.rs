//! OCR / capture settings.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{LineMergeOrder, ModelTier};
use windows_reactor::{HorizontalAlignment, LayoutExt, RadioButton, StackPanel, Updater, VerticalAlignment, hstack, vstack};

use crate::ui::{
    chrome::{section_header, settings_card, settings_page_shell, subsection_header},
    controls::{SliderNumberParams, card_slider_number, card_toggle},
    shared::{Snapshot, UiCx, UiShared, mark_dirty},
};

/// Bind a line-merge f32 field from a slider/number value.
fn set_merge_f32(cx: &UiCx, set: impl FnOnce(&mut translator_core::LineMergeConfig, f32), v: f64) {
    cx.with_mut(|ui| {
        set(&mut ui.draft.ocr.line_merge, v as f32);
        mark_dirty(ui);
    });
}

/// Percent slider: UI shows 0–100 (or a subrange); config stores the 0–1 ratio.
///
/// Slider min/max/step are the displayed percents so defaults land on ticks
/// (`min + n×step`). The stored ratio is `percent / 100`, clamped to the same range.
fn card_merge_pct(
    cx: &UiCx,
    p: SliderNumberParams,
    set: impl Fn(&mut translator_core::LineMergeConfig, f32) + Copy + 'static,
) -> windows_reactor::Border {
    let lo = (p.min / 100.0) as f32;
    let hi = (p.max / 100.0) as f32;
    card_slider_number(p, {
        let cx = cx.clone();
        move |v| set_merge_f32(&cx, |m, x| set(m, (x / 100.0).clamp(lo, hi)), v)
    })
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

    let timing = vstack((
        section_header("Timing"),
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
    // Shrink-wrap: a fixed width left-aligns the glyph inside the box.
    let order_radio = |label: &str, checked: bool, on: Box<dyn Fn() + 'static>| {
        let mut rb = RadioButton::new(label).group("ocr-merge-order").checked(checked).on_checked(on);
        rb.modifiers.horizontal_alignment = Some(HorizontalAlignment::Left);
        rb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);
        rb
    };

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

    let merge_join = vstack((
        section_header("Line merge"),
        card_toggle(
            "ocr-line-merge",
            "Merge lines",
            Some("Join stacked OCR lines that share a column, similar height, and a small vertical gap."),
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
        card_toggle(
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
        card_toggle(
            "ocr-merge-join-space",
            "Join with space",
            Some("On: insert a space between joined lines. Off: concatenate (typical for CJK)."),
            snap.merge_join_with_space,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.join_with_space = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let merge_order = vstack((
        subsection_header("Reading order"),
        settings_card(
            "ocr-merge-order",
            "Merge order",
            Some("Reading order when joining lines inside a merged block."),
            vstack((
                order_radio("Left to right, then top to bottom", order_idx == 1, Box::new(pick_order(1))),
                order_radio("Top to bottom, then left to right", order_idx == 0, Box::new(pick_order(0))),
            ))
            .spacing(4.0)
            .horizontal_alignment(HorizontalAlignment::Right),
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-order-band",
                header: "Order band (% of window)".into(),
                description: Some("Row/column grouping width for reading order. Default 1.2.".into()),
                value: snap.merge_order_band_pct,
                min: 0.1,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.order_band_ratio = x,
        ),
    ))
    .spacing(4.0);

    let merge_stacking = vstack((
        subsection_header("Stacking"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-gap",
                header: "Gap (% of window height)".into(),
                description: Some("Allowed |vertical gap|. Overlap and a small space count the same. Default 1.5.".into()),
                value: snap.merge_gap_pct,
                min: 0.0,
                max: 8.0,
                step: 0.1,
            },
            |m, x| m.gap_ratio = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-below-mid",
                header: "Below-mid slack (% of line height)".into(),
                description: Some("How far a lower line may sit above the upper mid and still count as below. Default 25.".into()),
                value: snap.merge_below_mid_pct,
                min: 0.0,
                max: 50.0,
                step: 1.0,
            },
            |m, x| m.below_mid_ratio = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-height-delta",
                header: "Height delta (%)".into(),
                description: Some("Allowed |h1 − h2| / larger height. Default 45.".into()),
                value: snap.merge_height_delta_pct,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.height_delta_ratio = x,
        ),
    ))
    .spacing(4.0);

    let merge_column = vstack((
        subsection_header("Column"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-overlap",
                header: "Overlap (% of shorter line)".into(),
                description: Some("Horizontal overlap vs the shorter line. Default 35.".into()),
                value: snap.merge_overlap_pct,
                min: 0.0,
                max: 100.0,
                step: 1.0,
            },
            |m, x| m.overlap_ratio = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-align",
                header: "Align tolerance (% of window width)".into(),
                description: Some("Left- or center-edge delta that still counts as one column. Default 1.2.".into()),
                value: snap.merge_align_pct,
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.align_ratio = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-align-overlap",
                header: "Align overlap (% of shorter line)".into(),
                description: Some("Overlap floor when using the column-align path. Default 10.".into()),
                value: snap.merge_align_overlap_pct,
                min: 0.0,
                max: 50.0,
                step: 1.0,
            },
            |m, x| m.align_overlap_ratio = x,
        ),
    ))
    .spacing(4.0);

    let merge_short = vstack((
        subsection_header("Short into long"),
        card_toggle(
            "ocr-merge-reject-short",
            "Don't merge short into long",
            Some("On: a shorter line above a much wider line stays its own block. Off: width is ignored."),
            snap.merge_reject_short_long,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.reject_short_long = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-width-delta",
                header: "Width delta (%)".into(),
                description: Some("When the short-into-long guard is on: allowed (lower − upper) / lower width. Default 40.".into()),
                value: snap.merge_width_delta_pct,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.width_delta_ratio = x,
        ),
    ))
    .spacing(4.0);

    let line_merge = vstack((merge_join, merge_order, merge_stacking, merge_column, merge_short)).spacing(4.0);

    settings_page_shell(shared, snap, bump, vstack((model, timing, detection, line_merge)).spacing(8.0))
}
