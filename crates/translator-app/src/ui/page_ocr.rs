//! OCR / capture settings.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{LineMergeConfig, LineMergeOrder, ModelTier};
use windows_reactor::{HorizontalAlignment, LayoutExt, RadioButton, StackPanel, Updater, VerticalAlignment, hstack, vstack};

use crate::ui::{
    chrome::{section_header, settings_card, settings_page_shell, subsection_header},
    controls::{SliderNumberParams, card_slider_number, card_toggle},
    shared::{ChromeSnap, UiCx, UiShared, mark_dirty},
};

/// Bind a line-merge f32 field from a slider/number value.
fn set_merge_f32(cx: &UiCx, set: impl FnOnce(&mut LineMergeConfig, f32), v: f64) {
    cx.with_mut(|ui| {
        set(&mut ui.draft.ocr.line_merge, v as f32);
        mark_dirty(ui);
    });
}

/// Percent slider: UI shows 0–100 (or a subrange); config stores the 0–1 ratio.
///
/// Slider min/max/step are the displayed percents so defaults land on ticks
/// (`min + n×step`). The stored ratio is `percent / 100`, clamped to the same range.
fn card_merge_pct(cx: &UiCx, p: SliderNumberParams, set: impl Fn(&mut LineMergeConfig, f32) + Copy + 'static) -> windows_reactor::Border {
    let lo = (p.min / 100.0) as f32;
    let hi = (p.max / 100.0) as f32;
    card_slider_number(p, {
        let cx = cx.clone();
        move |v| set_merge_f32(&cx, |m, x| set(m, (x / 100.0).clamp(lo, hi)), v)
    })
}

pub fn ocr_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);
    let (ocr, capture) = {
        let ui = shared.lock();
        (ui.draft.ocr.clone(), ui.draft.capture.clone())
    };

    let model = vstack((
        section_header("Model"),
        // Compact but readable: default RadioButton MinWidth (~120) spreads
        // short labels too far; zero padding/min-width crushes circle+text.
        // Cap width near content size and space items with hstack only.
        settings_card("ocr-tier", "Model size", Some("Smaller is faster; larger is more accurate. Reloads on Save."), {
            let idx = match ocr.model_tier {
                ModelTier::Tiny => 0,
                ModelTier::Small => 1,
                ModelTier::Medium => 2,
            };
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
                value: capture.min_interval_ms as f64,
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
                value: ocr.stable_duration_ms as f64,
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
                value: ocr.max_unstable_ms as f64,
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
                value: ocr.block_persist_ms as f64,
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
                value: ocr.block_max_miss_ms as f64,
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

    let order_idx = match ocr.line_merge.order {
        LineMergeOrder::TopToBottomLeftToRight => 0,
        LineMergeOrder::LeftToRightTopToBottom => 1,
    };
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
                value: f64::from(ocr.confidence_threshold),
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
        card_toggle(
            "ocr-filter-single",
            "Ignore single characters",
            Some("Drop lone single-character detections."),
            ocr.filter_single_char,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.filter_single_char = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let merge_join = vstack((
        section_header("Line merge"),
        card_toggle(
            "ocr-line-merge",
            "Merge lines",
            Some("Join nearby OCR lines that share a column or row, similar height, and a small gap."),
            ocr.line_merge.enabled,
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
            ocr.line_merge.merge_whole_region,
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
            ocr.line_merge.join_with_space,
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
                value: f64::from(ocr.line_merge.order_band_ratio) * 100.0,
                min: 0.1,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.order_band_ratio = x,
        ),
    ))
    .spacing(4.0);

    let merge_stacking = vstack((
        subsection_header("Vertical"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-gap",
                header: "Gap (% of window height)".into(),
                description: Some("Allowed |vertical gap|. Overlap and a small space count the same. Default 1.5.".into()),
                value: f64::from(ocr.line_merge.gap_ratio) * 100.0,
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
                description: Some("How far a lower/right line may cross the mid and still count as below/right. Default 25.".into()),
                value: f64::from(ocr.line_merge.below_mid_ratio) * 100.0,
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
                value: f64::from(ocr.line_merge.height_delta_ratio) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.height_delta_ratio = x,
        ),
    ))
    .spacing(4.0);

    let merge_column = vstack((
        subsection_header("Horizontal"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-horizontal-gap",
                header: "Gap (% of window width)".into(),
                description: Some(
                    "Allowed |horizontal gap| for side-by-side lines. Overlap and a small space count the same. Default 1.5. Set 0 to disable."
                        .into(),
                ),
                value: f64::from(ocr.line_merge.horizontal_gap_ratio) * 100.0,
                min: 0.0,
                max: 8.0,
                step: 0.1,
            },
            |m, x| m.horizontal_gap_ratio = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-align",
                header: "Align tolerance (% of window width)".into(),
                description: Some(
                    "Left-/center-edge delta for one column (× width), or top-/center for one row (× height). Default 1.2."
                        .into(),
                ),
                value: f64::from(ocr.line_merge.align_ratio) * 100.0,
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.align_ratio = x,
        ),
    ))
    .spacing(4.0);

    let merge_short = vstack((
        subsection_header("Short into long"),
        card_toggle(
            "ocr-merge-reject-short",
            "Don't merge short into long",
            Some("On: a shorter line above a much wider line stays its own block. Off: width is ignored."),
            ocr.line_merge.reject_short_long,
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
                value: f64::from(ocr.line_merge.width_delta_ratio) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.width_delta_ratio = x,
        ),
    ))
    .spacing(4.0);

    let line_merge = vstack((merge_join, merge_order, merge_stacking, merge_column, merge_short)).spacing(4.0);

    settings_page_shell(shared, chrome, bump, vstack((model, timing, detection, line_merge)).spacing(8.0))
}
