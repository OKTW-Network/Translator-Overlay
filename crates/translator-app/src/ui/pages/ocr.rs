//! OCR settings: model, timing, detection, and line merge.

use std::sync::Arc;

use parking_lot::Mutex;
use rust_i18n::t;
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
        section_header(t!("ocr.model")),
        settings_card("ocr-tier", t!("ocr.model_size"), Some(&t!("ocr.model_size_desc")), {
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
                    radio("ocr-model-tier", &t!("ocr.tiny"), Some(64.0), idx == 0, pick(0)),
                    radio("ocr-model-tier", &t!("ocr.small"), Some(72.0), idx == 1, pick(1)),
                    radio("ocr-model-tier", &t!("ocr.medium"), Some(84.0), idx == 2, pick(2)),
                ))
        }),
        settings_card("ocr-device", t!("ocr.device"), Some(&t!("ocr.device_desc")), {
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
                    radio("ocr-device", &t!("ocr.webgpu"), Some(84.0), idx == 0, pick(0)).tooltip_with(wrap_tooltip(&t!("ocr.webgpu_tip"))),
                    radio("ocr-device", &t!("ocr.directml"), Some(96.0), idx == 1, pick(1))
                        .tooltip_with(wrap_tooltip(&t!("ocr.directml_tip"))),
                    radio("ocr-device", &t!("ocr.cpu"), Some(64.0), idx == 2, pick(2)).tooltip_with(wrap_tooltip(&t!("ocr.cpu_tip"))),
                ))
        }),
    ));

    let timing = StackPanel::new().spacing(4.0).children((
        section_header(t!("ocr.timing")),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-interval-ms",
                header: t!("ocr.interval"),
                description: Some(t!("ocr.interval_desc")),
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
                header: t!("ocr.stable"),
                description: Some(t!("ocr.stable_desc")),
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
                header: t!("ocr.force"),
                description: Some(t!("ocr.force_desc")),
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
                header: t!("ocr.keep"),
                description: Some(t!("ocr.keep_desc")),
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
                header: t!("ocr.drop"),
                description: Some(t!("ocr.drop_desc")),
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
        section_header(t!("ocr.detection")),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-confidence",
                header: t!("ocr.confidence"),
                description: Some(t!("ocr.confidence_desc")),
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
        card_toggle("ocr-filter-single", t!("ocr.filter_single"), Some(&t!("ocr.filter_single_desc")), ocr.filter_single_char, {
            let cx = cx.clone();
            move |v| {
                cx.with_mut(|ui| {
                    ui.draft.ocr.filter_single_char = v;
                    mark_dirty(ui);
                });
            }
        }),
    ));

    let merge_join = StackPanel::new().spacing(4.0).children((
        section_header(t!("ocr.line_merge")),
        card_toggle("ocr-line-merge", t!("ocr.merge_lines"), Some(&t!("ocr.merge_lines_desc")), ocr.line_merge.enabled, {
            let cx = cx.clone();
            move |v| {
                cx.with_mut(|ui| {
                    ui.draft.ocr.line_merge.enabled = v;
                    mark_dirty(ui);
                });
            }
        }),
        card_toggle(
            "ocr-merge-whole-region",
            t!("ocr.merge_region"),
            Some(&t!("ocr.merge_region_desc")),
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
        card_toggle("ocr-merge-join-space", t!("ocr.join_space"), Some(&t!("ocr.join_space_desc")), ocr.line_merge.join_with_space, {
            let cx = cx.clone();
            move |v| {
                cx.with_mut(|ui| {
                    ui.draft.ocr.line_merge.join_with_space = v;
                    mark_dirty(ui);
                });
            }
        }),
    ));

    let merge_order = StackPanel::new().spacing(4.0).children((
        subsection_header(t!("ocr.reading_order")),
        settings_card(
            "ocr-merge-order",
            t!("ocr.reading_order"),
            Some(&t!("ocr.reading_order_desc")),
            StackPanel::new()
                .spacing(4.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .children((
                    radio("ocr-merge-order", &t!("ocr.order_rows"), None, order_idx == 1, pick_order(1)),
                    radio("ocr-merge-order", &t!("ocr.order_cols"), None, order_idx == 0, pick_order(0)),
                )),
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-order-band",
                header: t!("ocr.order_band"),
                description: Some(t!("ocr.order_band_desc")),
                value: f64::from(ocr.line_merge.order_band_ratio) * 100.0,
                min: 0.1,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.order_band_ratio = x,
        ),
    ));

    let merge_stacking = StackPanel::new().spacing(4.0).children((
        subsection_header(t!("ocr.vertical")),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-gap",
                header: t!("ocr.gap_h"),
                description: Some(t!("ocr.gap_h_desc")),
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
                header: t!("ocr.mid"),
                description: Some(t!("ocr.mid_desc")),
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
                header: t!("ocr.height_delta"),
                description: Some(t!("ocr.height_delta_desc")),
                value: f64::from(ocr.line_merge.height_delta_ratio) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.height_delta_ratio = x,
        ),
    ));

    let merge_column = StackPanel::new().spacing(4.0).children((
        subsection_header(t!("ocr.horizontal")),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-horizontal-gap",
                header: t!("ocr.gap_w"),
                description: Some(t!("ocr.gap_w_desc")),
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
                header: t!("ocr.align"),
                description: Some(t!("ocr.align_desc")),
                value: f64::from(ocr.line_merge.align_ratio) * 100.0,
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.align_ratio = x,
        ),
    ));

    let merge_short = StackPanel::new().spacing(4.0).children((
        subsection_header(t!("ocr.short")),
        card_toggle(
            "ocr-merge-reject-short",
            t!("ocr.reject_short"),
            Some(&t!("ocr.reject_short_desc")),
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
                header: t!("ocr.width_delta"),
                description: Some(t!("ocr.width_delta_desc")),
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
