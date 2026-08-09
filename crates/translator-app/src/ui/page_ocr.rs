//! OCR / capture settings.

use std::sync::{Arc, Mutex};

use translator_core::ModelTier;
use windows_reactor::*;

use crate::ui::{
    chrome::{section_header, settings_card, settings_expander, settings_page_shell},
    controls::{SliderNumberParams, card_slider_number, card_toggle, row_slider_number, row_toggle},
    shared::{Snapshot, UiShared, mark_dirty},
};

/// Bind a line-merge f32 field from a slider/number value.
fn set_merge_f32(shared: &Arc<Mutex<UiShared>>, set: impl FnOnce(&mut translator_core::LineMergeConfig, f32), v: f64) {
    if let Ok(mut ui) = shared.lock() {
        set(&mut ui.draft.ocr.line_merge, v as f32);
        mark_dirty(&mut ui);
    }
}

pub fn ocr_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> Element {
    let s_tier = Arc::clone(shared);
    let s_conf = Arc::clone(shared);
    let s_stable = Arc::clone(shared);
    let s_max_unstable = Arc::clone(shared);
    let s_interval = Arc::clone(shared);
    let s_filter = Arc::clone(shared);
    let s_persist = Arc::clone(shared);
    let s_miss = Arc::clone(shared);
    let s_merge = Arc::clone(shared);
    let s_m1 = Arc::clone(shared);
    let s_m2 = Arc::clone(shared);
    let s_m3 = Arc::clone(shared);
    let s_m4 = Arc::clone(shared);
    let s_m5 = Arc::clone(shared);
    let s_m6 = Arc::clone(shared);
    let s_m7 = Arc::clone(shared);
    let s_m8 = Arc::clone(shared);
    let s_m9 = Arc::clone(shared);
    let s_m10 = Arc::clone(shared);
    let s_m11 = Arc::clone(shared);
    let s_m12 = Arc::clone(shared);
    let bump_a = bump.clone();
    let bump_b = bump.clone();
    let bump_c = bump.clone();
    let bump_c2 = bump.clone();
    let bump_d = bump.clone();
    let bump_e = bump.clone();
    let bump_f = bump.clone();
    let bump_g = bump.clone();
    let bump_i = bump.clone();
    let bump_j = bump.clone();
    let bump_k = bump.clone();
    let bump_l = bump.clone();
    let bump_m = bump.clone();
    let bump_n = bump.clone();
    let bump_o = bump.clone();
    let bump_p = bump.clone();
    let bump_q = bump.clone();
    let bump_r = bump.clone();
    let bump_s = bump.clone();
    let bump_t = bump.clone();
    let bump_u = bump.clone();

    let model = vstack((
        section_header("Model"),
        // Compact but readable: default RadioButton MinWidth (~120) spreads
        // short labels too far; zero padding/min-width crushes circle+text.
        // Cap width near content size and space items with hstack only.
        settings_card("ocr-tier", "Model size", Some("Smaller is faster; larger is more accurate. Reloads on Save."), {
            let idx = snap.model_tier_idx;
            let pick = move |choice: i32| {
                let s = Arc::clone(&s_tier);
                let bump = bump_a.clone();
                move || {
                    if let Ok(mut ui) = s.lock() {
                        ui.draft.ocr.model_tier = match choice {
                            0 => ModelTier::Tiny,
                            2 => ModelTier::Medium,
                            _ => ModelTier::Small,
                        };
                        mark_dirty(&mut ui);
                    }
                    bump.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_conf.lock() {
                    ui.draft.ocr.confidence_threshold = v.clamp(0.0, 1.0) as f32;
                    mark_dirty(&mut ui);
                }
                bump_b.call(|n| n.wrapping_add(1));
            },
        ),
        card_toggle(
            "ocr-filter-single",
            "Ignore single characters",
            Some("Drop lone single-character detections."),
            snap.filter_single,
            move |v| {
                if let Ok(mut ui) = s_filter.lock() {
                    ui.draft.ocr.filter_single_char = v;
                    mark_dirty(&mut ui);
                }
                bump_e.call(|n| n.wrapping_add(1));
            },
        ),
    ))
    .spacing(4.0);

    let s_exp_merge = Arc::clone(shared);
    let s_exp_adv = Arc::clone(shared);
    let bump_exp = bump.clone();
    let bump_exp2 = bump.clone();

    // Flat rows inside Expander (SettingsExpander.Items style) — no nested cards.
    let line_merge_body = vstack((
        row_toggle(
            "ocr-line-merge",
            "Merge lines into paragraphs",
            Some("Join stacked OCR lines that look like one paragraph (geometry only)."),
            snap.merge_enabled,
            move |v| {
                if let Ok(mut ui) = s_merge.lock() {
                    ui.draft.ocr.line_merge.enabled = v;
                    mark_dirty(&mut ui);
                }
                bump_i.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m1, |m, x| m.max_gap_ratio = x.max(0.05), v);
                bump_j.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m2, |m, x| m.min_gap_ratio = x, v);
                bump_k.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m3, |m, x| m.gap_slack = x.max(0.5), v);
                bump_l.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m4, |m, x| m.list_gap_min_ratio = x.max(0.0), v);
                bump_m.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m5, |m, x| m.wrap_width_ratio = x.max(1.0), v);
                bump_n.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m6, |m, x| m.height_ratio_min = x.clamp(0.1, 1.0), v);
                bump_o.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m7, |m, x| m.left_align_ratio = x.max(0.05), v);
                bump_p.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m8, |m, x| m.short_max_gap_ratio = x.max(0.0), v);
                bump_q.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_m9.lock() {
                    ui.draft.ocr.line_merge.list_min_peers = v.max(2.0) as u32;
                    mark_dirty(&mut ui);
                }
                bump_r.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m10, |m, x| m.compact_aspect_max = x.max(0.5), v);
                bump_s.call(|n| n.wrapping_add(1));
            },
        ),
        row_toggle(
            "merge-nameplate",
            "Keep nameplates separate",
            Some("Short narrow box above a wider line stays its own block."),
            snap.merge_keep_nameplate,
            move |v| {
                if let Ok(mut ui) = s_m11.lock() {
                    ui.draft.ocr.line_merge.keep_speaker_separate = v;
                    mark_dirty(&mut ui);
                }
                bump_t.call(|n| n.wrapping_add(1));
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
            move |v| {
                set_merge_f32(&s_m12, |m, x| m.nameplate_body_width_ratio = x.max(1.0), v);
                bump_u.call(|n| n.wrapping_add(1));
            },
        ),
    ))
    .spacing(0.0)
    .horizontal_alignment(HorizontalAlignment::Stretch);

    let line_merge = settings_expander(
        "ocr-exp-line-merge",
        "Line merge",
        snap.expand_line_merge,
        move |open| {
            if let Ok(mut ui) = s_exp_merge.lock() {
                ui.expand_line_merge = open;
            }
            bump_exp.call(|n| n.wrapping_add(1));
        },
        line_merge_body,
    );

    let line_merge_advanced = settings_expander(
        "ocr-exp-line-merge-adv",
        "Line merge · advanced",
        snap.expand_line_merge_adv,
        move |open| {
            if let Ok(mut ui) = s_exp_adv.lock() {
                ui.expand_line_merge_adv = open;
            }
            bump_exp2.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_stable.lock() {
                    ui.draft.ocr.stable_duration_ms = v.max(0.0) as u64;
                    mark_dirty(&mut ui);
                }
                bump_c.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_max_unstable.lock() {
                    ui.draft.ocr.max_unstable_ms = v.max(0.0) as u64;
                    mark_dirty(&mut ui);
                }
                bump_c2.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_interval.lock() {
                    ui.draft.capture.min_interval_ms = v.max(50.0) as u64;
                    mark_dirty(&mut ui);
                }
                bump_d.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_persist.lock() {
                    ui.draft.ocr.block_persist_ms = v.max(0.0) as u64;
                    mark_dirty(&mut ui);
                }
                bump_f.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_miss.lock() {
                    ui.draft.ocr.block_max_miss_ms = v.max(0.0) as u64;
                    mark_dirty(&mut ui);
                }
                bump_g.call(|n| n.wrapping_add(1));
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
