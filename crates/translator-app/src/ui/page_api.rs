//! API settings page: connection, optional sampling, reliability.

use std::sync::Arc;

use parking_lot::Mutex;
use windows_reactor::*;

use crate::ui::{
    chrome::{section_header, settings_page_shell},
    controls::{
        OptionalNumberParams, OptionalSliderParams, OptionalTextParams, SliderNumberParams, card_password, card_slider_number, card_text,
        optional_number_row, optional_slider_row, optional_text_row,
    },
    shared::{Snapshot, UiCx, UiShared, mark_dirty},
};

pub fn api_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> Element {
    let cx = UiCx::new(shared, bump);

    let connection = vstack((
        section_header("Connection"),
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
        card_text("api-model", "Model", Some("Model name, e.g. gpt-4o-mini."), snap.draft_model.clone(), "gpt-4o-mini", {
            let cx = cx.clone();
            move |v| {
                cx.with_mut(|ui| {
                    ui.draft.api.model = v;
                    mark_dirty(ui);
                });
            }
        }),
    ))
    .spacing(4.0);

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
        text_block("Turn On to include in the request. Off = omit.")
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
                description: Some("0 = wait forever.".into()),
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

    settings_page_shell(shared, snap, bump, vstack((connection, sampling, reliability)).spacing(8.0).into())
}
