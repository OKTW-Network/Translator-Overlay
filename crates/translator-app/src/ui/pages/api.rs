//! API settings: provider, connection, sampling, and named profiles.

use std::borrow::Cow;

use rust_i18n::t;
use translator_core::{ApiConfig, HttpApi, ModelProvider, OcrDevice, ServiceTier};
use translator_translate::{BUILTIN_MODELS, find_builtin_model};
use translator_winui::{
    Border, Flyout, FlyoutPlacementMode, HorizontalAlignment, InfoBar, Interface, Layout, Result, StackPanel, Style, TextWrapping,
    UIElement, VerticalAlignment, XamlReader,
};

use crate::{
    children,
    ui::{
        pages::ocr::{memory_infobar, radio_card},
        state::{API_PROFILES, NamedDialog, UiCx, UiShared, load_api_profile, mark_dirty, request_model_list, selected_api_profile},
        widgets::{
            Binds, Prop, button,
            cards::{Card, SEMI_BOLD, section_header, settings_card},
            column,
            controls::{
                ModelSuggestParams, OptionalTextParams, SliderNumberParams, card_model_suggest, card_password, card_slider_number,
                card_text, card_toggle, optional_number_row, optional_slider_row, optional_text_row, radio,
            },
            fields::Combo,
            hs, icon_button,
            named::{name_entry_row, named_combo, named_confirm_dialog, named_delete_button},
            row, text,
        },
    },
};

fn provider_label(provider: ModelProvider) -> Cow<'static, str> {
    Cow::Borrowed(match provider {
        ModelProvider::OpenaiCompatible => "OpenAI-compatible",
        ModelProvider::GrokCli => "Grok CLI",
        ModelProvider::OpenCodeCli => "OpenCode CLI",
        ModelProvider::CodexCli => "Codex CLI",
        ModelProvider::ClaudeCli => "Claude Code",
        ModelProvider::Builtin => return t!("api.provider_builtin"),
    })
}

const PROVIDERS: [ModelProvider; 6] = [
    ModelProvider::Builtin,
    ModelProvider::OpenaiCompatible,
    ModelProvider::GrokCli,
    ModelProvider::OpenCodeCli,
    ModelProvider::CodexCli,
    ModelProvider::ClaudeCli,
];

fn builtin_section(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<(Card, Border, Border, InfoBar)> {
    let mut model = Combo::new("", {
        let cx = cx.clone();
        move |idx| {
            let Some(spec) = idx.and_then(|i| BUILTIN_MODELS.get(i)) else {
                return;
            };
            cx.with_mut(|ui| {
                if ui.draft.api.builtin_model != spec.id {
                    ui.draft.api.builtin_model = spec.id.to_string();
                    mark_dirty(ui);
                }
            });
        }
    })?;
    model.cb.width(280.0)?.valign(VerticalAlignment::Center)?;
    let model_card = settings_card("api-builtin-model", &t!("api.builtin_model"), Some(&t!("api.builtin_model_desc")), &model.cb)?;
    b.add(move |ui, _| {
        let items = BUILTIN_MODELS.iter().map(|m| m.label.to_string()).collect();
        let selected = find_builtin_model(&ui.draft.api.builtin_model).and_then(|s| BUILTIN_MODELS.iter().position(|m| m.id == s.id));
        model.set(items, selected, true)
    });
    // No DirectML: its attention kernel breaks Gemma 4 (see translator-translate's builtin module).
    const DEVICES: [OcrDevice; 2] = [OcrDevice::Webgpu, OcrDevice::Cpu];
    let device_card = radio_card(
        b,
        cx,
        ("api-builtin-device", &t!("api.builtin_device"), &t!("api.builtin_device_desc")),
        // The OCR tooltips talk about OCR, so these radios go without.
        ("api-builtin-device", &[("ocr.webgpu", Some(84.0), None), ("ocr.cpu", Some(64.0), None)], &DEVICES),
        false,
        |ui| (DEVICES.iter().position(|&d| d == ui.draft.api.builtin_device).unwrap_or(0), true),
        |ui, v| ui.draft.api.builtin_device = v,
    )?;
    let context_card = card_slider_number(
        b,
        SliderNumberParams {
            key: "api-builtin-context",
            header: t!("api.builtin_context"),
            description: Some(t!("api.builtin_context_desc")),
            min: 1024.0,
            max: 32768.0,
            step: 1024.0,
        },
        |ui| (f64::from(ui.draft.api.builtin_context_tokens), true),
        cx.edit(|ui, v: f64| ui.draft.api.builtin_context_tokens = v.max(1024.0) as u32),
    )?;
    let memory = memory_infobar(b, |ui| {
        let api = &ui.draft.api;
        find_builtin_model(&api.builtin_model).map_or((0, 0), |s| s.memory_estimate(api.builtin_device, api.builtin_context_tokens))
    })?;
    Ok((model_card, device_card, context_card.root, memory))
}

fn api_profile_section(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<StackPanel> {
    let combo = named_combo(b, cx, &API_PROFILES, &t!("api.select_profile"))?;
    let save = button(&t!("action.save"), {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                ui.profiles.name_draft = selected_api_profile(ui)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| provider_label(ui.draft.api.provider).into());
                ui.profiles.dialog = NamedDialog::SaveName;
            });
        }
    })?;
    save.tooltip(&t!("api.save_profile_tip"))?;
    let load = button(&t!("action.load"), {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                if let Err(e) = load_api_profile(ui) {
                    ui.state.write().set_error(e);
                }
            });
        }
    })?;
    load.tooltip(&t!("api.load_profile_tip"))?;
    let load_el = load.clone();
    let mut load_enabled = Prop::default();
    b.add(move |ui, _| load_enabled.set(selected_api_profile(ui).is_some(), |&e| load_el.SetIsEnabled(e)));
    let delete = named_delete_button(b, cx, &API_PROFILES, &t!("api.delete_profile_tip"))?;
    let toolbar = row(8.0)?;
    toolbar.valign(VerticalAlignment::Center)?.halign(HorizontalAlignment::Left)?;
    children!(toolbar; combo, save, load, delete);

    let section = column(4.0)?;
    children!(section; section_header(&t!("api.profiles"))?, toolbar, name_entry_row(b, cx, &API_PROFILES)?);
    named_confirm_dialog(b, cx, &API_PROFILES)?;
    Ok(section)
}

/// Shows `el` only for the providers `pred` accepts.
fn shown_for(b: &mut Binds<UiShared>, el: &impl Layout, pred: fn(ModelProvider) -> bool) -> Result<UIElement> {
    let el = el.fe()?;
    let ui_el: UIElement = el.cast()?;
    let mut shown = Prop::default();
    b.add(move |ui, _| shown.set(pred(ui.draft.api.provider), |&v| el.visible(v).map(drop)));
    Ok(ui_el)
}

pub fn api_page(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<StackPanel> {
    let mut provider = Combo::new("", {
        let cx = cx.clone();
        move |idx| {
            let Some(&provider) = idx.and_then(|i| PROVIDERS.get(i)) else {
                return;
            };
            cx.with_mut(|ui| {
                if ui.draft.api.provider != provider {
                    ui.draft.api.provider = provider;
                    mark_dirty(ui);
                }
            });
        }
    })?;
    provider.cb.width(200.0)?.valign(VerticalAlignment::Center)?;
    let hint_title = text(&t!("api.builtin_hint_title"))?;
    hint_title.SetFontWeight(SEMI_BOLD)?;
    let hint_body = text(&t!("api.builtin_hint"))?;
    hint_body.SetTextWrapping(TextWrapping::Wrap)?;
    hint_body.max_width(320.0)?;
    let hint = column(8.0)?;
    children!(hint; hint_title, hint_body);
    let flyout = Flyout::new()?;
    flyout.SetContent(&hint)?;
    // The default ThemeShadow paints as a solid block over Mica. A presenter style replaces the
    // default one, so inherit it to keep corners, padding and colours.
    let no_shadow = "<Style xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' TargetType='FlyoutPresenter' \
                     BasedOn='{StaticResource DefaultFlyoutPresenterStyle}'>\
                     <Setter Property='IsDefaultShadowEnabled' Value='False'/></Style>";
    flyout.SetPlacement(FlyoutPlacementMode::Left)?;
    flyout.SetFlyoutPresenterStyle(&XamlReader::Load(&hs(no_shadow))?.cast::<Style>()?)?;
    // The button's Flyout opens on click and light-dismisses on its own.
    let info = icon_button("\u{E946}", || {})?;
    info.tooltip(&t!("api.builtin_hint_title"))?;
    info.SetFlyout(&flyout)?;
    let provider_row = row(4.0)?;
    provider_row.valign(VerticalAlignment::Center)?;
    children!(provider_row; info, provider.cb);
    let provider_card = settings_card("api-provider", &t!("api.provider"), Some(&t!("api.provider_desc")), &provider_row)?;
    let mut info_shown = Prop::default();
    b.add(move |ui, _| {
        let items = PROVIDERS.iter().map(|&p| provider_label(p).to_string()).collect();
        provider.set(items, PROVIDERS.iter().position(|&p| p == ui.draft.api.provider), true)?;
        info_shown.set(ui.draft.api.provider == ModelProvider::Builtin, |&v| info.visible(v).map(drop))
    });

    let pick = |choice: HttpApi| {
        let set = cx.edit(move |ui, ()| ui.draft.api.http_api = choice);
        move || set(())
    };
    let mut chat = radio("api-http-api", &t!("api.chat_completions"), Some(168.0), pick(HttpApi::ChatCompletions))?;
    let mut responses = radio("api-http-api", &t!("api.responses"), Some(120.0), pick(HttpApi::Responses))?;
    let radios = row(12.0)?;
    radios.valign(VerticalAlignment::Center)?;
    children!(radios; chat.rb, responses.rb);
    let http_api_card = settings_card("api-http-api", &t!("api.http_api"), Some(&t!("api.http_api_desc")), &radios)?;
    b.add(move |ui, _| {
        chat.set(ui.draft.api.http_api == HttpApi::ChatCompletions, true)?;
        responses.set(ui.draft.api.http_api == HttpApi::Responses, true)
    });

    let model_card = card_model_suggest(
        b,
        ModelSuggestParams {
            key: "api-model",
            header: t!("api.model"),
            placeholder: Cow::Borrowed(""),
        },
        |ui| {
            let api = &ui.draft.api;
            let hint = match api.provider {
                ModelProvider::OpenCodeCli => t!("api.model_opencode"),
                ModelProvider::ClaudeCli => t!("api.model_claude"),
                ModelProvider::GrokCli | ModelProvider::CodexCli => t!("api.model_cli"),
                // Built-in hides this card (it has its own model list).
                ModelProvider::OpenaiCompatible | ModelProvider::Builtin => t!("api.model_http"),
            };
            let q = api.model.trim().to_ascii_lowercase();
            let suggestions = ui
                .model_catalog
                .iter()
                .filter(|id| q.is_empty() || id.to_ascii_lowercase().contains(&q))
                .take(40)
                .cloned()
                .collect();
            (hint, api.model.clone(), suggestions, ui.model_list_loading)
        },
        {
            let cx = cx.clone();
            move |v| {
                cx.with_mut(|ui| {
                    if ui.draft.api.model == v {
                        return;
                    }
                    ui.draft.api.model = v;
                    mark_dirty(ui);
                });
            }
        },
        {
            let cx = cx.clone();
            move || cx.with_mut(request_model_list)
        },
    )?;

    let priority_card = card_toggle(
        b,
        ("api-priority-mode", &t!("api.priority"), Some(&t!("api.priority_desc"))),
        |ui| ui.draft.api.service_tier == ServiceTier::Priority,
        cx.edit(|ui, on| ui.draft.api.service_tier = if on { ServiceTier::Priority } else { ServiceTier::Standard }),
    )?;

    let cli_path = card_text(
        b,
        ("api-cli-path", &t!("api.cli_path"), Some(&t!("api.cli_path_desc"))),
        "",
        |ui| ui.draft.api.cli_path.clone(),
        cx.edit(|ui, v| ui.draft.api.cli_path = v),
    )?;
    let base_url = card_text(
        b,
        ("api-base-url", &t!("api.base_url"), Some(&t!("api.base_url_desc"))),
        "",
        |ui| ui.draft.api.base_url.clone(),
        cx.edit(|ui, v| ui.draft.api.base_url = v),
    )?;
    let key = card_password(
        b,
        ("api-key", &t!("api.key"), Some(&t!("api.key_desc"))),
        |ui| (ui.draft.api.api_key.clone(), ui.api_key_revealed),
        cx.edit(|ui, v| ui.draft.api.api_key = v),
        {
            let cx = cx.clone();
            move || cx.with_mut(|ui| ui.api_key_revealed = !ui.api_key_revealed)
        },
    )?;
    let toggle =
        |b: &mut Binds<UiShared>, key: &str, header: &str, desc: &str, get: fn(&ApiConfig) -> bool, set: fn(&mut ApiConfig, bool)| {
            card_toggle(b, (key, header, Some(desc)), move |ui| get(&ui.draft.api), cx.edit(move |ui, on| set(&mut ui.draft.api, on)))
        };
    let structured = toggle(
        b,
        "api-structured-outputs",
        &t!("api.structured"),
        &t!("api.structured_desc"),
        |api| api.structured_outputs,
        |api, on| api.structured_outputs = on,
    )?;
    let stream = toggle(b, "api-stream", &t!("api.stream"), &t!("api.stream_desc"), |api| api.stream, |api, on| api.stream = on)?;
    let send_reasoning = toggle(
        b,
        "api-send-reasoning-content",
        &t!("api.send_reasoning"),
        &t!("api.send_reasoning_desc"),
        |api| api.send_reasoning_content,
        |api, on| api.send_reasoning_content = on,
    )?;

    let (builtin_model, builtin_device, builtin_context, builtin_memory) = builtin_section(b, cx)?;
    let output_ratio = card_slider_number(
        b,
        SliderNumberParams {
            key: "api-builtin-output-ratio",
            header: t!("api.builtin_output_ratio"),
            description: Some(t!("api.builtin_output_ratio_desc")),
            min: 1.0,
            max: 8.0,
            step: 0.5,
        },
        |ui| (f64::from(ui.draft.api.builtin_output_ratio), true),
        cx.edit(|ui, v: f64| ui.draft.api.builtin_output_ratio = v.max(1.0) as f32),
    )?;
    let dry = toggle(
        b,
        "api-builtin-dry",
        &t!("api.builtin_dry"),
        &t!("api.builtin_dry_desc"),
        |api| api.builtin_dry,
        |api, on| api.builtin_dry = on,
    )?;

    let http: fn(ModelProvider) -> bool = ModelProvider::is_http;
    let builtin: fn(ModelProvider) -> bool = |p| p == ModelProvider::Builtin;
    let not_builtin: fn(ModelProvider) -> bool = |p| p != ModelProvider::Builtin;
    let connection = column(4.0)?;
    children!(connection;
        section_header(&t!("api.connection"))?,
        provider_card.root,
        shown_for(b, &builtin_memory, builtin)?,
        shown_for(b, &http_api_card.root, http)?,
        shown_for(b, &cli_path, ModelProvider::is_cli)?,
        shown_for(b, &base_url, http)?,
        shown_for(b, &key, http)?,
        shown_for(b, &priority_card, |p| p == ModelProvider::CodexCli)?,
        shown_for(b, &model_card, not_builtin)?,
        shown_for(b, &builtin_model.root, builtin)?,
        shown_for(b, &builtin_device, builtin)?,
        shown_for(b, &builtin_context, builtin)?,
        shown_for(b, &output_ratio.root, builtin)?,
        shown_for(b, &dry, builtin)?,
        shown_for(b, &structured, http)?,
        shown_for(b, &stream, http)?,
        shown_for(b, &send_reasoning, http)?,
    );

    let reasoning = optional_text_row(
        b,
        OptionalTextParams {
            key: "api-reasoning",
            header: t!("api.reasoning"),
            description: Some(t!("api.reasoning_desc")),
            placeholder: t!("api.reasoning_placeholder"),
        },
        |ui| (ui.optional.reasoning_str.clone(), ui.optional.reasoning_enabled),
        cx.edit(|ui, v| ui.optional.reasoning_str = v),
        cx.edit(|ui, on| ui.optional.reasoning_enabled = on),
    )?;
    let sampling_applies = |ui: &UiShared| ui.draft.api.provider.is_http() || ui.draft.api.provider == ModelProvider::Builtin;
    let sampling = column(4.0)?;
    children!(sampling;
        section_header(&t!("api.optional"))?,
        optional_slider_row(
            b,
            SliderNumberParams {
                key: "api-temperature",
                header: t!("api.temperature"),
                description: Some(t!("api.temperature_desc")),
                min: 0.0,
                max: 2.0,
                step: 0.05,
            },
            move |ui| (ui.optional.temp_val, ui.optional.temp_enabled, sampling_applies(ui)),
            cx.edit(|ui, v| ui.optional.temp_val = v),
            cx.edit(|ui, on| ui.optional.temp_enabled = on),
        )?,
        optional_slider_row(
            b,
            SliderNumberParams {
                key: "api-top-p",
                header: t!("api.top_p"),
                description: Some(t!("api.top_p_desc")),
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            move |ui| (ui.optional.top_p_val, ui.optional.top_p_enabled, sampling_applies(ui)),
            cx.edit(|ui, v| ui.optional.top_p_val = v),
            cx.edit(|ui, on| ui.optional.top_p_enabled = on),
        )?,
        optional_number_row(
            b,
            SliderNumberParams {
                key: "api-top-k",
                header: t!("api.top_k"),
                description: Some(t!("api.top_k_desc")),
                min: 0.0,
                max: 1000.0,
                step: 1.0,
            },
            move |ui| (ui.optional.top_k_val, ui.optional.top_k_enabled, sampling_applies(ui)),
            cx.edit(|ui, v| ui.optional.top_k_val = v),
            cx.edit(|ui, on| ui.optional.top_k_enabled = on),
        )?,
        optional_number_row(
            b,
            SliderNumberParams {
                key: "api-max-tokens",
                header: t!("api.max_tokens"),
                description: Some(t!("api.max_tokens_desc")),
                min: 1.0,
                max: 1_000_000.0,
                step: 1.0,
            },
            move |ui| (ui.optional.max_tokens_val, ui.optional.max_tokens_enabled, sampling_applies(ui)),
            cx.edit(|ui, v| ui.optional.max_tokens_val = v),
            cx.edit(|ui, on| ui.optional.max_tokens_enabled = on),
        )?,
        shown_for(b, &reasoning, not_builtin)?,
    );

    let timeout = card_slider_number(
        b,
        SliderNumberParams {
            key: "api-timeout",
            header: t!("api.timeout"),
            description: Some(t!("api.timeout_http")),
            min: 0.0,
            max: 600.0,
            step: 1.0,
        },
        |ui| (ui.draft.api.request_timeout_secs as f64, true),
        cx.edit(|ui, v: f64| ui.draft.api.request_timeout_secs = v.max(0.0) as u64),
    )?;
    if let Some(desc) = timeout.desc.clone() {
        let mut shown = Prop::default();
        b.add(move |ui, _| {
            let api = &ui.draft.api;
            let text = if api.provider.is_cli() {
                t!("api.timeout_cli")
            } else if api.stream {
                t!("api.timeout_stream")
            } else {
                t!("api.timeout_http")
            };
            shown.set(text.into_owned(), |t| desc.SetText(&hs(t)))
        });
    }
    let reliability = column(4.0)?;
    children!(reliability;
        section_header(&t!("api.reliability"))?,
        // The built-in model has no network idle to time out.
        shown_for(b, &timeout.root, not_builtin)?,
        card_slider_number(
            b,
            SliderNumberParams {
                key: "api-retries",
                header: t!("api.retries"),
                description: Some(t!("api.retries_desc")),
                min: 0.0,
                max: 10.0,
                step: 1.0,
            },
            |ui| (f64::from(ui.draft.api.max_retries), true),
            cx.edit(|ui, v: f64| ui.draft.api.max_retries = v.max(0.0) as u32),
        )?
        .root,
        card_slider_number(
            b,
            SliderNumberParams {
                key: "api-backoff",
                header: t!("api.backoff"),
                description: Some(t!("api.backoff_desc")),
                min: 50.0,
                max: 30_000.0,
                step: 50.0,
            },
            |ui| (ui.draft.api.retry_backoff_ms as f64, true),
            cx.edit(|ui, v: f64| ui.draft.api.retry_backoff_ms = v.max(50.0) as u64),
        )?
        .root,
    );

    let body = column(8.0)?;
    children!(body; api_profile_section(b, cx)?, connection, sampling, reliability);
    Ok(body)
}
