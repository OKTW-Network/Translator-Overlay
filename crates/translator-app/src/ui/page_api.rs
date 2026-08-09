//! API settings page: connection, optional sampling, reliability.

use std::sync::{Arc, Mutex};

use windows_reactor::*;

use super::chrome::{section_header, settings_page_shell};
use super::controls::{
    OptionalNumberParams, OptionalSliderParams, OptionalTextParams, SliderNumberParams,
    card_password, card_slider_number, card_text, optional_number_row, optional_slider_row,
    optional_text_row,
};
use super::shared::{Snapshot, UiShared, mark_dirty};

pub fn api_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> Element {
    let s_url = Arc::clone(shared);
    let s_key = Arc::clone(shared);
    let s_model = Arc::clone(shared);
    let s_temp = Arc::clone(shared);
    let s_top = Arc::clone(shared);
    let s_max = Arc::clone(shared);
    let s_reason = Arc::clone(shared);
    let s_timeout = Arc::clone(shared);
    let s_retries = Arc::clone(shared);
    let s_backoff = Arc::clone(shared);
    let bump_u = bump.clone();
    let bump_k = bump.clone();
    let bump_m = bump.clone();
    let bump_t = bump.clone();
    let bump_p = bump.clone();
    let bump_x = bump.clone();
    let bump_r = bump.clone();
    let bump_to = bump.clone();
    let bump_re = bump.clone();
    let bump_bo = bump.clone();
    let bump_te = bump.clone();
    let bump_pe = bump.clone();
    let bump_xe = bump.clone();
    let bump_re_en = bump.clone();

    let connection = vstack((
        section_header("Connection"),
        card_text(
            "api-base-url",
            "Base URL",
            Some("OpenAI-compatible API endpoint."),
            snap.base_url.clone(),
            "https://api.openai.com/v1",
            move |v| {
                if let Ok(mut ui) = s_url.lock() {
                    ui.draft.api.base_url = v;
                    mark_dirty(&mut ui);
                }
                bump_u.call(|n| n.wrapping_add(1));
            },
        ),
        card_password(
            "api-key",
            "API key",
            Some("Stored only on this PC."),
            snap.api_key.clone(),
            snap.api_key_revealed,
            {
                let s = Arc::clone(&s_key);
                move |v| {
                    if let Ok(mut ui) = s.lock() {
                        ui.draft.api.api_key = v;
                        mark_dirty(&mut ui);
                    }
                    bump_k.call(|n| n.wrapping_add(1));
                }
            },
            {
                let s = s_key;
                let bump = bump.clone();
                move || {
                    if let Ok(mut ui) = s.lock() {
                        ui.api_key_revealed = !ui.api_key_revealed;
                    }
                    bump.call(|n| n.wrapping_add(1));
                }
            },
        ),
        card_text(
            "api-model",
            "Model",
            Some("Model name, e.g. gpt-4o-mini."),
            snap.draft_model.clone(),
            "gpt-4o-mini",
            move |v| {
                if let Ok(mut ui) = s_model.lock() {
                    ui.draft.api.model = v;
                    mark_dirty(&mut ui);
                }
                bump_m.call(|n| n.wrapping_add(1));
            },
        ),
    ))
    .spacing(4.0);

    // Note: windows-reactor TeachingTip emits CloseButtonText/ActionButtonText,
    // but the WinUI backend only handles CloseButton/ActionButton — using
    // close_button()/action_button() logs "unhandled prop". Light-dismiss only.
    let tip = {
        let s = Arc::clone(shared);
        let bump_tip = bump.clone();
        TeachingTip::new("Optional parameters")
            .subtitle(
                "Turn Off to leave a field out of the API request. Controls stay disabled while Off.",
            )
            .is_open(!snap.optional_tip_seen)
            .light_dismiss()
            .on_closed(move || {
                if let Ok(mut ui) = s.lock() {
                    ui.optional_tip_seen = true;
                }
                bump_tip.call(|n| n.wrapping_add(1));
            })
            .with_key("api-optional-tip")
    };

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
                let s = Arc::clone(&s_temp);
                move |v| {
                    if let Ok(mut ui) = s.lock() {
                        ui.temp_val = v;
                        mark_dirty(&mut ui);
                    }
                    bump_t.call(|n| n.wrapping_add(1));
                }
            },
            {
                let s = Arc::clone(&s_temp);
                move |on| {
                    if let Ok(mut ui) = s.lock() {
                        ui.temp_enabled = on;
                        mark_dirty(&mut ui);
                    }
                    bump_te.call(|n| n.wrapping_add(1));
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
                let s = Arc::clone(&s_top);
                move |v| {
                    if let Ok(mut ui) = s.lock() {
                        ui.top_p_val = v;
                        mark_dirty(&mut ui);
                    }
                    bump_p.call(|n| n.wrapping_add(1));
                }
            },
            {
                let s = Arc::clone(&s_top);
                move |on| {
                    if let Ok(mut ui) = s.lock() {
                        ui.top_p_enabled = on;
                        mark_dirty(&mut ui);
                    }
                    bump_pe.call(|n| n.wrapping_add(1));
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
                let s = Arc::clone(&s_max);
                move |v| {
                    if let Ok(mut ui) = s.lock() {
                        ui.max_tokens_val = v;
                        mark_dirty(&mut ui);
                    }
                    bump_x.call(|n| n.wrapping_add(1));
                }
            },
            {
                let s = Arc::clone(&s_max);
                move |on| {
                    if let Ok(mut ui) = s.lock() {
                        ui.max_tokens_enabled = on;
                        mark_dirty(&mut ui);
                    }
                    bump_xe.call(|n| n.wrapping_add(1));
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
                let s = Arc::clone(&s_reason);
                move |v| {
                    if let Ok(mut ui) = s.lock() {
                        ui.reasoning_str = v;
                        mark_dirty(&mut ui);
                    }
                    bump_r.call(|n| n.wrapping_add(1));
                }
            },
            {
                let s = s_reason;
                move |on| {
                    if let Ok(mut ui) = s.lock() {
                        ui.reasoning_enabled = on;
                        mark_dirty(&mut ui);
                    }
                    bump_re_en.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_timeout.lock() {
                    ui.draft.api.request_timeout_secs = v.max(0.0) as u64;
                    mark_dirty(&mut ui);
                }
                bump_to.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_retries.lock() {
                    ui.draft.api.max_retries = v.max(0.0) as u32;
                    mark_dirty(&mut ui);
                }
                bump_re.call(|n| n.wrapping_add(1));
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
            move |v| {
                if let Ok(mut ui) = s_backoff.lock() {
                    ui.draft.api.retry_backoff_ms = v.max(50.0) as u64;
                    mark_dirty(&mut ui);
                }
                bump_bo.call(|n| n.wrapping_add(1));
            },
        ),
    ))
    .spacing(4.0);

    settings_page_shell(
        shared,
        snap,
        bump,
        vstack((connection, sampling, reliability))
            .spacing(8.0)
            .into(),
    )
}
