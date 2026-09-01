//! Translation / context settings.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{TRANSLATION_CACHE_MAX_CAP, TRANSLATION_CACHE_MAX_MIN};
use windows_reactor::{LayoutExt, StackPanel, TooltipExt, Updater, button, text_box, vstack};

use crate::{
    pipeline::PipelineCommand,
    ui::{
        chrome::{section_header, settings_card, settings_card_stack, settings_page_shell},
        controls::{SliderNumberParams, card_slider_number, card_text, card_toggle},
        shared::{ChromeSnap, UiCx, UiShared, mark_dirty},
    },
};

pub fn translation_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);
    let (translation, cache_len) = {
        let ui = shared.lock();
        let cache_len = ui.state.read().translation_cache_len;
        (ui.draft.translation.clone(), cache_len)
    };

    let languages = vstack((
        section_header("Languages"),
        card_text("tr-source-lang", "Source language", Some("Language on screen, or auto."), translation.source_lang.clone(), "auto", {
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
            translation.target_lang.clone(),
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
                value: translation.history_max_items as f64,
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
                value: translation.conversation_max_turns as f64,
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
            translation.cache_enabled,
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
                value: translation.cache_max_entries as f64,
                min: TRANSLATION_CACHE_MAX_MIN as f64,
                max: TRANSLATION_CACHE_MAX_CAP as f64,
                step: 1.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.translation.cache_max_entries =
                            v.round().clamp(TRANSLATION_CACHE_MAX_MIN as f64, TRANSLATION_CACHE_MAX_CAP as f64) as usize;
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
                cache_len, translation.cache_max_entries
            )),
            button("Clear cache")
                .enabled(cache_len > 0)
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
            text_box(translation.system_prompt.clone().unwrap_or_default())
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

    settings_page_shell(shared, chrome, bump, vstack((languages, context, cache, prompt)).spacing(8.0))
}
