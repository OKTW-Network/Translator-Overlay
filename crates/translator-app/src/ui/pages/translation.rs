//! Translation settings: languages, history, cache, and system prompt.

use std::sync::Arc;

use parking_lot::Mutex;
use rust_i18n::t;
use translator_core::{TRANSLATION_CACHE_MAX_CAP, TRANSLATION_CACHE_MAX_MIN};
use windows_reactor::{
    Button, ChildrenControl, ContentControl, LayoutControl, LocalSender, StackPanel, TextBox, TextWrapping, TooltipExt, View,
};

use crate::{
    pipeline::PipelineCommand,
    ui::{
        cards::{section_header, settings_card, settings_card_stack},
        chrome::settings_page_shell,
        controls::{SliderNumberParams, card_slider_number, card_text, card_toggle},
        shared::{AppMsg, ChromeSnap, UiCx, UiShared, mark_dirty},
    },
};

pub fn translation_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let (translation, cache_len) = {
        let ui = shared.lock();
        let cache_len = ui.state.read().translation_cache_len;
        (ui.draft.translation.clone(), cache_len)
    };

    let source_desc = t!("tr.source_desc");
    let target_desc = t!("tr.target_desc");
    let languages = StackPanel::new().spacing(4.0).children((
        section_header(t!("tr.languages")),
        card_text("tr-source-lang", t!("tr.source"), Some(&source_desc), translation.source_lang.clone(), t!("tr.source_placeholder"), {
            let cx = cx.clone();
            move |v| {
                cx.with_mut(|ui| {
                    ui.draft.translation.source_lang = v;
                    mark_dirty(ui);
                });
            }
        }),
        card_text("tr-target-lang", t!("tr.target"), Some(&target_desc), translation.target_lang.clone(), t!("tr.target_placeholder"), {
            let cx = cx.clone();
            move |v| {
                cx.with_mut(|ui| {
                    ui.draft.translation.target_lang = v;
                    mark_dirty(ui);
                });
            }
        }),
    ));

    let context = StackPanel::new().spacing(4.0).children((
        section_header(t!("tr.history")),
        card_slider_number(
            SliderNumberParams {
                key: "tr-history-max",
                header: t!("tr.history_max"),
                description: Some(t!("tr.history_max_desc")),
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
                header: t!("tr.conv_max"),
                description: Some(t!("tr.conv_max_desc")),
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
    ));

    let cache = StackPanel::new().spacing(4.0).children((
        section_header(t!("tr.cache")),
        card_toggle("tr-cache-enabled", t!("tr.cache_enabled"), Some(&t!("tr.cache_enabled_desc")), translation.cache_enabled, {
            let cx = cx.clone();
            move |on| {
                cx.with_mut(|ui| {
                    ui.draft.translation.cache_enabled = on;
                    mark_dirty(ui);
                });
            }
        }),
        card_slider_number(
            SliderNumberParams {
                key: "tr-cache-max",
                header: t!("tr.cache_size"),
                description: Some(t!("tr.cache_size_desc")),
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
            t!("tr.clear_cache"),
            Some(&t!("tr.cache_usage", cached = cache_len, max = translation.cache_max_entries)),
            Button::new()
                .is_enabled(cache_len > 0)
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::ClearTranslationCache)
                })
                .content(t!("tr.clear_cache").as_ref())
                .tooltip(t!("tr.clear_cache_tip")),
        ),
    ));

    let prompt = StackPanel::new().spacing(4.0).children((
        section_header(t!("tr.prompt")),
        settings_card_stack(
            "tr-system-prompt",
            t!("tr.system_prompt"),
            Some(&t!("tr.system_prompt_desc")),
            TextBox::new()
                .text(translation.system_prompt.clone().unwrap_or_default())
                .accepts_return(true)
                .text_wrapping(TextWrapping::Wrap)
                .height(120.0)
                .placeholder_text(t!("tr.prompt_placeholder"))
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
    ));

    settings_page_shell(shared, chrome, bump, StackPanel::new().spacing(8.0).children((languages, context, cache, prompt)))
}
