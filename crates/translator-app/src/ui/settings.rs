//! Settings forms: API, translation, OCR, and overlay appearance.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{
    HttpApi, LineMergeConfig, LineMergeOrder, ModelProvider, ModelTier, READER_FONT_PX_MAX, READER_FONT_PX_MIN, ServiceTier,
    TRANSLATION_CACHE_MAX_CAP, TRANSLATION_CACHE_MAX_MIN,
};
use windows_reactor::{
    Button, ChildrenControl, ContentControl, HorizontalAlignment, LayoutControl, LocalSender, Orientation, RadioButton, StackPanel,
    TextBlock, TextBox, TextWrapping, ThemeBrush, TooltipExt, VerticalAlignment, View,
};

use crate::{
    pipeline::PipelineCommand,
    ui::{
        chrome::{section_header, settings_card, settings_card_stack, settings_page_shell, subsection_header},
        controls::{
            ColorPopupParams, OptionalNumberParams, OptionalSliderParams, OptionalTextParams, SliderNumberParams, card_color_popup,
            card_password, card_slider_number, card_text, card_toggle, optional_number_row, optional_slider_row, optional_text_row,
        },
        shared::{AppMsg, ChromeSnap, UiCx, UiShared, mark_dirty, send_overlay_display},
    },
};

fn radio(group: &'static str, label: &str, width: Option<f64>, checked: bool, on: impl Fn() + 'static) -> View {
    let mut rb = RadioButton::new()
        .group_name(group)
        .is_checked(checked)
        .on_checked(move |is_checked: bool| {
            if is_checked {
                on();
            }
        })
        .vertical_alignment(VerticalAlignment::Center);
    if let Some(w) = width {
        rb = rb.min_width(w).width(w);
    } else {
        rb = rb.horizontal_alignment(HorizontalAlignment::Left);
    }
    rb.content(label)
}

fn note(text: &str) -> TextBlock {
    TextBlock::new()
        .text(text)
        .font_size(12.0)
        .foreground(ThemeBrush::PrimaryText)
        .opacity(0.72)
        .text_wrapping(TextWrapping::WrapWholeWords)
}

pub fn api_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
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
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                radio("api-provider", "OpenAI-compatible", Some(168.0), idx == 0, pick(0)),
                radio("api-provider", "Grok CLI", Some(96.0), idx == 1, pick(1)),
                radio("api-provider", "Codex CLI", Some(104.0), idx == 2, pick(2)),
            ))
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
            note("A local agent session stays open and only the new turn is sent. Tools are denied."),
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
    };

    let sampling = StackPanel::new().spacing(4.0).children((
        section_header("Optional parameters"),
        note(if api.provider.is_cli() {
            "Turn On to include. Temperature, Top P, and Max tokens apply to HTTP only. Reasoning effort is sent to the CLI."
        } else {
            "Turn On to include in the request. Off = omit."
        }),
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

    settings_page_shell(shared, chrome, bump, StackPanel::new().spacing(8.0).children((connection, sampling, reliability)))
}

fn set_merge_f32(cx: &UiCx, set: impl FnOnce(&mut LineMergeConfig, f32), v: f64) {
    cx.with_mut(|ui| {
        set(&mut ui.draft.ocr.line_merge, v as f32);
        mark_dirty(ui);
    });
}

fn card_merge_pct(cx: &UiCx, p: SliderNumberParams, set: impl Fn(&mut LineMergeConfig, f32) + Copy + 'static) -> View {
    let lo = (p.min / 100.0) as f32;
    let hi = (p.max / 100.0) as f32;
    card_slider_number(p, {
        let cx = cx.clone();
        move |v| set_merge_f32(&cx, |m, x| set(m, (x / 100.0).clamp(lo, hi)), v)
    })
}

pub fn ocr_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let (ocr, capture) = {
        let ui = shared.lock();
        (ui.draft.ocr.clone(), ui.draft.capture.clone())
    };

    let model = StackPanel::new().spacing(4.0).children((
        section_header("Model"),
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
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .vertical_alignment(VerticalAlignment::Center)
                .children((
                    radio("ocr-model-tier", "tiny", Some(64.0), idx == 0, pick(0)),
                    radio("ocr-model-tier", "small", Some(72.0), idx == 1, pick(1)),
                    radio("ocr-model-tier", "medium", Some(84.0), idx == 2, pick(2)),
                ))
        }),
    ));

    let timing = StackPanel::new().spacing(4.0).children((
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
    ));

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
    let detection = StackPanel::new().spacing(4.0).children((
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
    ));

    let merge_join = StackPanel::new().spacing(4.0).children((
        section_header("Line merge"),
        card_toggle(
            "ocr-line-merge",
            "Merge lines",
            Some("Join nearby lines that share a column or row, similar height, and a small gap."),
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
            Some("Join every line in each drawn OCR region. Ignored for whole-window capture."),
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
            Some("On: space between joined lines. Off: glue them (typical for CJK)."),
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
    ));

    let merge_order = StackPanel::new().spacing(4.0).children((
        subsection_header("Reading order"),
        settings_card(
            "ocr-merge-order",
            "Reading order",
            Some("Order for joining lines and listing the blocks."),
            StackPanel::new()
                .spacing(4.0)
                .horizontal_alignment(HorizontalAlignment::Right)
                .children((
                    radio("ocr-merge-order", "Left to right, then top to bottom (rows)", None, order_idx == 1, pick_order(1)),
                    radio("ocr-merge-order", "Top to bottom, then left to right (columns)", None, order_idx == 0, pick_order(0)),
                )),
        ),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-order-band",
                header: "Row/column band (% of window)".into(),
                description: Some("How close lines must be to count as the same row or column. Default 1.2.".into()),
                value: f64::from(ocr.line_merge.order_band_ratio) * 100.0,
                min: 0.1,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.order_band_ratio = x,
        ),
    ));

    let merge_stacking = StackPanel::new().spacing(4.0).children((
        subsection_header("Vertical"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-gap",
                header: "Gap (% of window height)".into(),
                description: Some("Max distance between stacked lines. A small overlap counts as a small gap. Default 1.5.".into()),
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
                header: "Midpoint slack (% of line size)".into(),
                description: Some(
                    "How far a line may sit past the previous midpoint and still count as below or to the right. Default 25.".into(),
                ),
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
                header: "Height difference (%)".into(),
                description: Some("Max height difference vs the taller line. Default 45.".into()),
                value: f64::from(ocr.line_merge.height_delta_ratio) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.height_delta_ratio = x,
        ),
    ));

    let merge_column = StackPanel::new().spacing(4.0).children((
        subsection_header("Horizontal"),
        card_merge_pct(
            &cx,
            SliderNumberParams {
                key: "merge-horizontal-gap",
                header: "Gap (% of window width)".into(),
                description: Some(
                    "Max distance between side-by-side lines. A small overlap counts as a small gap. 0 = only if they touch. Default 1.5."
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
                header: "Align tolerance (%)".into(),
                description: Some("Max left/center drift for one column, or top/center for one row. Default 1.2.".into()),
                value: f64::from(ocr.line_merge.align_ratio) * 100.0,
                min: 0.0,
                max: 5.0,
                step: 0.1,
            },
            |m, x| m.align_ratio = x,
        ),
    ));

    let merge_short = StackPanel::new().spacing(4.0).children((
        subsection_header("Short into long"),
        card_toggle(
            "ocr-merge-reject-short",
            "Don't merge short into long",
            Some("Keep a short line separate from a much wider line below or to its right."),
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
                header: "Width difference (%)".into(),
                description: Some("How much wider that lower/right line may be before the short line stays separate. Default 40.".into()),
                value: f64::from(ocr.line_merge.width_delta_ratio) * 100.0,
                min: 0.0,
                max: 90.0,
                step: 1.0,
            },
            |m, x| m.width_delta_ratio = x,
        ),
    ));

    let line_merge = StackPanel::new()
        .spacing(4.0)
        .children((merge_join, merge_order, merge_stacking, merge_column, merge_short));

    settings_page_shell(shared, chrome, bump, StackPanel::new().spacing(8.0).children((model, timing, detection, line_merge)))
}

pub fn overlay_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let (overlay, text_argb_str, bg_argb_str, text_color_picker_open, bg_color_picker_open) = {
        let ui = shared.lock();
        (ui.draft.overlay.clone(), ui.text_argb_str.clone(), ui.bg_argb_str.clone(), ui.text_color_picker_open, ui.bg_color_picker_open)
    };

    let display = StackPanel::new().spacing(4.0).children((
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
    ));

    let colors = StackPanel::new().spacing(4.0).children((
        section_header("Appearance"),
        card_color_popup(
            ColorPopupParams {
                key: "ov-text-color",
                header: "Text color".into(),
                hex: text_argb_str.clone(),
                open: text_color_picker_open,
                placeholder: "#FFFFFFFF".into(),
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
                hex: bg_argb_str.clone(),
                open: bg_color_picker_open,
                placeholder: "#C8000000".into(),
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
    ));

    let notes = StackPanel::new()
        .spacing(4.0)
        .children((
            section_header("Notes"),
            note("The in-place overlay is click-through and follows the target window only while it is in the foreground. OBS Window Capture should use Translator Overlay Captions, not the control window. The translation window is borderless and semi-transparent, uses the same colours and typeface, and stays visible independently."),
        ));

    settings_page_shell(shared, chrome, bump, StackPanel::new().spacing(8.0).children((display, colors, notes)))
}

pub fn translation_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let (translation, cache_len) = {
        let ui = shared.lock();
        let cache_len = ui.state.read().translation_cache_len;
        (ui.draft.translation.clone(), cache_len)
    };

    let languages = StackPanel::new().spacing(4.0).children((
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
    ));

    let context = StackPanel::new().spacing(4.0).children((
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
    ));

    let cache = StackPanel::new().spacing(4.0).children((
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
            Button::new()
                .is_enabled(cache_len > 0)
                .on_click({
                    let cx = cx.clone();
                    move || cx.send_cmd(PipelineCommand::ClearTranslationCache)
                })
                .content("Clear cache")
                .tooltip("Drop all cached translations. Does not clear chat history."),
        ),
    ));

    let prompt = StackPanel::new().spacing(4.0).children((
        section_header("Prompt"),
        settings_card_stack(
            "tr-system-prompt",
            "System prompt",
            Some("Leave empty for the built-in prompt. Custom text must reply {\"b\":[[id,\"translation\"],...]}."),
            TextBox::new()
                .text(translation.system_prompt.clone().unwrap_or_default())
                .accepts_return(true)
                .text_wrapping(TextWrapping::Wrap)
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
    ));

    settings_page_shell(shared, chrome, bump, StackPanel::new().spacing(8.0).children((languages, context, cache, prompt)))
}
