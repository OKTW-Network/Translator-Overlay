//! OCR settings: model, timing, detection, and line merge.

use std::sync::Arc;

use parking_lot::Mutex;
use rust_i18n::t;
use translator_core::{LineJoin, LineMergeConfig, LineMergeMode, LineMergeOrder, ModelTier, OcrDevice};
use windows_reactor::{
    ChildrenControl, ComboBox, HorizontalAlignment, LayoutControl, LocalSender, Orientation, StackPanel, TooltipExt, VerticalAlignment,
    View,
};

use crate::ui::{
    cards::{section_header, settings_card, subsection_header},
    chrome::settings_page_shell,
    controls::{SliderNumberParams, card_slider_number, card_slider_number_enabled, card_toggle, compact_toggle, radio, wrap_tooltip},
    shared::{AppMsg, ChromeSnap, UiCx, UiShared, mark_dirty},
};

fn set_merge_f32(cx: &UiCx, set: impl FnOnce(&mut LineMergeConfig, f32), v: f64) {
    cx.with_mut(|ui| {
        set(&mut ui.draft.ocr.line_merge, v as f32);
        mark_dirty(ui);
    });
}

fn card_merge_pct(cx: &UiCx, p: SliderNumberParams, enabled: bool, set: impl Fn(&mut LineMergeConfig, f32) + Copy + 'static) -> View {
    let lo = (p.min / 100.0) as f32;
    let hi = (p.max / 100.0) as f32;
    card_slider_number_enabled(p, enabled, {
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
                    radio("ocr-model-tier", &t!("ocr.tiny"), Some(64.0), idx == 0, true, pick(0)),
                    radio("ocr-model-tier", &t!("ocr.small"), Some(72.0), idx == 1, true, pick(1)),
                    radio("ocr-model-tier", &t!("ocr.medium"), Some(84.0), idx == 2, true, pick(2)),
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
                    radio("ocr-device", &t!("ocr.webgpu"), Some(84.0), idx == 0, true, pick(0))
                        .tooltip_with(wrap_tooltip(&t!("ocr.webgpu_tip"))),
                    radio("ocr-device", &t!("ocr.directml"), Some(96.0), idx == 1, true, pick(1))
                        .tooltip_with(wrap_tooltip(&t!("ocr.directml_tip"))),
                    radio("ocr-device", &t!("ocr.cpu"), Some(64.0), idx == 2, true, pick(2)).tooltip_with(wrap_tooltip(&t!("ocr.cpu_tip"))),
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
        LineMergeOrder::TopToBottomRightToLeft => 0,
        LineMergeOrder::LeftToRightTopToBottom => 1,
    };
    let cx_order = cx.clone();
    let pick_order = move |choice: i32| {
        let cx = cx_order.clone();
        move || {
            cx.with_mut(|ui| {
                ui.draft.ocr.line_merge.order = match choice {
                    1 => LineMergeOrder::LeftToRightTopToBottom,
                    _ => LineMergeOrder::TopToBottomRightToLeft,
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

    // `Off` ignores every merge knob; Layout and Region group by boxes, so rule thresholds only apply to `Rules`.
    let merging = ocr.line_merge.mode != LineMergeMode::Off;
    let rules_only = ocr.line_merge.mode == LineMergeMode::Rules;
    let merge_join = StackPanel::new().spacing(4.0).children((
        section_header(t!("ocr.line_merge")),
        settings_card("ocr-line-merge", t!("ocr.merge_lines"), Some(&t!("ocr.merge_lines_desc")), {
            ComboBox::new()
                .items_source(vec![
                    t!("ocr.merge_mode_off").into_owned(),
                    t!("ocr.merge_mode_rules").into_owned(),
                    t!("ocr.merge_mode_layout").into_owned(),
                    t!("ocr.merge_mode_region").into_owned(),
                ])
                // Item order must match `LineMergeMode` declaration order.
                .selected_index(Some(ocr.line_merge.mode as usize))
                .on_selection_changed({
                    let cx = cx.clone();
                    move |idx: Option<usize>| {
                        let choice = match idx {
                            Some(0) => LineMergeMode::Off,
                            Some(1) => LineMergeMode::Rules,
                            Some(2) => LineMergeMode::Layout,
                            Some(3) => LineMergeMode::Region,
                            _ => return,
                        };
                        cx.with_mut(|ui| {
                            if ui.draft.ocr.line_merge.mode != choice {
                                ui.draft.ocr.line_merge.mode = choice;
                                mark_dirty(ui);
                            }
                        });
                    }
                })
                .width(160.0)
                .vertical_alignment(VerticalAlignment::Center)
        }),
        settings_card("ocr-merge-join", t!("ocr.join"), Some(&t!("ocr.join_desc")), {
            ComboBox::new()
                .items_source(vec![
                    t!("ocr.join_none").into_owned(),
                    t!("ocr.join_space").into_owned(),
                    t!("ocr.join_newline").into_owned(),
                ])
                .selected_index(Some(ocr.line_merge.join as usize))
                .on_selection_changed({
                    let cx = cx.clone();
                    move |idx: Option<usize>| {
                        let choice = match idx {
                            Some(0) => LineJoin::None,
                            Some(1) => LineJoin::Space,
                            Some(2) => LineJoin::Newline,
                            _ => return,
                        };
                        cx.with_mut(|ui| {
                            if ui.draft.ocr.line_merge.join != choice {
                                ui.draft.ocr.line_merge.join = choice;
                                mark_dirty(ui);
                            }
                        });
                    }
                })
                .is_enabled(merging)
                .width(160.0)
                .vertical_alignment(VerticalAlignment::Center)
        }),
        settings_card(
            "ocr-merge-order",
            t!("ocr.reading_order"),
            Some(&t!("ocr.reading_order_desc")),
            StackPanel::new()
                .spacing(4.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .children((
                    radio("ocr-merge-order", &t!("ocr.order_rows"), None, order_idx == 1, merging, pick_order(1)),
                    radio("ocr-merge-order", &t!("ocr.order_cols"), None, order_idx == 0, merging, pick_order(0)),
                )),
        ),
    ));

    // Row / column tolerances also band the reading order, so every merging mode uses them.
    let merge_row = StackPanel::new().spacing(4.0).children((
        subsection_header(t!("ocr.merge_horizontal")),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-word-gap",
                header: t!("ocr.word_gap"),
                description: Some(t!("ocr.word_gap_desc")),
                value: f64::from(ocr.line_merge.word_gap) * 100.0,
                min: 0.0,
                max: 8.0,
                step: 0.1,
            },
            rules_only,
            |m, x| m.word_gap = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-row-tolerance",
                header: t!("ocr.row_tolerance"),
                description: Some(t!("ocr.row_tolerance_desc")),
                value: f64::from(ocr.line_merge.row_tolerance) * 100.0,
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            merging,
            |m, x| m.row_tolerance = x,
        ),
    ));

    let merge_paragraph = StackPanel::new().spacing(4.0).children((
        subsection_header(t!("ocr.merge_vertical")),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-line-gap",
                header: t!("ocr.line_gap"),
                description: Some(t!("ocr.line_gap_desc")),
                value: f64::from(ocr.line_merge.line_gap) * 100.0,
                min: 0.0,
                max: 8.0,
                step: 0.1,
            },
            rules_only,
            |m, x| m.line_gap = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-column-tolerance",
                header: t!("ocr.column_tolerance"),
                description: Some(t!("ocr.column_tolerance_desc")),
                value: f64::from(ocr.line_merge.column_tolerance) * 100.0,
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            merging,
            |m, x| m.column_tolerance = x,
        ),
        settings_card(
            "ocr-merge-split-short",
            t!("ocr.split_short"),
            Some(&t!("ocr.split_short_desc")),
            compact_toggle(ocr.line_merge.split_short_above_wide, rules_only, {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.split_short_above_wide = v;
                        mark_dirty(ui);
                    });
                }
            }),
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-width-diff",
                header: t!("ocr.width_diff"),
                description: Some(t!("ocr.width_diff_desc")),
                value: f64::from(ocr.line_merge.width_diff) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            rules_only && ocr.line_merge.split_short_above_wide,
            |m, x| m.width_diff = x,
        ),
    ));

    let merge_shared = StackPanel::new().spacing(4.0).children((
        subsection_header(t!("ocr.merge_general")),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-size-diff",
                header: t!("ocr.size_diff"),
                description: Some(t!("ocr.size_diff_desc")),
                value: f64::from(ocr.line_merge.size_diff) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            rules_only,
            |m, x| m.size_diff = x,
        ),
    ));

    let line_merge = StackPanel::new()
        .spacing(4.0)
        .children((merge_join, merge_row, merge_paragraph, merge_shared));

    settings_page_shell(shared, chrome, bump, StackPanel::new().spacing(8.0).children((model, timing, detection, line_merge)))
}
