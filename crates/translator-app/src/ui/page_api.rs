//! API settings page: connection, optional sampling, reliability.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{HttpApi, ModelProvider, ServiceTier};
use windows_reactor::{
    Element, KeyExt, LayoutExt, RadioButton, StackPanel, TeachingTip, TextStyleExt, ThemeRef, Updater, VerticalAlignment, hstack,
    text_block, vstack,
};

use crate::ui::{
    chrome::{section_header, settings_card, settings_page_shell},
    controls::{
        OptionalNumberParams, OptionalSliderParams, OptionalTextParams, SliderNumberParams, card_password, card_slider_number, card_text,
        card_toggle, optional_number_row, optional_slider_row, optional_text_row,
    },
    shared::{Snapshot, UiCx, UiShared, mark_dirty},
};

pub fn api_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);

    let provider_card = settings_card("api-provider", "Provider", Some("How to reach the translation model."), {
        let idx = snap.provider_idx;
        let cx_p = cx.clone();
        let pick = move |choice: i32| {
            let cx = cx_p.clone();
            move || {
                cx.with_mut(|ui| {
                    ui.draft.api.provider = match choice {
                        1 => ModelProvider::GrokCli,
                        2 => ModelProvider::CodexCli,
                        _ => ModelProvider::OpenaiCompatible,
                    };
                    mark_dirty(ui);
                });
            }
        };
        hstack((
            api_radio("api-provider", "OpenAI-compatible", 168.0, idx == 0, Box::new(pick(0))),
            api_radio("api-provider", "Grok CLI", 96.0, idx == 1, Box::new(pick(1))),
            api_radio("api-provider", "Codex CLI", 104.0, idx == 2, Box::new(pick(2))),
        ))
        .spacing(12.0)
        .vertical_alignment(VerticalAlignment::Center)
    });

    let http_api_card =
        settings_card("api-http-api", "API type", Some("Select standard /chat/completions or newer /responses endpoint."), {
            let idx = snap.http_api_idx;
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
            hstack((
                api_radio("api-http-api", "Chat Completions", 168.0, idx == 0, Box::new(pick(0))),
                api_radio("api-http-api", "Responses", 120.0, idx == 1, Box::new(pick(1))),
            ))
            .spacing(12.0)
            .vertical_alignment(VerticalAlignment::Center)
        });

    let model_placeholder = match snap.provider {
        ModelProvider::GrokCli => "grok-4.5",
        ModelProvider::CodexCli => "gpt-5.6",
        ModelProvider::OpenaiCompatible => "gpt-4o-mini",
    };
    let model_hint = if snap.provider.is_cli() {
        "Model id passed to the local CLI."
    } else {
        "Model name, e.g. gpt-4o-mini."
    };

    let model_card = card_text("api-model", "Model", Some(model_hint), snap.draft_model.clone(), model_placeholder, {
        let cx = cx.clone();
        move |v| {
            cx.with_mut(|ui| {
                ui.draft.api.model = v;
                mark_dirty(ui);
            });
        }
    });

    let priority_tier_card = if snap.provider.supports_priority_tier() {
        card_toggle(
            "api-priority-mode",
            "Priority mode",
            Some("Request the provider's faster processing tier. May consume more credits."),
            snap.priority_mode,
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
        .into()
    } else {
        Element::Empty
    };

    let connection = if snap.provider.is_cli() {
        vstack((
            section_header("Connection"),
            provider_card,
            card_text(
                "api-cli-path",
                "CLI path",
                Some("Leave empty to use grok / codex on PATH. Uses your existing CLI login."),
                snap.cli_path.clone(),
                snap.provider.default_bin(),
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
            text_block("A local agent session stays open and only the new turn is sent. Tools are denied.")
                .font_size(12.0)
                .foreground(ThemeRef::SecondaryText)
                .wrap(),
            model_card,
        ))
        .spacing(4.0)
    } else {
        vstack((
            section_header("Connection"),
            provider_card,
            http_api_card,
            card_text(
                "api-base-url",
                "Base URL",
                Some("OpenAI-compatible API endpoint."),
                snap.base_url.clone(),
                "https://api.openai.com/v1",
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
                snap.api_key.clone(),
                snap.api_key_revealed,
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
                snap.structured_outputs,
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
        ))
        .spacing(4.0)
    };

    // Note: windows-reactor TeachingTip emits CloseButtonText/ActionButtonText,
    // but the WinUI backend only handles CloseButton/ActionButton — using
    // close_button()/action_button() logs "unhandled prop". Light-dismiss only.
    let tip = TeachingTip::new("Optional parameters")
        .subtitle("Turn Off to leave a field out of the API request. Controls stay disabled while Off.")
        .is_open(!snap.optional_tip_seen)
        .light_dismiss()
        .on_closed({
            let cx = cx.clone();
            move || {
                cx.with_mut(|ui| {
                    ui.optional_tip_seen = true;
                });
            }
        })
        .with_key("api-optional-tip");

    let sampling = vstack((
        section_header("Optional parameters"),
        text_block(if snap.provider.is_cli() {
            "Turn On to include. Temperature, Top P, and Max tokens apply to HTTP only. Reasoning effort is sent to the CLI."
        } else {
            "Turn On to include in the request. Off = omit."
        })
        .font_size(12.0)
        .foreground(ThemeRef::SecondaryText)
        .wrap(),
        tip,
        optional_slider_row(
            OptionalSliderParams {
                key: "api-temperature",
                header: "Temperature".into(),
                description: "Higher = more random (0–2).".into(),
                value: snap.temp_val,
                enabled: snap.temp_enabled,
                min: 0.0,
                max: 2.0,
                step: 0.05,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.temp_val = v;
                        mark_dirty(ui);
                    });
                }
            },
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.temp_enabled = on;
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
                value: snap.top_p_val,
                enabled: snap.top_p_enabled,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.top_p_val = v;
                        mark_dirty(ui);
                    });
                }
            },
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.top_p_enabled = on;
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
                value: snap.max_tokens_val,
                enabled: snap.max_tokens_enabled,
                min: 1.0,
                max: 1_000_000.0,
                step: 1.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.max_tokens_val = v;
                        mark_dirty(ui);
                    });
                }
            },
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.max_tokens_enabled = on;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        optional_text_row(
            OptionalTextParams {
                key: "api-reasoning",
                header: "Reasoning effort".into(),
                description: Some("For models that support it: low, medium, or high.".into()),
                text: snap.reasoning_str.clone(),
                enabled: snap.reasoning_enabled,
                placeholder: "low | medium | high".into(),
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.reasoning_str = v;
                        mark_dirty(ui);
                    });
                }
            },
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        ui.reasoning_enabled = on;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let reliability = vstack((
        section_header("Reliability"),
        card_slider_number(
            SliderNumberParams {
                key: "api-timeout",
                header: "Timeout (seconds)".into(),
                description: Some(if snap.provider.is_cli() {
                    "Per-turn CLI prompt timeout. 0 = wait up to 1 hour.".into()
                } else {
                    "0 = wait forever.".into()
                }),
                value: snap.timeout_secs,
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
                value: snap.max_retries,
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
                value: snap.retry_backoff,
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
    ))
    .spacing(4.0);

    settings_page_shell(shared, snap, bump, vstack((connection, sampling, reliability)).spacing(8.0))
}

fn api_radio(group: &'static str, label: &str, width: f64, checked: bool, on: Box<dyn Fn() + 'static>) -> RadioButton {
    let mut rb = RadioButton::new(label).group(group).checked(checked).on_checked(on);
    rb.modifiers.min_width = Some(width);
    rb.modifiers.width = Some(width);
    rb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);
    rb
}
