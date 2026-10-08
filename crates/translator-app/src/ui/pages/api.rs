//! API settings for the provider, connection, sampling, and named profiles.

use std::{borrow::Cow, sync::Arc};

use parking_lot::Mutex;
use rust_i18n::t;
use translator_core::{HttpApi, ModelProvider, ServiceTier};
use windows_reactor::{
    Button, ChildrenControl, ComboBox, ContentControl, HorizontalAlignment, LayoutControl, LocalSender, Orientation, StackPanel,
    TooltipExt, VerticalAlignment, View,
};

use crate::ui::{
    cards::{section_header, settings_card},
    chrome::settings_page_shell,
    controls::{
        ModelSuggestParams, OptionalNumberParams, OptionalSliderParams, OptionalTextParams, SliderNumberParams, card_model_suggest,
        card_password, card_slider_number, card_text, card_toggle, optional_number_row, optional_slider_row, optional_text_row, radio,
    },
    named::{NamedSnap, name_entry_row, named_combo, named_confirm_dialog, named_delete_button},
    shared::{
        API_PROFILES, AppMsg, ChromeSnap, NamedDialog, UiCx, UiShared, load_api_profile, mark_dirty, request_model_list,
        schedule_model_list_if_needed, selected_api_profile,
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

fn api_profile_section(cx: &UiCx, snap: &NamedSnap) -> View {
    let combo = named_combo(cx, &API_PROFILES, snap, t!("api.select_profile"));
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
                            ui.profiles.name_draft = selected_api_profile(ui)
                                .map(|p| p.name.clone())
                                .unwrap_or_else(|| provider_label(ui.draft.api.provider).into());
                            ui.profiles.dialog = NamedDialog::SaveName;
                        });
                    }
                })
                .content(t!("action.save").as_ref())
                .tooltip(t!("api.save_profile_tip")),
            Button::new()
                .is_enabled(snap.selected().is_some())
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
                .content(t!("action.load").as_ref())
                .tooltip(t!("api.load_profile_tip")),
            named_delete_button(cx, &API_PROFILES, snap, t!("api.delete_profile_tip")),
        ));

    StackPanel::new().spacing(4.0).children((
        section_header(t!("api.profiles")),
        toolbar,
        name_entry_row(cx, &API_PROFILES, snap),
        named_confirm_dialog(cx, &API_PROFILES, snap),
    ))
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
            NamedSnap::take(&mut ui, &API_PROFILES),
            ui.model_catalog.clone(),
            ui.model_list_loading,
        )
    };

    let provider_desc = t!("api.provider_desc");
    let provider_card = settings_card("api-provider", t!("api.provider"), Some(&provider_desc), {
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

    let http_api_desc = t!("api.http_api_desc");
    let http_api_card = settings_card("api-http-api", t!("api.http_api"), Some(&http_api_desc), {
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
                radio("api-http-api", &t!("api.chat_completions"), Some(168.0), idx == 0, pick(0)),
                radio("api-http-api", &t!("api.responses"), Some(120.0), idx == 1, pick(1)),
            ))
    });

    let model_hint = match api.provider {
        ModelProvider::OpenCodeCli => t!("api.model_opencode"),
        ModelProvider::ClaudeCli => t!("api.model_claude"),
        ModelProvider::GrokCli | ModelProvider::CodexCli => t!("api.model_cli"),
        ModelProvider::OpenaiCompatible => t!("api.model_http"),
    };

    let model_card = card_model_suggest(
        ModelSuggestParams {
            key: "api-model",
            header: t!("api.model"),
            description: model_hint,
            value: api.model.clone(),
            placeholder: Cow::Borrowed(""),
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
        card_toggle("api-priority-mode", t!("api.priority"), Some(&t!("api.priority_desc")), api.service_tier == ServiceTier::Priority, {
            let cx = cx.clone();
            move |on| {
                cx.with_mut(|ui| {
                    ui.draft.api.service_tier = if on { ServiceTier::Priority } else { ServiceTier::Standard };
                    mark_dirty(ui);
                });
            }
        })
    } else {
        View::empty()
    };

    let connection = if api.provider.is_cli() {
        StackPanel::new().spacing(4.0).children((
            section_header(t!("api.connection")),
            provider_card,
            card_text("api-cli-path", t!("api.cli_path"), Some(&t!("api.cli_path_desc")), api.cli_path.clone(), "", {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.api.cli_path = v;
                        mark_dirty(ui);
                    });
                }
            }),
            priority_tier_card,
            model_card,
        ))
    } else {
        StackPanel::new().spacing(4.0).children((
            section_header(t!("api.connection")),
            provider_card,
            http_api_card,
            card_text("api-base-url", t!("api.base_url"), Some(&t!("api.base_url_desc")), api.base_url.clone(), "", {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.api.base_url = v;
                        mark_dirty(ui);
                    });
                }
            }),
            card_password(
                "api-key",
                t!("api.key"),
                Some(&t!("api.key_desc")),
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
            card_toggle("api-structured-outputs", t!("api.structured"), Some(&t!("api.structured_desc")), api.structured_outputs, {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.draft.api.structured_outputs = on;
                        mark_dirty(ui);
                    });
                }
            }),
            card_toggle("api-stream", t!("api.stream"), Some(&t!("api.stream_desc")), api.stream, {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.draft.api.stream = on;
                        mark_dirty(ui);
                    });
                }
            }),
            card_toggle(
                "api-send-reasoning-content",
                t!("api.send_reasoning"),
                Some(&t!("api.send_reasoning_desc")),
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
        section_header(t!("api.optional")),
        optional_slider_row(
            OptionalSliderParams {
                key: "api-temperature",
                header: t!("api.temperature"),
                description: t!("api.temperature_desc"),
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
                header: t!("api.top_p"),
                description: t!("api.top_p_desc"),
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
                header: t!("api.max_tokens"),
                description: Some(t!("api.max_tokens_desc")),
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
                header: t!("api.reasoning"),
                description: Some(t!("api.reasoning_desc")),
                text: optional.reasoning_str.clone(),
                enabled: optional.reasoning_enabled,
                placeholder: t!("api.reasoning_placeholder"),
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
        section_header(t!("api.reliability")),
        card_slider_number(
            SliderNumberParams {
                key: "api-timeout",
                header: t!("api.timeout"),
                description: Some(if api.provider.is_cli() {
                    t!("api.timeout_cli")
                } else if api.stream {
                    t!("api.timeout_stream")
                } else {
                    t!("api.timeout_http")
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
                header: t!("api.retries"),
                description: Some(t!("api.retries_desc")),
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
                header: t!("api.backoff"),
                description: Some(t!("api.backoff_desc")),
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
