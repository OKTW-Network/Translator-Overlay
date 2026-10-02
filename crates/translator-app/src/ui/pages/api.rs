//! API settings: provider, connection, sampling, and named profiles.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{HttpApi, ModelProvider, ServiceTier};
use windows_reactor::{
    Border, Button, ButtonStyle, ChildrenControl, ComboBox, ContentControl, ContentDialog, ContentDialogResult, FontWeight,
    HorizontalAlignment, LayoutControl, LocalSender, Orientation, StackPanel, TextBlock, TextBox, ThemeBrush, Thickness, TooltipExt,
    VerticalAlignment, View,
};

use crate::ui::{
    cards::{section_header, settings_card},
    chrome::settings_page_shell,
    controls::{
        ModelSuggestParams, OptionalNumberParams, OptionalSliderParams, OptionalTextParams, SliderNumberParams, card_model_suggest,
        card_password, card_slider_number, card_text, card_toggle, optional_number_row, optional_slider_row, optional_text_row, radio,
    },
    shared::{
        AppMsg, ChromeSnap, PresetDialog, UiCx, UiShared, commit_api_profile, load_api_profile, mark_dirty, request_model_list,
        save_api_profiles, schedule_model_list_if_needed, selected_api_profile,
    },
};

fn provider_label(provider: ModelProvider) -> &'static str {
    match provider {
        ModelProvider::OpenaiCompatible => "OpenAI-compatible",
        ModelProvider::GrokCli => "Grok CLI",
        ModelProvider::OpenCodeCli => "OpenCode CLI",
        ModelProvider::CodexCli => "Codex CLI",
        ModelProvider::ClaudeCli => "Claude Code",
    }
}

struct ApiProfileSnap {
    names: Vec<String>,
    selected_idx: i32,
    name_draft: String,
    dialog: PresetDialog,
}

fn api_profile_section(cx: &UiCx, snap: &ApiProfileSnap) -> View {
    let has_profiles = !snap.names.is_empty();
    let profile_selected = has_profiles && snap.selected_idx >= 0;
    let profile_idx = if profile_selected { Some(snap.selected_idx as usize) } else { None };

    let combo = ComboBox::new()
        .items_source(snap.names.clone())
        .selected_index(profile_idx)
        .placeholder_text("Select profile…")
        .is_enabled(has_profiles)
        .on_selection_changed({
            let cx = cx.clone();
            move |idx: Option<usize>| {
                cx.with_mut(|ui| {
                    ui.api_profile_selected_idx = idx.filter(|&i| i < ui.api_profiles.len()).map(|i| i as i32).unwrap_or(-1);
                });
            }
        })
        .width(180.0)
        .min_width(140.0)
        .vertical_alignment(VerticalAlignment::Center);

    let toolbar = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Left)
        .children((
            combo,
            Button::new()
                .on_click({
                    let cx = cx.clone();
                    move || {
                        cx.with_mut(|ui| {
                            ui.api_profile_name_draft = selected_api_profile(ui)
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| provider_label(ui.draft.api.provider).into());
                            ui.api_profile_dialog = PresetDialog::SaveName;
                        });
                    }
                })
                .content("Save")
                .tooltip("Save current API settings as a named profile"),
            Button::new()
                .is_enabled(profile_selected)
                .on_click({
                    let cx = cx.clone();
                    move || {
                        cx.with_mut(|ui| {
                            if let Err(e) = load_api_profile(ui) {
                                ui.state.write().set_error(e);
                            }
                        });
                    }
                })
                .content("Load")
                .tooltip("Copy the selected profile into the form"),
            Button::new()
                .is_enabled(profile_selected)
                .on_click({
                    let cx = cx.clone();
                    move || {
                        cx.with_mut(|ui| {
                            let Some(name) = selected_api_profile(ui).map(|p| p.name.clone()) else {
                                return;
                            };
                            ui.api_profile_dialog = PresetDialog::Delete { name };
                        });
                    }
                })
                .content("Delete")
                .tooltip("Delete the selected profile"),
        ));

    StackPanel::new().spacing(4.0).children((
        section_header("Profiles"),
        toolbar,
        api_profile_name_row(cx, snap),
        api_profile_confirm_dialog(cx, snap),
    ))
}

fn api_profile_name_row(cx: &UiCx, snap: &ApiProfileSnap) -> View {
    if !matches!(snap.dialog, PresetDialog::SaveName) {
        return View::empty();
    }

    let name_tb = TextBox::new()
        .text(snap.name_draft.clone())
        .on_text_changed({
            let cx = cx.clone();
            move |text: String| {
                cx.with_mut(|ui| ui.api_profile_name_draft = text);
            }
        })
        .width(180.0)
        .vertical_alignment(VerticalAlignment::Center);

    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(4.0)
        .padding(Thickness::new(8.0, 4.0, 8.0, 4.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .content(
            StackPanel::new().orientation(Orientation::Horizontal).spacing(8.0).children((
                TextBlock::new()
                    .text("Name")
                    .font_weight(FontWeight::SEMI_BOLD)
                    .vertical_alignment(VerticalAlignment::Center),
                name_tb,
                Button::new()
                    .style(ButtonStyle::Accent)
                    .on_click({
                        let cx = cx.clone();
                        move || {
                            cx.with_mut(|ui| {
                                let name = ui.api_profile_name_draft.trim().to_string();
                                if name.is_empty() {
                                    ui.state.write().set_error("Profile name is required.");
                                    return;
                                }
                                if ui.api_profiles.iter().any(|p| p.name == name) {
                                    ui.api_profile_dialog = PresetDialog::Overwrite { name };
                                    return;
                                }
                                if let Err(e) = commit_api_profile(ui, name) {
                                    ui.state.write().set_error(e);
                                }
                            });
                        }
                    })
                    .content("Save"),
                Button::new()
                    .on_click({
                        let cx = cx.clone();
                        move || {
                            cx.with_mut(|ui| {
                                ui.api_profile_dialog = PresetDialog::None;
                                ui.api_profile_name_draft.clear();
                            });
                        }
                    })
                    .content("Cancel"),
            )),
        )
}

fn api_profile_confirm_dialog(cx: &UiCx, snap: &ApiProfileSnap) -> View {
    let (open, title, body, primary) = match &snap.dialog {
        PresetDialog::Overwrite { name } => {
            (true, "Overwrite profile?", format!("Replace the API settings saved in \"{name}\"?"), "Overwrite")
        }
        PresetDialog::Delete { name } => (true, "Delete profile?", format!("Delete profile \"{name}\"? This cannot be undone."), "Delete"),
        PresetDialog::None | PresetDialog::SaveName => (false, "", String::new(), "OK"),
    };

    ContentDialog::new()
        .title(title)
        .primary_button_text(primary)
        .close_button_text("Cancel")
        .is_open(open)
        .on_closed({
            let cx = cx.clone();
            move |result: ContentDialogResult| {
                cx.with_mut(|ui| {
                    let action = ui.api_profile_dialog.clone();
                    ui.api_profile_dialog = PresetDialog::None;
                    if result != ContentDialogResult::Primary {
                        if matches!(action, PresetDialog::Overwrite { .. }) {
                            ui.api_profile_dialog = PresetDialog::SaveName;
                        }
                        return;
                    }
                    match action {
                        PresetDialog::Overwrite { name } => {
                            if let Err(e) = commit_api_profile(ui, name) {
                                ui.state.write().set_error(e);
                            }
                        }
                        PresetDialog::Delete { name } => {
                            ui.api_profiles.retain(|p| p.name != name);
                            ui.api_profile_selected_idx = -1;
                            ui.api_profile_name_draft.clear();
                            if let Err(e) = save_api_profiles(ui) {
                                ui.state.write().set_error(e);
                            }
                        }
                        PresetDialog::None | PresetDialog::SaveName => {}
                    }
                });
            }
        })
        .content(body)
}

pub fn api_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let (api, optional, api_key_revealed, profile_snap, model_catalog, model_list_loading) = {
        let mut ui = shared.lock();
        schedule_model_list_if_needed(&mut ui, bump);
        (
            ui.draft.api.clone(),
            ui.optional.clone(),
            ui.api_key_revealed,
            ApiProfileSnap {
                names: ui.api_profiles.iter().map(|p| p.name.clone()).collect(),
                selected_idx: ui.api_profile_selected_idx,
                name_draft: ui.api_profile_name_draft.clone(),
                dialog: ui.api_profile_dialog.clone(),
            },
            ui.model_catalog.clone(),
            ui.model_list_loading,
        )
    };

    let provider_card = settings_card("api-provider", "Provider", Some("HTTP endpoint or a local CLI."), {
        const PROVIDERS: [ModelProvider; 5] = [
            ModelProvider::OpenaiCompatible,
            ModelProvider::GrokCli,
            ModelProvider::OpenCodeCli,
            ModelProvider::CodexCli,
            ModelProvider::ClaudeCli,
        ];
        ComboBox::new()
            .items_source(PROVIDERS.iter().map(|&p| provider_label(p).to_string()).collect::<Vec<_>>())
            .selected_index(PROVIDERS.iter().position(|&p| p == api.provider))
            .on_selection_changed({
                let cx = cx.clone();
                move |idx: Option<usize>| {
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
            })
            .width(200.0)
            .vertical_alignment(VerticalAlignment::Center)
    });

    let http_api_card =
        settings_card("api-http-api", "API type", Some("Select standard /chat/completions or newer /responses endpoint."), {
            let idx = match api.http_api {
                HttpApi::ChatCompletions => 0,
                HttpApi::Responses => 1,
            };
            let cx_h = cx.clone();
            let pick = move |choice: i32| {
                let cx = cx_h.clone();
                move || {
                    cx.with_mut(|ui| {
                        ui.draft.api.http_api = if choice == 1 {
                            HttpApi::Responses
                        } else {
                            HttpApi::ChatCompletions
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
                    radio("api-http-api", "Chat Completions", Some(168.0), idx == 0, pick(0)),
                    radio("api-http-api", "Responses", Some(120.0), idx == 1, pick(1)),
                ))
        });

    let model_hint = match api.provider {
        ModelProvider::OpenCodeCli => "Model id as provider/model, e.g. opencode/gpt-5.",
        ModelProvider::ClaudeCli => "Alias (sonnet, haiku, opus) or full model id. Empty = Claude Code default.",
        ModelProvider::GrokCli | ModelProvider::CodexCli => "Model id passed to the local CLI.",
        ModelProvider::OpenaiCompatible => "Model name, e.g. gpt-4o-mini.",
    };

    let model_card = card_model_suggest(
        ModelSuggestParams {
            key: "api-model",
            header: "Model".into(),
            description: model_hint.into(),
            value: api.model.clone(),
            placeholder: String::new(),
            suggestions: {
                let q = api.model.trim().to_ascii_lowercase();
                model_catalog
                    .iter()
                    .filter(|id| q.is_empty() || id.to_ascii_lowercase().contains(&q))
                    .take(40)
                    .cloned()
                    .collect()
            },
            loading: model_list_loading,
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
            move || {
                let bump = cx.bump.clone();
                cx.with_mut(|ui| request_model_list(ui, &bump));
            }
        },
    );

    let priority_tier_card: View = if api.provider == ModelProvider::CodexCli {
        card_toggle(
            "api-priority-mode",
            "Priority mode",
            Some("Request the provider's faster processing tier. May consume more credits."),
            api.service_tier == ServiceTier::Priority,
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.draft.api.service_tier = if on { ServiceTier::Priority } else { ServiceTier::Standard };
                        mark_dirty(ui);
                    });
                }
            },
        )
    } else {
        View::empty()
    };

    let connection = if api.provider.is_cli() {
        StackPanel::new().spacing(4.0).children((
            section_header("Connection"),
            provider_card,
            card_text(
                "api-cli-path",
                "CLI path",
                Some("Leave empty to use grok / opencode / codex / claude on PATH. Uses your existing CLI login."),
                api.cli_path.clone(),
                "",
                {
                    let cx = cx.clone();
                    move |v| {
                        cx.with_mut(|ui| {
                            ui.draft.api.cli_path = v;
                            mark_dirty(ui);
                        });
                    }
                },
            ),
            priority_tier_card,
            model_card,
        ))
    } else {
        StackPanel::new()
            .spacing(4.0)
            .children((
                section_header("Connection"),
                provider_card,
                http_api_card,
                card_text(
                    "api-base-url",
                    "Base URL",
                    Some("OpenAI-compatible API endpoint."),
                    api.base_url.clone(),
                    "",
                    {
                        let cx = cx.clone();
                        move |v| {
                            cx.with_mut(|ui| {
                                ui.draft.api.base_url = v;
                                mark_dirty(ui);
                            });
                        }
                    },
                ),
                card_password(
                    "api-key",
                    "API key",
                    Some("Stored only on this PC."),
                    api.api_key.clone(),
                    api_key_revealed,
                    {
                        let cx = cx.clone();
                        move |v| {
                            cx.with_mut(|ui| {
                                ui.draft.api.api_key = v;
                                mark_dirty(ui);
                            });
                        }
                    },
                    {
                        let cx = cx.clone();
                        move || {
                            cx.with_mut(|ui| {
                                ui.api_key_revealed = !ui.api_key_revealed;
                            });
                        }
                    },
                ),
                priority_tier_card,
                model_card,
                card_toggle(
                    "api-structured-outputs",
                    "Structured outputs",
                    Some("Ask the model to return JSON matching the translation schema. Turn off if the endpoint rejects json_schema."),
                    api.structured_outputs,
                    {
                        let cx = cx.clone();
                        move |on| {
                            cx.with_mut(|ui| {
                                ui.draft.api.structured_outputs = on;
                                mark_dirty(ui);
                            });
                        }
                    },
                ),
                card_toggle(
                    "api-stream",
                    "Stream",
                    Some("Receive the response as it is generated. Turn off if the endpoint rejects stream."),
                    api.stream,
                    {
                        let cx = cx.clone();
                        move |on| {
                            cx.with_mut(|ui| {
                                ui.draft.api.stream = on;
                                mark_dirty(ui);
                            });
                        }
                    },
                ),
                card_toggle(
                    "api-send-reasoning-content",
                    "Send reasoning",
                    Some("Replay the model's reasoning with assistant messages on follow-up turns. Turn off if the endpoint rejects reasoning_content."),
                    api.send_reasoning_content,
                    {
                        let cx = cx.clone();
                        move |on| {
                            cx.with_mut(|ui| {
                                ui.draft.api.send_reasoning_content = on;
                                mark_dirty(ui);
                            });
                        }
                    },
                ),
            ))
    };

    let http_sampling = !api.provider.is_cli();
    let sampling = StackPanel::new().spacing(4.0).children((
        section_header("Optional parameters"),
        optional_slider_row(
            OptionalSliderParams {
                key: "api-temperature",
                header: "Temperature".into(),
                description: "Higher = more random (0–2).".into(),
                value: optional.temp_val,
                enabled: optional.temp_enabled,
                applicable: http_sampling,
                min: 0.0,
                max: 2.0,
                step: 0.05,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.optional.temp_val = v;
                        mark_dirty(ui);
                    });
                }
            },
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.optional.temp_enabled = on;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        optional_slider_row(
            OptionalSliderParams {
                key: "api-top-p",
                header: "Top P".into(),
                description: "Nucleus sampling limit (0–1).".into(),
                value: optional.top_p_val,
                enabled: optional.top_p_enabled,
                applicable: http_sampling,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.optional.top_p_val = v;
                        mark_dirty(ui);
                    });
                }
            },
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.optional.top_p_enabled = on;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        optional_number_row(
            OptionalNumberParams {
                key: "api-max-tokens",
                header: "Max tokens".into(),
                description: Some("Max reply length. Limit depends on the model.".into()),
                value: optional.max_tokens_val,
                enabled: optional.max_tokens_enabled,
                applicable: http_sampling,
                min: 1.0,
                max: 1_000_000.0,
                step: 1.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.optional.max_tokens_val = v;
                        mark_dirty(ui);
                    });
                }
            },
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.optional.max_tokens_enabled = on;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        optional_text_row(
            OptionalTextParams {
                key: "api-reasoning",
                header: "Reasoning effort".into(),
                description: Some("For models that support it: none, minimal, low, medium, high, xhigh, or max.".into()),
                text: optional.reasoning_str.clone(),
                enabled: optional.reasoning_enabled,
                placeholder: "none | minimal | low | medium | high | xhigh | max".into(),
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.optional.reasoning_str = v;
                        mark_dirty(ui);
                    });
                }
            },
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.optional.reasoning_enabled = on;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ));

    let reliability = StackPanel::new().spacing(4.0).children((
        section_header("Reliability"),
        card_slider_number(
            SliderNumberParams {
                key: "api-timeout",
                header: "Timeout (seconds)".into(),
                description: Some(if api.provider.is_cli() {
                    "Idle timeout between CLI events. 0 = wait up to 1 hour.".into()
                } else if api.stream {
                    "Idle timeout between stream chunks. 0 = wait forever.".into()
                } else {
                    "Max wait for the HTTP response. 0 = wait forever.".into()
                }),
                value: api.request_timeout_secs as f64,
                min: 0.0,
                max: 600.0,
                step: 1.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.api.request_timeout_secs = v.max(0.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "api-retries",
                header: "Retries".into(),
                description: Some("Extra attempts after a failed request.".into()),
                value: f64::from(api.max_retries),
                min: 0.0,
                max: 10.0,
                step: 1.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.api.max_retries = v.max(0.0) as u32;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "api-backoff",
                header: "Retry delay (ms)".into(),
                description: Some("Wait before retry; doubles each attempt.".into()),
                value: api.retry_backoff_ms as f64,
                min: 50.0,
                max: 30_000.0,
                step: 50.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.api.retry_backoff_ms = v.max(50.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ));

    settings_page_shell(
        shared,
        chrome,
        bump,
        StackPanel::new()
            .spacing(8.0)
            .children((api_profile_section(&cx, &profile_snap), connection, sampling, reliability)),
    )
}
