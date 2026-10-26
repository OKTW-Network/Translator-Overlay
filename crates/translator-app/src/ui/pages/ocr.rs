//! OCR settings: model, timing, detection, and line merge.

use rust_i18n::t;
use translator_core::{LineJoin, LineMergeConfig, LineMergeMode, LineMergeOrder, ModelTier, OcrDevice};
use translator_ocr::memory_estimate;
use translator_winui::{Border, HorizontalAlignment, InfoBar, Layout, Result, StackPanel, VerticalAlignment};

use crate::{
    children,
    ui::{
        state::{UiCx, UiShared, mark_dirty},
        widgets::{
            Binds, Prop,
            cards::{section_header, settings_card, subsection_header},
            column,
            controls::{SliderNumberParams, card_slider_number, card_toggle, compact_toggle, radio, wrap_tooltip},
            fields::{Combo, Radio},
            hs, row,
        },
    },
};

/// `(label key, width, tooltip key)` per radio, in value order.
type RadioSpec = (&'static str, Option<f64>, Option<&'static str>);

/// One radio group card; `get` returns the checked index and whether the group is enabled.
pub(crate) fn radio_card<T: Copy + Send + 'static>(
    b: &mut Binds<UiShared>,
    cx: &UiCx,
    (key, header, desc): (&str, &str, &str),
    (group, specs, values): (&str, &[RadioSpec], &'static [T]),
    vertical: bool,
    get: impl Fn(&UiShared) -> (usize, bool) + 'static,
    set: fn(&mut UiShared, T),
) -> Result<Border> {
    let panel = if vertical {
        let p = column(4.0)?;
        p.halign(HorizontalAlignment::Right)?;
        p
    } else {
        let p = row(12.0)?;
        p.valign(VerticalAlignment::Center)?;
        p
    };
    let set = cx.edit(set);
    let mut radios: Vec<Radio> = Vec::new();
    for (&(label, width, tip), &value) in specs.iter().zip(values) {
        let set = set.clone();
        let r = radio(group, &t!(label), width, move || set(value))?;
        if let Some(tip) = tip {
            r.rb.tooltip_with(&wrap_tooltip(&t!(tip))?)?;
        }
        children!(panel; r.rb);
        radios.push(r);
    }
    let card = settings_card(key, header, Some(desc), &panel)?;
    b.add(move |ui, _| {
        let (checked, enabled) = get(ui);
        for (i, r) in radios.iter_mut().enumerate() {
            r.set(i == checked, enabled)?;
        }
        Ok(())
    });
    Ok(card.root)
}

/// Informational bar showing `estimate`'s `(VRAM, RAM)` bytes for the draft settings.
pub(crate) fn memory_infobar(b: &mut Binds<UiShared>, estimate: impl Fn(&UiShared) -> (u64, u64) + 'static) -> Result<InfoBar> {
    let bar = InfoBar::new()?;
    bar.SetIsOpen(true)?;
    bar.SetIsClosable(false)?;
    bar.SetTitle(&hs(&t!("ocr.memory")))?;
    let el = bar.clone();
    let mut shown = Prop::default();
    b.add(move |ui, _| {
        let gb = |bytes: u64| match bytes >> 20 {
            mb @ ..1024 => format!("{mb} MB"),
            mb => format!("{:.1} GB", mb as f64 / 1024.0),
        };
        let (vram, ram) = estimate(ui);
        let ram = format!("RAM ≈ {}", gb(ram));
        let line = if vram == 0 {
            ram
        } else {
            format!("VRAM ≈ {} · {ram}", gb(vram))
        };
        shown.set(line, |s| el.SetMessage(&hs(s)))
    });
    Ok(bar)
}

fn merge_pct(
    b: &mut Binds<UiShared>,
    cx: &UiCx,
    p: SliderNumberParams,
    get: fn(&LineMergeConfig) -> f32,
    enabled: fn(&LineMergeConfig) -> bool,
    set: fn(&mut LineMergeConfig, f32),
) -> Result<Border> {
    let lo = (p.min / 100.0) as f32;
    let hi = (p.max / 100.0) as f32;
    Ok(card_slider_number(
        b,
        p,
        move |ui| (f64::from(get(&ui.draft.ocr.line_merge)) * 100.0, enabled(&ui.draft.ocr.line_merge)),
        cx.edit(move |ui, v: f64| set(&mut ui.draft.ocr.line_merge, ((v as f32) / 100.0).clamp(lo, hi))),
    )?
    .root)
}

fn ms_card(
    b: &mut Binds<UiShared>,
    cx: &UiCx,
    p: SliderNumberParams,
    get: fn(&UiShared) -> u64,
    set: fn(&mut UiShared, u64),
) -> Result<Border> {
    let floor = p.min;
    Ok(card_slider_number(b, p, move |ui| (get(ui) as f64, true), cx.edit(move |ui, v: f64| set(ui, v.max(floor) as u64)))?.root)
}

fn merging(m: &LineMergeConfig) -> bool {
    m.mode != LineMergeMode::Off
}

/// Layout and Region group by boxes, so rule thresholds only apply to `Rules`.
fn rules_only(m: &LineMergeConfig) -> bool {
    m.mode == LineMergeMode::Rules
}

pub fn ocr_page(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<StackPanel> {
    const TIERS: [ModelTier; 3] = [ModelTier::Tiny, ModelTier::Small, ModelTier::Medium];
    const DEVICES: [OcrDevice; 3] = [OcrDevice::Webgpu, OcrDevice::Directml, OcrDevice::Cpu];
    const ORDERS: [LineMergeOrder; 2] = [LineMergeOrder::LeftToRightTopToBottom, LineMergeOrder::TopToBottomRightToLeft];

    let model = column(4.0)?;
    children!(model;
        section_header(&t!("ocr.model"))?,
        memory_infobar(b, |ui| {
            let ocr = &ui.draft.ocr;
            memory_estimate(ocr.model_tier, ocr.device, ocr.uses_layout())
        })?,
        radio_card(
            b,
            cx,
            ("ocr-tier", &t!("ocr.model_size"), &t!("ocr.model_size_desc")),
            ("ocr-model-tier", &[("ocr.tiny", Some(64.0), None), ("ocr.small", Some(72.0), None), ("ocr.medium", Some(84.0), None)], &TIERS),
            false,
            |ui| (TIERS.iter().position(|&t| t == ui.draft.ocr.model_tier).unwrap_or(1), true),
            |ui, v| ui.draft.ocr.model_tier = v,
        )?,
        radio_card(
            b,
            cx,
            ("ocr-device", &t!("ocr.device"), &t!("ocr.device_desc")),
            (
                "ocr-device",
                &[
                    ("ocr.webgpu", Some(84.0), Some("ocr.webgpu_tip")),
                    ("ocr.directml", Some(96.0), Some("ocr.directml_tip")),
                    ("ocr.cpu", Some(64.0), Some("ocr.cpu_tip")),
                ],
                &DEVICES,
            ),
            false,
            |ui| (DEVICES.iter().position(|&d| d == ui.draft.ocr.device).unwrap_or(0), true),
            |ui, v| ui.draft.ocr.device = v,
        )?,
    );

    let timing = column(4.0)?;
    children!(timing;
        section_header(&t!("ocr.timing"))?,
        ms_card(
            b,
            cx,
            SliderNumberParams {
                key: "ocr-interval-ms",
                header: t!("ocr.interval"),
                description: Some(t!("ocr.interval_desc")),
                min: 50.0,
                max: 5_000.0,
                step: 50.0,
            },
            |ui| ui.draft.capture.min_interval_ms,
            |ui, v| ui.draft.capture.min_interval_ms = v,
        )?,
        ms_card(
            b,
            cx,
            SliderNumberParams {
                key: "ocr-stable-ms",
                header: t!("ocr.stable"),
                description: Some(t!("ocr.stable_desc")),
                min: 0.0,
                max: 10_000.0,
                step: 50.0,
            },
            |ui| ui.draft.ocr.stable_duration_ms,
            |ui, v| ui.draft.ocr.stable_duration_ms = v,
        )?,
        ms_card(
            b,
            cx,
            SliderNumberParams {
                key: "ocr-max-unstable-ms",
                header: t!("ocr.force"),
                description: Some(t!("ocr.force_desc")),
                min: 0.0,
                max: 15_000.0,
                step: 100.0,
            },
            |ui| ui.draft.ocr.max_unstable_ms,
            |ui, v| ui.draft.ocr.max_unstable_ms = v,
        )?,
        ms_card(
            b,
            cx,
            SliderNumberParams {
                key: "ocr-persist-ms",
                header: t!("ocr.keep"),
                description: Some(t!("ocr.keep_desc")),
                min: 0.0,
                max: 5_000.0,
                step: 50.0,
            },
            |ui| ui.draft.ocr.block_persist_ms,
            |ui, v| ui.draft.ocr.block_persist_ms = v,
        )?,
        ms_card(
            b,
            cx,
            SliderNumberParams {
                key: "ocr-miss-ms",
                header: t!("ocr.drop"),
                description: Some(t!("ocr.drop_desc")),
                min: 0.0,
                max: 10_000.0,
                step: 50.0,
            },
            |ui| ui.draft.ocr.block_max_miss_ms,
            |ui, v| ui.draft.ocr.block_max_miss_ms = v,
        )?,
    );

    let detection = column(4.0)?;
    children!(detection;
        section_header(&t!("ocr.detection"))?,
        card_slider_number(
            b,
            SliderNumberParams {
                key: "ocr-confidence",
                header: t!("ocr.confidence"),
                description: Some(t!("ocr.confidence_desc")),
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            |ui| (f64::from(ui.draft.ocr.confidence_threshold), true),
            cx.edit(|ui, v: f64| ui.draft.ocr.confidence_threshold = v.clamp(0.0, 1.0) as f32),
        )?
        .root,
        card_toggle(
            b,
            ("ocr-filter-single", &t!("ocr.filter_single"), Some(&t!("ocr.filter_single_desc"))),
            |ui| ui.draft.ocr.filter_single_char,
            cx.edit(|ui, v| ui.draft.ocr.filter_single_char = v),
        )?,
    );

    let combo_card = |b: &mut Binds<UiShared>,
                      (key, header, desc): (&str, &str, &str),
                      items: [&'static str; 4],
                      get: fn(&LineMergeConfig) -> (usize, bool),
                      set: fn(&mut LineMergeConfig, usize) -> bool|
     -> Result<Border> {
        let cx = cx.clone();
        let mut combo = Combo::new("", move |idx| {
            let Some(idx) = idx else {
                return;
            };
            cx.with_mut(|ui| {
                if set(&mut ui.draft.ocr.line_merge, idx) {
                    mark_dirty(ui);
                }
            });
        })?;
        combo.cb.width(160.0)?.valign(VerticalAlignment::Center)?;
        let card = settings_card(key, header, Some(desc), &combo.cb)?;
        b.add(move |ui, _| {
            let labels = items.iter().filter(|k| !k.is_empty()).map(|k| t!(*k).into_owned()).collect();
            let (selected, enabled) = get(&ui.draft.ocr.line_merge);
            combo.set(labels, Some(selected), enabled)
        });
        Ok(card.root)
    };

    let merge_join = column(4.0)?;
    children!(merge_join;
        section_header(&t!("ocr.line_merge"))?,
        // Item order must match `LineMergeMode` declaration order.
        combo_card(
            b,
            ("ocr-line-merge", &t!("ocr.merge_lines"), &t!("ocr.merge_lines_desc")),
            ["ocr.merge_mode_off", "ocr.merge_mode_rules", "ocr.merge_mode_layout", "ocr.merge_mode_region"],
            |m| (m.mode as usize, true),
            |m, idx| {
                let choice = match idx {
                    0 => LineMergeMode::Off,
                    1 => LineMergeMode::Rules,
                    2 => LineMergeMode::Layout,
                    3 => LineMergeMode::Region,
                    _ => return false,
                };
                let changed = m.mode != choice;
                m.mode = choice;
                changed
            },
        )?,
        combo_card(
            b,
            ("ocr-merge-join", &t!("ocr.join"), &t!("ocr.join_desc")),
            ["ocr.join_none", "ocr.join_space", "ocr.join_newline", ""],
            |m| (m.join as usize, merging(m)),
            |m, idx| {
                let choice = match idx {
                    0 => LineJoin::None,
                    1 => LineJoin::Space,
                    2 => LineJoin::Newline,
                    _ => return false,
                };
                let changed = m.join != choice;
                m.join = choice;
                changed
            },
        )?,
        radio_card(
            b,
            cx,
            ("ocr-merge-order", &t!("ocr.reading_order"), &t!("ocr.reading_order_desc")),
            ("ocr-merge-order", &[("ocr.order_rows", None, None), ("ocr.order_cols", None, None)], &ORDERS),
            true,
            |ui| {
                let m = &ui.draft.ocr.line_merge;
                (ORDERS.iter().position(|&o| o == m.order).unwrap_or(0), merging(m))
            },
            |ui, v| ui.draft.ocr.line_merge.order = v,
        )?,
    );

    // Row / column tolerances also band the reading order, so every merging mode uses them.
    let merge_row = column(4.0)?;
    children!(merge_row;
        subsection_header(&t!("ocr.merge_horizontal"))?,
        merge_pct(
            b,
            cx,
            SliderNumberParams {
                key: "merge-word-gap",
                header: t!("ocr.word_gap"),
                description: Some(t!("ocr.word_gap_desc")),
                min: 0.0,
                max: 8.0,
                step: 0.1,
            },
            |m| m.word_gap,
            rules_only,
            |m, x| m.word_gap = x,
        )?,
        merge_pct(
            b,
            cx,
            SliderNumberParams {
                key: "merge-row-tolerance",
                header: t!("ocr.row_tolerance"),
                description: Some(t!("ocr.row_tolerance_desc")),
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            |m| m.row_tolerance,
            merging,
            |m, x| m.row_tolerance = x,
        )?,
    );

    let mut split = compact_toggle(cx.edit(|ui, v| ui.draft.ocr.line_merge.split_short_above_wide = v))?;
    let split_card = settings_card("ocr-merge-split-short", &t!("ocr.split_short"), Some(&t!("ocr.split_short_desc")), &split.ts)?;
    b.add(move |ui, _| {
        let m = &ui.draft.ocr.line_merge;
        split.set(m.split_short_above_wide, rules_only(m))
    });
    let merge_paragraph = column(4.0)?;
    children!(merge_paragraph;
        subsection_header(&t!("ocr.merge_vertical"))?,
        merge_pct(
            b,
            cx,
            SliderNumberParams {
                key: "merge-line-gap",
                header: t!("ocr.line_gap"),
                description: Some(t!("ocr.line_gap_desc")),
                min: 0.0,
                max: 8.0,
                step: 0.1,
            },
            |m| m.line_gap,
            rules_only,
            |m, x| m.line_gap = x,
        )?,
        merge_pct(
            b,
            cx,
            SliderNumberParams {
                key: "merge-column-tolerance",
                header: t!("ocr.column_tolerance"),
                description: Some(t!("ocr.column_tolerance_desc")),
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            |m| m.column_tolerance,
            merging,
            |m, x| m.column_tolerance = x,
        )?,
        split_card.root,
        merge_pct(
            b,
            cx,
            SliderNumberParams {
                key: "merge-width-diff",
                header: t!("ocr.width_diff"),
                description: Some(t!("ocr.width_diff_desc")),
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m| m.width_diff,
            |m| rules_only(m) && m.split_short_above_wide,
            |m, x| m.width_diff = x,
        )?,
    );

    let merge_shared = column(4.0)?;
    children!(merge_shared;
        subsection_header(&t!("ocr.merge_general"))?,
        merge_pct(
            b,
            cx,
            SliderNumberParams {
                key: "merge-size-diff",
                header: t!("ocr.size_diff"),
                description: Some(t!("ocr.size_diff_desc")),
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m| m.size_diff,
            rules_only,
            |m, x| m.size_diff = x,
        )?,
    );

    let line_merge = column(4.0)?;
    children!(line_merge; merge_join, merge_row, merge_paragraph, merge_shared);
    let body = column(8.0)?;
    children!(body; model, timing, detection, line_merge);
    Ok(body)
}
