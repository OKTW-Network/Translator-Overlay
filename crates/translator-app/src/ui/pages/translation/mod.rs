//! Translation settings: languages, history, cache, and system prompt.

mod dictionary;

use rust_i18n::t;
use translator_core::{TRANSLATION_CACHE_MAX_CAP, TRANSLATION_CACHE_MAX_MIN};
use translator_winui::{HorizontalAlignment, Layout, Result, StackPanel, TextWrapping, VerticalAlignment};

use crate::{
    children,
    pipeline::PipelineCommand,
    ui::{
        pages::translation::dictionary::dictionary_editor_dialog,
        state::{UiCx, UiShared, mark_dirty, open_dictionary_editor},
        widgets::{
            Binds, Prop, button,
            cards::{section_header, settings_card, settings_card_stack},
            column,
            controls::{SliderNumberParams, card_slider_number, card_text, card_toggle},
            fields::{Combo, TextField},
            hs, icon_button, row,
        },
    },
};

pub fn translation_page(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<StackPanel> {
    let languages = column(4.0)?;
    children!(languages;
        section_header(&t!("tr.languages"))?,
        card_text(
            b,
            ("tr-source-lang", &t!("tr.source"), Some(&t!("tr.source_desc"))),
            &t!("tr.source_placeholder"),
            |ui| ui.draft.translation.source_lang.clone(),
            cx.edit(|ui, v| ui.draft.translation.source_lang = v),
        )?,
        card_text(
            b,
            ("tr-target-lang", &t!("tr.target"), Some(&t!("tr.target_desc"))),
            &t!("tr.target_placeholder"),
            |ui| ui.draft.translation.target_lang.clone(),
            cx.edit(|ui, v| ui.draft.translation.target_lang = v),
        )?,
    );

    let context = column(4.0)?;
    children!(context;
        section_header(&t!("tr.history"))?,
        card_slider_number(
            b,
            SliderNumberParams {
                key: "tr-history-max",
                header: t!("tr.history_max"),
                description: Some(t!("tr.history_max_desc")),
                min: 1.0,
                max: 100.0,
                step: 1.0,
            },
            |ui| (ui.draft.translation.history_max_items as f64, true),
            cx.edit(|ui, v: f64| ui.draft.translation.history_max_items = v.max(1.0) as usize),
        )?
        .root,
        card_slider_number(
            b,
            SliderNumberParams {
                key: "tr-conv-max",
                header: t!("tr.conv_max"),
                description: Some(t!("tr.conv_max_desc")),
                min: 1.0,
                max: 200.0,
                step: 1.0,
            },
            |ui| (ui.draft.translation.conversation_max_turns as f64, true),
            cx.edit(|ui, v: f64| ui.draft.translation.conversation_max_turns = v.max(1.0) as usize),
        )?
        .root,
    );

    let clear = button(&t!("tr.clear_cache"), {
        let cx = cx.clone();
        move || cx.send_cmd(PipelineCommand::ClearTranslationCache)
    })?;
    clear.tooltip(&t!("tr.clear_cache_tip"))?;
    let clear_card = settings_card("tr-cache-clear", &t!("tr.clear_cache"), Some(" "), &clear)?;
    let mut usage = Prop::default();
    b.add(move |ui, _| {
        let cached = ui.state.read().translation_cache_len;
        usage.set((cached, ui.draft.translation.cache_max_entries), |&(cached, max)| {
            clear.SetIsEnabled(cached > 0)?;
            if let Some(desc) = &clear_card.desc {
                desc.SetText(&hs(&t!("tr.cache_usage", cached = cached, max = max)))?;
            }
            Ok(())
        })
    });
    let cache = column(4.0)?;
    children!(cache;
        section_header(&t!("tr.cache"))?,
        card_toggle(
            b,
            ("tr-cache-enabled", &t!("tr.cache_enabled"), Some(&t!("tr.cache_enabled_desc"))),
            |ui| ui.draft.translation.cache_enabled,
            cx.edit(|ui, on| ui.draft.translation.cache_enabled = on),
        )?,
        card_slider_number(
            b,
            SliderNumberParams {
                key: "tr-cache-max",
                header: t!("tr.cache_size"),
                description: Some(t!("tr.cache_size_desc")),
                min: TRANSLATION_CACHE_MAX_MIN as f64,
                max: TRANSLATION_CACHE_MAX_CAP as f64,
                step: 1.0,
            },
            |ui| (ui.draft.translation.cache_max_entries as f64, true),
            cx.edit(|ui, v: f64| {
                ui.draft.translation.cache_max_entries =
                    v.round().clamp(TRANSLATION_CACHE_MAX_MIN as f64, TRANSLATION_CACHE_MAX_CAP as f64) as usize;
            }),
        )?
        .root,
        clear_card.root,
    );

    let mut dictionary = Combo::new("", {
        let cx = cx.clone();
        move |idx| {
            let Some(idx) = idx else {
                return;
            };
            cx.with_mut(|ui| {
                let name = if idx == 0 {
                    None
                } else {
                    ui.dictionary.names.get(idx - 1).cloned()
                };
                if ui.draft.translation.dictionary_profile != name {
                    ui.draft.translation.dictionary_profile = name;
                    mark_dirty(ui);
                }
            });
        }
    })?;
    dictionary.cb.width(180.0)?.min_width(140.0)?.valign(VerticalAlignment::Center)?;
    let edit = icon_button("\u{E70F}", {
        let cx = cx.clone();
        move || cx.with_mut(open_dictionary_editor)
    })?;
    edit.automation_id("btn-edit-dictionary")?
        .automation_name(&t!("tr.edit_dictionary_tip"))?
        .tooltip(&t!("tr.edit_dictionary_tip"))?;
    let dictionary_controls = row(8.0)?;
    dictionary_controls
        .valign(VerticalAlignment::Center)?
        .halign(HorizontalAlignment::Right)?;
    children!(dictionary_controls; dictionary.cb, edit);
    let dictionary_card = settings_card("tr-dictionary", &t!("tr.dictionary"), Some(&t!("tr.dictionary_desc")), &dictionary_controls)?;
    b.add(move |ui, _| {
        let names = &ui.dictionary.names;
        let mut items = vec![t!("tr.dictionary_none").into_owned()];
        items.extend(names.iter().cloned());
        let index = ui
            .draft
            .translation
            .dictionary_profile
            .as_ref()
            .and_then(|name| names.iter().position(|n| n == name).map(|i| i + 1))
            .unwrap_or(0);
        dictionary.set(items, Some(index), true)
    });

    let prompt_field = TextField::new(
        &t!("tr.prompt_placeholder"),
        cx.edit(|ui, v: String| {
            let t = v.trim().to_string();
            ui.draft.translation.system_prompt = if t.is_empty() { None } else { Some(t) };
        }),
    )?;
    prompt_field.tb.SetAcceptsReturn(true)?;
    prompt_field.tb.SetTextWrapping(TextWrapping::Wrap)?;
    prompt_field.tb.height(120.0)?;
    let prompt_card =
        settings_card_stack("tr-system-prompt", &t!("tr.system_prompt"), Some(&t!("tr.system_prompt_desc")), &prompt_field.tb)?;
    b.add(move |ui, _| prompt_field.set(ui.draft.translation.system_prompt.as_deref().unwrap_or_default()));
    let prompt = column(4.0)?;
    children!(prompt; section_header(&t!("tr.prompt"))?, dictionary_card.root, prompt_card.root);

    dictionary_editor_dialog(b, cx)?;
    let body = column(8.0)?;
    children!(body; languages, context, cache, prompt);
    Ok(body)
}
