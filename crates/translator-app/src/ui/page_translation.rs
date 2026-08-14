//! Translation / context settings.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{TRANSLATION_CACHE_SLIDER_MAX, TRANSLATION_CACHE_SLIDER_MIN};
use windows_reactor::{LayoutExt, StackPanel, TooltipExt, Updater, button, text_box, vstack};

use crate::{
    pipeline::PipelineCommand,
    ui::{
        chrome::{section_header, settings_card, settings_card_stack, settings_page_shell},
        controls::{SliderNumberParams, card_slider_number, card_text, card_toggle},
        shared::{Snapshot, UiCx, UiShared, mark_dirty},
    },
};

pub fn translation_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);

    let languages = vstack((
        section_header("Languages"),
        card_text("tr-source-lang", "Source language", Some("Language on screen, or auto."), snap.source_lang_draft.clone(), "auto", {
            let cx = cx.clone();
            move |v| {
                cx.with_mut(|ui| {
                    ui.draft.translation.source_lang = v;
                    mark_dirty(ui);
                });
            }
        }),
        card_text(
            "tr-target-lang",
            "Target language",
            Some("Language for the translation."),
            snap.target_lang_draft.clone(),
            "e.g. zh-TW",
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.translation.target_lang = v;
                        mark_dirty(ui);
                    });
                }
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
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.translation.history_max_items = v.max(1.0) as usize;
                        mark_dirty(ui);
                    });
                }
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
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.translation.conversation_max_turns = v.max(1.0) as usize;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let cache = vstack((
        section_header("Cache"),
        card_toggle(
            "tr-cache-enabled",
            "Enable translation cache",
            Some("Reuse translations for source text already seen this session. Skips the API for repeats."),
            snap.cache_enabled,
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.draft.translation.cache_enabled = on;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "tr-cache-max",
                header: "Cache size".into(),
                description: Some("Max unique phrases kept in memory. Full cache evicts the least-used first.".into()),
                value: snap.cache_max,
                min: TRANSLATION_CACHE_SLIDER_MIN as f64,
                max: TRANSLATION_CACHE_SLIDER_MAX as f64,
                step: 1.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.translation.cache_max_entries = v
                            .round()
                            .clamp(TRANSLATION_CACHE_SLIDER_MIN as f64, TRANSLATION_CACHE_SLIDER_MAX as f64)
                            as usize;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        settings_card(
            "tr-cache-clear",
            "Clear cache",
            Some(&format!(
                "{} of {} phrases cached this session. Closing the app also clears it.",
                snap.cache_len, snap.cache_max as usize
            )),
            button("Clear cache")
                .enabled(snap.cache_len > 0)
                .tooltip("Drop all cached translations. Does not clear chat history.")
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::ClearTranslationCache)
                }),
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
                .on_text_changed({
                    let cx = cx.clone();
                    move |v: String| {
                        cx.with_mut(|ui| {
                            let t = v.trim().to_string();
                            ui.draft.translation.system_prompt = if t.is_empty() { None } else { Some(t) };
                            mark_dirty(ui);
                        });
                    }
                }),
        ),
    ))
    .spacing(4.0);

    settings_page_shell(shared, snap, bump, vstack((languages, context, cache, prompt)).spacing(8.0))
}
