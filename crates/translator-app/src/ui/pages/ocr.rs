//! OCR settings: model, timing, detection, and line merge.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{LineMergeConfig, LineMergeOrder, ModelTier, OcrDevice};
use windows_reactor::{
    ChildrenControl, HorizontalAlignment, LayoutControl, LocalSender, Orientation, StackPanel, TooltipExt, VerticalAlignment, View,
};

use crate::ui::{
    cards::{section_header, settings_card, subsection_header},
    chrome::settings_page_shell,
    controls::{SliderNumberParams, card_slider_number, card_toggle, radio, wrap_tooltip},
    shared::{AppMsg, ChromeSnap, UiCx, UiShared, mark_dirty},
};

fn set_merge_f32(cx: &UiCx, set: impl FnOnce(&mut LineMergeConfig, f32), v: f64) {
    cx.with_mut(|ui| {
        set(&mut ui.draft.ocr.line_merge, v as f32);
        mark_dirty(ui);
    });
}

fn card_merge_pct(cx: &UiCx, p: SliderNumberParams, set: impl Fn(&mut LineMergeConfig, f32) + Copy + 'static) -> View {
    let lo = (p.min / 100.0) as f32;
    let hi = (p.max / 100.0) as f32;
    card_slider_number(p, {
        let cx = cx.clone();
        move |v| set_merge_f32(&cx, |m, x| set(m, (x / 100.0).clamp(lo, hi)), v)
    })
}

pub fn ocr_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let (ocr, capture) = {
        let ui = shared.lock();
        (ui.draft.ocr.clone(), ui.draft.capture.clone())
    };

    let model = StackPanel::new().spacing(4.0).children((
        section_header("Model"),
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
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .vertical_alignment(VerticalAlignment::Center)
                .children((
                    radio("ocr-model-tier", "tiny", Some(64.0), idx == 0, pick(0)),
                    radio("ocr-model-tier", "small", Some(72.0), idx == 1, pick(1)),
                    radio("ocr-model-tier", "medium", Some(84.0), idx == 2, pick(2)),
                ))
        }),
        settings_card("ocr-device", "Device", Some("GPU (WebGPU or DirectML) is faster; CPU is more compatible. Reloads on Save."), {
            let idx = match ocr.device {
                OcrDevice::Webgpu => 0,
                OcrDevice::Directml => 1,
                OcrDevice::Cpu => 2,
            };
            let cx_device = cx.clone();
            let pick = move |choice: i32| {
                let cx = cx_device.clone();
                move || {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.device = match choice {
                            1 => OcrDevice::Directml,
                            2 => OcrDevice::Cpu,
                            _ => OcrDevice::Webgpu,
                        };
                        mark_dirty(ui);
                    });
                }
            };
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .vertical_alignment(VerticalAlignment::Center)
                .children((
                    radio("ocr-device", "WebGPU", Some(84.0), idx == 0, pick(0)).tooltip_with(wrap_tooltip(
                        "OCR on the GPU via WebGPU (Dawn / D3D12).\nFaster than CPU; often faster than DirectML on Intel.\nFalls back to CPU if WebGPU fails.",
                    )),
                    radio("ocr-device", "DirectML", Some(96.0), idx == 1, pick(1)).tooltip_with(wrap_tooltip(
                        "OCR on the GPU via DirectML (D3D12).\nMay be faster than WebGPU on NVIDIA/AMD.\nFalls back to CPU if DirectML fails.",
                    )),
                    radio("ocr-device", "CPU", Some(64.0), idx == 2, pick(2)).tooltip_with(wrap_tooltip(
                        "OCR on the CPU only.\nSlower than GPU.\nMore compatible if WebGPU and DirectML are unavailable.",
                    )),
                ))
        }),
    ));

    let timing = StackPanel::new().spacing(4.0).children((
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
    ));

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
    let detection = StackPanel::new().spacing(4.0).children((
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
    ));

    let merge_join = StackPanel::new().spacing(4.0).children((
        section_header("Line merge"),
        card_toggle(
            "ocr-line-merge",
            "Merge lines",
            Some("Join nearby lines that share a column or row, similar height, and a small gap."),
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
            Some("Join every line in each drawn OCR region. Ignored for whole-window capture."),
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
            Some("On: space between joined lines. Off: glue them (typical for CJK)."),
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
    ));

    let merge_order = StackPanel::new().spacing(4.0).children((
        subsection_header("Reading order"),
        settings_card(
            "ocr-merge-order",
            "Reading order",
            Some("Order for joining lines and listing the blocks."),
            StackPanel::new()
                .spacing(4.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .children((
                    radio("ocr-merge-order", "Left to right, then top to bottom (rows)", None, order_idx == 1, pick_order(1)),
                    radio("ocr-merge-order", "Top to bottom, then left to right (columns)", None, order_idx == 0, pick_order(0)),
                )),
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-order-band",
                header: "Row/column band (% of window)".into(),
                description: Some("How close lines must be to count as the same row or column. Default 1.2.".into()),
                value: f64::from(ocr.line_merge.order_band_ratio) * 100.0,
                min: 0.1,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.order_band_ratio = x,
        ),
    ));

    let merge_stacking = StackPanel::new().spacing(4.0).children((
        subsection_header("Vertical"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-gap",
                header: "Gap (% of window height)".into(),
                description: Some("Max distance between stacked lines. A small overlap counts as a small gap. Default 1.5.".into()),
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
                header: "Midpoint slack (% of line size)".into(),
                description: Some(
                    "How far a line may sit past the previous midpoint and still count as below or to the right. Default 25.".into(),
                ),
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
                header: "Height difference (%)".into(),
                description: Some("Max height difference vs the taller line. Default 45.".into()),
                value: f64::from(ocr.line_merge.height_delta_ratio) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.height_delta_ratio = x,
        ),
    ));

    let merge_column = StackPanel::new().spacing(4.0).children((
        subsection_header("Horizontal"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-horizontal-gap",
                header: "Gap (% of window width)".into(),
                description: Some(
                    "Max distance between side-by-side lines. A small overlap counts as a small gap. 0 = only if they touch. Default 1.5."
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
                header: "Align tolerance (%)".into(),
                description: Some("Max left/center drift for one column, or top/center for one row. Default 1.2.".into()),
                value: f64::from(ocr.line_merge.align_ratio) * 100.0,
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.align_ratio = x,
        ),
    ));

    let merge_short = StackPanel::new().spacing(4.0).children((
        subsection_header("Short into long"),
        card_toggle(
            "ocr-merge-reject-short",
            "Don't merge short into long",
            Some("Keep a short line separate from a much wider line below."),
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
                header: "Width difference (%)".into(),
                description: Some("If the lower line is at least this much wider, keep the short line separate. Default 25.".into()),
                value: f64::from(ocr.line_merge.width_delta_ratio) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.width_delta_ratio = x,
        ),
    ));

    let line_merge = StackPanel::new()
        .spacing(4.0)
        .children((merge_join, merge_order, merge_stacking, merge_column, merge_short));

    settings_page_shell(shared, chrome, bump, StackPanel::new().spacing(8.0).children((model, timing, detection, line_merge)))
}
