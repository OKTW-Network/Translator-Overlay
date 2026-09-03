//! Settings forms: API, translation, OCR, and overlay appearance.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{
    HttpApi, LineMergeConfig, LineMergeOrder, ModelProvider, ModelTier, READER_FONT_PX_MAX, READER_FONT_PX_MIN, ServiceTier,
    TRANSLATION_CACHE_MAX_CAP, TRANSLATION_CACHE_MAX_MIN,
};
use windows_reactor::{
    Element, HorizontalAlignment, LayoutExt, RadioButton, StackPanel, TextStyleExt, ThemeRef, TooltipExt, Updater, VerticalAlignment,
    button, hstack, text_block, text_box, vstack,
};

use crate::{
    pipeline::PipelineCommand,
    ui::{
        chrome::{section_header, settings_card, settings_card_stack, settings_page_shell, subsection_header},
        controls::{
            ColorPopupParams, OptionalNumberParams, OptionalSliderParams, OptionalTextParams, SliderNumberParams, card_color_popup,
            card_password, card_slider_number, card_text, card_toggle, optional_number_row, optional_slider_row, optional_text_row,
        },
        shared::{ChromeSnap, UiCx, UiShared, mark_dirty, parts_to_argb_u32, send_overlay_display},
    },
};
pub fn api_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);
    let (api, optional, api_key_revealed) = {
        let ui = shared.lock();
        (ui.draft.api.clone(), ui.optional.clone(), ui.api_key_revealed)
    };

    let provider_card = settings_card("api-provider", "Provider", Some("How to reach the translation model."), {
        let idx = match api.provider {
            ModelProvider::OpenaiCompatible => 0,
            ModelProvider::GrokCli => 1,
            ModelProvider::CodexCli => 2,
        };
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
            hstack((
                api_radio("api-http-api", "Chat Completions", 168.0, idx == 0, Box::new(pick(0))),
                api_radio("api-http-api", "Responses", 120.0, idx == 1, Box::new(pick(1))),
            ))
            .spacing(12.0)
            .vertical_alignment(VerticalAlignment::Center)
        });

    let model_placeholder = match api.provider {
        ModelProvider::GrokCli => "grok-4.5",
        ModelProvider::CodexCli => "gpt-5.6",
        ModelProvider::OpenaiCompatible => "gpt-4o-mini",
    };
    let model_hint = if api.provider.is_cli() {
        "Model id passed to the local CLI."
    } else {
        "Model name, e.g. gpt-4o-mini."
    };

    let model_card = card_text("api-model", "Model", Some(model_hint), api.model.clone(), model_placeholder, {
        let cx = cx.clone();
        move |v| {
            cx.with_mut(|ui| {
                ui.draft.api.model = v;
                mark_dirty(ui);
            });
        }
    });

    let priority_tier_card = if api.provider == ModelProvider::CodexCli {
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
        .into()
    } else {
        Element::Empty
    };

    let connection = if api.provider.is_cli() {
        vstack((
            section_header("Connection"),
            provider_card,
            card_text(
                "api-cli-path",
                "CLI path",
                Some("Leave empty to use grok / codex on PATH. Uses your existing CLI login."),
                api.cli_path.clone(),
                api.provider.default_bin(),
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
                api.base_url.clone(),
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
        .spacing(4.0)
    };

    let sampling = vstack((
        section_header("Optional parameters"),
        text_block(if api.provider.is_cli() {
            "Turn On to include. Temperature, Top P, and Max tokens apply to HTTP only. Reasoning effort is sent to the CLI."
        } else {
            "Turn On to include in the request. Off = omit."
        })
        .font_size(12.0)
        .foreground(ThemeRef::SecondaryText)
        .wrap(),
        optional_slider_row(
            OptionalSliderParams {
                key: "api-temperature",
                header: "Temperature".into(),
                description: "Higher = more random (0–2).".into(),
                value: optional.temp_val,
                enabled: optional.temp_enabled,
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
                description: Some("For models that support it: low, medium, or high.".into()),
                text: optional.reasoning_str.clone(),
                enabled: optional.reasoning_enabled,
                placeholder: "low | medium | high".into(),
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
    ))
    .spacing(4.0);

    let reliability = vstack((
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
    ))
    .spacing(4.0);

    settings_page_shell(shared, chrome, bump, vstack((connection, sampling, reliability)).spacing(8.0))
}

fn api_radio(group: &'static str, label: &str, width: f64, checked: bool, on: Box<dyn Fn() + 'static>) -> RadioButton {
    let mut rb = RadioButton::new(label).group(group).checked(checked).on_checked(on);
    rb.modifiers.min_width = Some(width);
    rb.modifiers.width = Some(width);
    rb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);
    rb
}

/// Bind a line-merge f32 field from a slider/number value.
fn set_merge_f32(cx: &UiCx, set: impl FnOnce(&mut LineMergeConfig, f32), v: f64) {
    cx.with_mut(|ui| {
        set(&mut ui.draft.ocr.line_merge, v as f32);
        mark_dirty(ui);
    });
}

/// Percent slider: UI shows 0–100 (or a subrange); config stores the 0–1 ratio.
///
/// Slider min/max/step are the displayed percents so defaults land on ticks
/// (`min + n×step`). The stored ratio is `percent / 100`, clamped to the same range.
fn card_merge_pct(cx: &UiCx, p: SliderNumberParams, set: impl Fn(&mut LineMergeConfig, f32) + Copy + 'static) -> windows_reactor::Border {
    let lo = (p.min / 100.0) as f32;
    let hi = (p.max / 100.0) as f32;
    card_slider_number(p, {
        let cx = cx.clone();
        move |v| set_merge_f32(&cx, |m, x| set(m, (x / 100.0).clamp(lo, hi)), v)
    })
}

pub fn ocr_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);
    let (ocr, capture) = {
        let ui = shared.lock();
        (ui.draft.ocr.clone(), ui.draft.capture.clone())
    };

    let model = vstack((
        section_header("Model"),
        // Compact but readable: default RadioButton MinWidth (~120) spreads
        // short labels too far; zero padding/min-width crushes circle+text.
        // Cap width near content size and space items with hstack only.
        settings_card("ocr-tier", "Model size", Some("Smaller is faster; larger is more accurate. Reloads on Save."), {
            let idx = match ocr.model_tier {
                ModelTier::Tiny => 0,
                ModelTier::Small => 1,
                ModelTier::Medium => 2,
            };
            let cx_tier = cx.clone();
            let pick = move |choice: i32| {
                let cx = cx_tier.clone();
                move || {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.model_tier = match choice {
                            0 => ModelTier::Tiny,
                            2 => ModelTier::Medium,
                            _ => ModelTier::Small,
                        };
                        mark_dirty(ui);
                    });
                }
            };
            // Width ≈ glyph + label; leave template padding for circle↔text.
            let radio = |label: &str, width: f64, checked: bool, on: Box<dyn Fn() + 'static>| {
                let mut rb = RadioButton::new(label).group("ocr-model-tier").checked(checked).on_checked(on);
                rb.modifiers.min_width = Some(width);
                rb.modifiers.width = Some(width);
                rb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);
                rb
            };
            hstack((
                radio("tiny", 64.0, idx == 0, Box::new(pick(0))),
                radio("small", 72.0, idx == 1, Box::new(pick(1))),
                radio("medium", 84.0, idx == 2, Box::new(pick(2))),
            ))
            .spacing(12.0)
            .vertical_alignment(VerticalAlignment::Center)
        }),
    ))
    .spacing(4.0);

    let timing = vstack((
        section_header("Timing"),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-interval-ms",
                header: "Capture interval (ms)".into(),
                description: Some("How often to grab a new frame.".into()),
                value: capture.min_interval_ms as f64,
                min: 50.0,
                max: 5_000.0,
                step: 50.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.capture.min_interval_ms = v.max(50.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-stable-ms",
                header: "Stable wait (ms)".into(),
                description: Some("Wait until text stops changing, then translate.".into()),
                value: ocr.stable_duration_ms as f64,
                min: 0.0,
                max: 10_000.0,
                step: 50.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.stable_duration_ms = v.max(0.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-max-unstable-ms",
                header: "Force translate (ms)".into(),
                description: Some("If OCR keeps changing, translate anyway after this long. 0 = off.".into()),
                value: ocr.max_unstable_ms as f64,
                min: 0.0,
                max: 15_000.0,
                step: 100.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.max_unstable_ms = v.max(0.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-persist-ms",
                header: "Keep after gone (ms)".into(),
                description: Some("Keep text on overlay after it disappears. 0 = off.".into()),
                value: ocr.block_persist_ms as f64,
                min: 0.0,
                max: 5_000.0,
                step: 50.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.block_persist_ms = v.max(0.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-miss-ms",
                header: "Drop after miss (ms)".into(),
                description: Some("Remove text if not seen again within this time.".into()),
                value: ocr.block_max_miss_ms as f64,
                min: 0.0,
                max: 10_000.0,
                step: 50.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.block_max_miss_ms = v.max(0.0) as u64;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let order_idx = match ocr.line_merge.order {
        LineMergeOrder::TopToBottomLeftToRight => 0,
        LineMergeOrder::LeftToRightTopToBottom => 1,
    };
    let cx_order = cx.clone();
    let pick_order = move |choice: i32| {
        let cx = cx_order.clone();
        move || {
            cx.with_mut(|ui| {
                ui.draft.ocr.line_merge.order = match choice {
                    1 => LineMergeOrder::LeftToRightTopToBottom,
                    _ => LineMergeOrder::TopToBottomLeftToRight,
                };
                mark_dirty(ui);
            });
        }
    };
    // Shrink-wrap: a fixed width left-aligns the glyph inside the box.
    let order_radio = |label: &str, checked: bool, on: Box<dyn Fn() + 'static>| {
        let mut rb = RadioButton::new(label).group("ocr-merge-order").checked(checked).on_checked(on);
        rb.modifiers.horizontal_alignment = Some(HorizontalAlignment::Left);
        rb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);
        rb
    };

    let detection = vstack((
        section_header("Detection"),
        card_slider_number(
            SliderNumberParams {
                key: "ocr-confidence",
                header: "Min confidence".into(),
                description: Some("Ignore text below this score (0–1).".into()),
                value: f64::from(ocr.confidence_threshold),
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.confidence_threshold = v.clamp(0.0, 1.0) as f32;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_toggle(
            "ocr-filter-single",
            "Ignore single characters",
            Some("Drop lone single-character detections."),
            ocr.filter_single_char,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.filter_single_char = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let merge_join = vstack((
        section_header("Line merge"),
        card_toggle(
            "ocr-line-merge",
            "Merge lines",
            Some("Join nearby OCR lines that share a column or row, similar height, and a small gap."),
            ocr.line_merge.enabled,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.enabled = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_toggle(
            "ocr-merge-whole-region",
            "Merge entire selected region",
            Some("When OCR regions are set, join every line inside each region. Ignored for whole-window OCR."),
            ocr.line_merge.merge_whole_region,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.merge_whole_region = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_toggle(
            "ocr-merge-join-space",
            "Join with space",
            Some("On: insert a space between joined lines. Off: concatenate (typical for CJK)."),
            ocr.line_merge.join_with_space,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.join_with_space = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let merge_order = vstack((
        subsection_header("Reading order"),
        settings_card(
            "ocr-merge-order",
            "Merge order",
            Some("Reading order when joining lines inside a merged block."),
            vstack((
                order_radio("Left to right, then top to bottom", order_idx == 1, Box::new(pick_order(1))),
                order_radio("Top to bottom, then left to right", order_idx == 0, Box::new(pick_order(0))),
            ))
            .spacing(4.0)
            .horizontal_alignment(HorizontalAlignment::Right),
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-order-band",
                header: "Order band (% of window)".into(),
                description: Some("Row/column grouping width for reading order. Default 1.2.".into()),
                value: f64::from(ocr.line_merge.order_band_ratio) * 100.0,
                min: 0.1,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.order_band_ratio = x,
        ),
    ))
    .spacing(4.0);

    let merge_stacking = vstack((
        subsection_header("Vertical"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-gap",
                header: "Gap (% of window height)".into(),
                description: Some("Allowed |vertical gap|. Overlap and a small space count the same. Default 1.5.".into()),
                value: f64::from(ocr.line_merge.gap_ratio) * 100.0,
                min: 0.0,
                max: 8.0,
                step: 0.1,
            },
            |m, x| m.gap_ratio = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-below-mid",
                header: "Below-mid slack (% of line height)".into(),
                description: Some("How far a lower/right line may cross the mid and still count as below/right. Default 25.".into()),
                value: f64::from(ocr.line_merge.below_mid_ratio) * 100.0,
                min: 0.0,
                max: 50.0,
                step: 1.0,
            },
            |m, x| m.below_mid_ratio = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-height-delta",
                header: "Height delta (%)".into(),
                description: Some("Allowed |h1 − h2| / larger height. Default 45.".into()),
                value: f64::from(ocr.line_merge.height_delta_ratio) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.height_delta_ratio = x,
        ),
    ))
    .spacing(4.0);

    let merge_column = vstack((
        subsection_header("Horizontal"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-horizontal-gap",
                header: "Gap (% of window width)".into(),
                description: Some(
                    "Allowed |horizontal gap| for side-by-side lines. Overlap and a small space count the same. Default 1.5. Set 0 to disable."
                        .into(),
                ),
                value: f64::from(ocr.line_merge.horizontal_gap_ratio) * 100.0,
                min: 0.0,
                max: 8.0,
                step: 0.1,
            },
            |m, x| m.horizontal_gap_ratio = x,
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-align",
                header: "Align tolerance (% of window width)".into(),
                description: Some(
                    "Left-/center-edge delta for one column (× width), or top-/center for one row (× height). Default 1.2."
                        .into(),
                ),
                value: f64::from(ocr.line_merge.align_ratio) * 100.0,
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.align_ratio = x,
        ),
    ))
    .spacing(4.0);

    let merge_short = vstack((
        subsection_header("Short into long"),
        card_toggle(
            "ocr-merge-reject-short",
            "Don't merge short into long",
            Some("On: a shorter line above a much wider line stays its own block. Off: width is ignored."),
            ocr.line_merge.reject_short_long,
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        ui.draft.ocr.line_merge.reject_short_long = v;
                        mark_dirty(ui);
                    });
                }
            },
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-width-delta",
                header: "Width delta (%)".into(),
                description: Some("When the short-into-long guard is on: allowed (lower − upper) / lower width. Default 40.".into()),
                value: f64::from(ocr.line_merge.width_delta_ratio) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.width_delta_ratio = x,
        ),
    ))
    .spacing(4.0);

    let line_merge = vstack((merge_join, merge_order, merge_stacking, merge_column, merge_short)).spacing(4.0);

    settings_page_shell(shared, chrome, bump, vstack((model, timing, detection, line_merge)).spacing(8.0))
}

pub fn overlay_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);
    let (overlay, text_argb_str, bg_argb_str, text_color_picker_open, bg_color_picker_open) = {
        let ui = shared.lock();
        (ui.draft.overlay.clone(), ui.text_argb_str.clone(), ui.bg_argb_str.clone(), ui.text_color_picker_open, ui.bg_color_picker_open)
    };

    let display = vstack((
        section_header("Display"),
        card_toggle("ov-enabled", "In-place overlay", Some("Draw translations on the target window (click-through)."), overlay.enabled, {
            let cx = cx.clone();
            move |on| {
                cx.with_mut(|ui| {
                    if ui.draft.overlay.enabled != on {
                        let reader = ui.draft.overlay.reader_enabled;
                        send_overlay_display(ui, on, reader);
                    }
                });
            }
        }),
        card_toggle(
            "ov-reader",
            "Translation window",
            Some("Borderless always-on-top window. Drag to move, resize from the edges. Hide with this switch."),
            overlay.reader_enabled,
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        if ui.draft.overlay.reader_enabled != on {
                            let enabled = ui.draft.overlay.enabled;
                            send_overlay_display(ui, enabled, on);
                        }
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let colors = vstack((
        section_header("Appearance"),
        card_color_popup(
            ColorPopupParams {
                key: "ov-text-color",
                header: "Text color".into(),
                description: Some("Click the swatch to pick colour and opacity, or type 0xAARRGGBB hex.".into()),
                hex: text_argb_str.clone(),
                open: text_color_picker_open,
                alpha_enabled: true,
                placeholder: "0xFFFFFFFF".into(),
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        if ui.text_argb_str != v {
                            ui.text_argb_str = v;
                            mark_dirty(ui);
                        }
                    });
                }
            },
            {
                let cx = cx.clone();
                move |(a, r, g, b)| {
                    cx.with_mut(|ui| {
                        let v = parts_to_argb_u32(a, r, g, b);
                        let hex = format!("0x{v:08X}");
                        if ui.text_argb_str != hex {
                            ui.text_argb_str = hex;
                            mark_dirty(ui);
                        }
                    });
                }
            },
            {
                let cx = cx.clone();
                move || {
                    cx.with_mut(|ui| {
                        ui.text_color_picker_open = !ui.text_color_picker_open;
                        if ui.text_color_picker_open {
                            ui.bg_color_picker_open = false;
                        }
                    });
                }
            },
        ),
        card_color_popup(
            ColorPopupParams {
                key: "ov-bg-color",
                header: "Background color".into(),
                description: Some("Click the swatch to pick colour and opacity, or type 0xAARRGGBB hex.".into()),
                hex: bg_argb_str.clone(),
                open: bg_color_picker_open,
                alpha_enabled: true,
                placeholder: "0xC8000000".into(),
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        if ui.bg_argb_str != v {
                            ui.bg_argb_str = v;
                            mark_dirty(ui);
                        }
                    });
                }
            },
            {
                let cx = cx.clone();
                move |(a, r, g, b)| {
                    cx.with_mut(|ui| {
                        let v = parts_to_argb_u32(a, r, g, b);
                        let hex = format!("0x{v:08X}");
                        if ui.bg_argb_str != hex {
                            ui.bg_argb_str = hex;
                            mark_dirty(ui);
                        }
                    });
                }
            },
            {
                let cx = cx.clone();
                move || {
                    cx.with_mut(|ui| {
                        ui.bg_color_picker_open = !ui.bg_color_picker_open;
                        if ui.bg_color_picker_open {
                            ui.text_color_picker_open = false;
                        }
                    });
                }
            },
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ov-reader-font",
                header: "Font size".into(),
                description: Some("Segoe UI size for the translation window. Overlay captions still fit the source text.".into()),
                value: f64::from(overlay.reader_font_px),
                min: f64::from(READER_FONT_PX_MIN),
                max: f64::from(READER_FONT_PX_MAX),
                step: 1.0,
            },
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        let px = v.round().clamp(f64::from(READER_FONT_PX_MIN), f64::from(READER_FONT_PX_MAX)) as u32;
                        if ui.draft.overlay.reader_font_px != px {
                            ui.draft.overlay.reader_font_px = px;
                            mark_dirty(ui);
                        }
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let notes = vstack((
        section_header("Notes"),
        text_block("The in-place overlay is click-through and follows the target window only while it is in the foreground. The translation window is borderless and semi-transparent, uses the same colours and typeface, and stays visible independently.")
            .font_size(12.0)
            .foreground(ThemeRef::SecondaryText)
            .wrap(),
    ))
    .spacing(4.0);

    settings_page_shell(shared, chrome, bump, vstack((display, colors, notes)).spacing(8.0))
}

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
