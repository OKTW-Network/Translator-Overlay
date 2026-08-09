//! Translation / context settings.

use std::sync::{Arc, Mutex};

use windows_reactor::*;

use crate::ui::{
    chrome::{section_header, settings_card_stack, settings_page_shell},
    controls::{SliderNumberParams, card_slider_number, card_text},
    shared::{Snapshot, UiShared, mark_dirty},
};

pub fn translation_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> Element {
    let s_src = Arc::clone(shared);
    let s_dst = Arc::clone(shared);
    let s_hist = Arc::clone(shared);
    let s_conv = Arc::clone(shared);
    let s_sys = Arc::clone(shared);
    let bump_a = bump.clone();
    let bump_b = bump.clone();
    let bump_c = bump.clone();
    let bump_d = bump.clone();
    let bump_e = bump.clone();

    let languages = vstack((
        section_header("Languages"),
        card_text(
            "tr-source-lang",
            "Source language",
            Some("Language on screen, or auto."),
            snap.source_lang_draft.clone(),
            "auto",
            move |v| {
                if let Ok(mut ui) = s_src.lock() {
                    ui.draft.translation.source_lang = v;
                    mark_dirty(&mut ui);
                }
                bump_a.call(|n| n.wrapping_add(1));
            },
        ),
        card_text(
            "tr-target-lang",
            "Target language",
            Some("Language for the translation."),
            snap.target_lang_draft.clone(),
            "e.g. zh-TW",
            move |v| {
                if let Ok(mut ui) = s_dst.lock() {
                    ui.draft.translation.target_lang = v;
                    mark_dirty(&mut ui);
                }
                bump_b.call(|n| n.wrapping_add(1));
            },
        ),
    ))
    .spacing(4.0);

    let context = vstack((
        section_header("History"),
        card_slider_number(
            SliderNumberParams {
                key: "tr-history-max",
                header: "Recent translations".into(),
                description: Some("How many past translations to keep.".into()),
                value: snap.history_max,
                min: 1.0,
                max: 100.0,
                step: 1.0,
            },
            move |v| {
                if let Ok(mut ui) = s_hist.lock() {
                    ui.draft.translation.history_max_items = v.max(1.0) as usize;
                    mark_dirty(&mut ui);
                }
                bump_c.call(|n| n.wrapping_add(1));
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "tr-conv-max",
                header: "Chat context turns".into(),
                description: Some("How much conversation history the model sees.".into()),
                value: snap.conv_max,
                min: 1.0,
                max: 200.0,
                step: 1.0,
            },
            move |v| {
                if let Ok(mut ui) = s_conv.lock() {
                    ui.draft.translation.conversation_max_turns = v.max(1.0) as usize;
                    mark_dirty(&mut ui);
                }
                bump_d.call(|n| n.wrapping_add(1));
            },
        ),
    ))
    .spacing(4.0);

    let prompt = vstack((
        section_header("Prompt"),
        settings_card_stack(
            "tr-system-prompt",
            "System prompt",
            Some("Leave empty to use the built-in prompt."),
            text_box(snap.system_prompt.clone())
                .multiline()
                .height(120.0)
                .placeholder_text("Built-in prompt")
                .on_text_changed(move |v: String| {
                    if let Ok(mut ui) = s_sys.lock() {
                        let t = v.trim().to_string();
                        ui.draft.translation.system_prompt = if t.is_empty() { None } else { Some(t) };
                        mark_dirty(&mut ui);
                    }
                    bump_e.call(|n| n.wrapping_add(1));
                }),
        ),
    ))
    .spacing(4.0);

    settings_page_shell(shared, snap, bump, vstack((languages, context, prompt)).spacing(8.0).into())
}
