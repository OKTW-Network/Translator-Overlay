//! OpenAI-compatible HTTP request/response types and parsers.

use serde::{Deserialize, Serialize};
use translator_core::{ApiConfig, HttpApi};

use crate::{Completion, TranslateError};

/// OpenAI `strict` requires every property in `required` and `additionalProperties: false`.
pub fn translation_json_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "blocks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "integer" },
                        "translation": { "type": "string" }
                    },
                    "required": ["id", "translation"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["blocks"],
        "additionalProperties": false
    })
}

fn is_false(v: &bool) -> bool {
    !*v
}

/// Chat Completions request body. Optional sampling fields are omitted when unset.
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequestBody<'a> {
    pub model: &'a str,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "is_false")]
    pub stream: bool,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub prompt_cache_key: &'a str,
    pub prompt_cache_retention: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<serde_json::Value>,
}

/// Body fragment used when calling the Responses API (`store: false` + encrypted reasoning).
#[derive(Debug, Clone, Serialize)]
pub struct ResponsesRequestBody<'a> {
    pub model: &'a str,
    pub input: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "is_false")]
    pub stream: bool,
    pub store: bool,
    pub include: &'a [&'a str],
    #[serde(skip_serializing_if = "str::is_empty")]
    pub prompt_cache_key: &'a str,
    pub prompt_cache_retention: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<serde_json::Value>,
}

pub fn chat_completion_body<'a>(api: &'a ApiConfig, messages: &'a [ChatMessage], session_id: &'a str) -> ChatCompletionRequestBody<'a> {
    let messages = if api.send_reasoning_content {
        messages.to_vec()
    } else {
        messages
            .iter()
            .map(|m| ChatMessage {
                reasoning_content: None,
                ..m.clone()
            })
            .collect()
    };
    ChatCompletionRequestBody {
        model: &api.model,
        messages,
        stream: api.stream,
        prompt_cache_key: session_id,
        prompt_cache_retention: "24h",
        temperature: api.temperature,
        top_p: api.top_p,
        max_tokens: api.max_tokens,
        reasoning_effort: api.reasoning_effort.as_deref(),
        response_format: api.structured_outputs.then(|| {
            serde_json::json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "translation_blocks",
                    "strict": true,
                    "schema": translation_json_schema()
                }
            })
        }),
    }
}

pub fn responses_request_body<'a>(api: &'a ApiConfig, items: &[ResponseItem], session_id: &'a str) -> ResponsesRequestBody<'a> {
    ResponsesRequestBody {
        model: &api.model,
        input: items.iter().map(ResponseItem::to_input_value).collect(),
        stream: api.stream,
        store: false,
        include: &["reasoning.encrypted_content"],
        prompt_cache_key: session_id,
        prompt_cache_retention: "24h",
        temperature: api.temperature,
        top_p: api.top_p,
        max_output_tokens: api.max_tokens,
        reasoning: api
            .reasoning_effort
            .as_deref()
            .map(|effort| serde_json::json!({ "effort": effort })),
        text: api.structured_outputs.then(|| {
            serde_json::json!({
                "format": {
                    "type": "json_schema",
                    "name": "translation_blocks",
                    "strict": true,
                    "schema": translation_json_schema()
                }
            })
        }),
    }
}

/// One item in a stateless Responses `input` list (authored message or API output).
#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Message {
        role: String,
        content: String,
        reasoning_content: Option<String>,
    },
    /// Pass-through `/responses` output item (reasoning or assistant message).
    Output(serde_json::Value),
}

impl ResponseItem {
    pub fn message(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Message {
            role: role.into(),
            content: content.into(),
            reasoning_content: None,
        }
    }

    pub fn to_input_value(&self) -> serde_json::Value {
        match self {
            Self::Message { role, content, .. } if role == "assistant" => serde_json::json!({
                "type": "message",
                "role": role,
                "content": [{ "type": "output_text", "text": content }],
            }),
            Self::Message { role, content, .. } => serde_json::json!({
                "role": role,
                "content": content,
            }),
            Self::Output(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            reasoning_content: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            reasoning_content: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            reasoning_content: None,
        }
    }
}

pub fn completion_from_http_body(http_api: HttpApi, body: &str) -> Result<Completion, TranslateError> {
    let trimmed = body.trim();
    let sse = trimmed.starts_with("data:") || trimmed.starts_with("event:") || trimmed.starts_with(':');
    match http_api {
        HttpApi::ChatCompletions if sse => completion_from_chat_sse(trimmed),
        HttpApi::Responses if sse => completion_from_responses_sse(trimmed),
        HttpApi::ChatCompletions => extract_chat_completion(trimmed),
        HttpApi::Responses => extract_responses_completion(trimmed),
    }
}

fn extract_chat_completion(response_json: &str) -> Result<Completion, TranslateError> {
    #[derive(Deserialize)]
    struct Root {
        choices: Vec<Choice>,
    }
    #[derive(Deserialize)]
    struct Choice {
        message: Msg,
    }
    #[derive(Deserialize)]
    struct Msg {
        content: Option<String>,
        refusal: Option<String>,
        reasoning_content: Option<String>,
        reasoning: Option<serde_json::Value>,
    }

    let root: Root = serde_json::from_str(response_json).map_err(|e| TranslateError::Parse(e.to_string()))?;
    let msg = root
        .choices
        .into_iter()
        .next()
        .map(|c| c.message)
        .ok_or_else(|| TranslateError::Parse("no choices/content in response".into()))?;
    if let Some(refusal) = msg.refusal.filter(|s| !s.is_empty()) {
        return Err(TranslateError::Parse(format!("model refused: {refusal}")));
    }
    let text = msg
        .content
        .ok_or_else(|| TranslateError::Parse("no choices/content in response".into()))?;
    if text.trim().is_empty() {
        return Err(TranslateError::Parse("no choices/content in response".into()));
    }
    Ok(chat_completion_result(text, chat_reasoning_text(msg.reasoning_content.as_deref(), msg.reasoning.as_ref())))
}

fn chat_reasoning_text(reasoning_content: Option<&str>, reasoning: Option<&serde_json::Value>) -> Option<String> {
    if let Some(s) = reasoning_content.filter(|s| !s.is_empty()) {
        return Some(s.to_string());
    }
    match reasoning {
        Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn chat_completion_result(text: String, reasoning_content: Option<String>) -> Completion {
    // Clone is intentional: history needs its own owned copy.
    let content = text.clone();
    Completion {
        replay_items: vec![ResponseItem::Message {
            role: "assistant".to_string(),
            content,
            reasoning_content,
        }],
        text,
    }
}

pub(crate) fn extract_responses_completion(response_json: &str) -> Result<Completion, TranslateError> {
    let root: serde_json::Value = serde_json::from_str(response_json).map_err(|e| TranslateError::Parse(e.to_string()))?;
    extract_responses_value(&root)
}

fn extract_responses_value(root: &serde_json::Value) -> Result<Completion, TranslateError> {
    let output = root
        .get("output")
        .and_then(|v| v.as_array())
        .ok_or_else(|| TranslateError::Parse("no output in responses body".into()))?;

    let mut text = String::new();
    let mut replay_items = Vec::new();

    for item in output {
        let ty = item.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
        match ty {
            "reasoning" => {
                let non_empty_array = |key: &str| item.get(key).is_some_and(|v| v.as_array().is_some_and(|a| !a.is_empty()));
                let has_content = item
                    .get("encrypted_content")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|s| !s.is_empty())
                    || non_empty_array("summary")
                    || non_empty_array("content");
                if has_content {
                    replay_items.push(ResponseItem::Output(item.clone()));
                }
            }
            "message" => {
                let role = item.get("role").and_then(serde_json::Value::as_str).unwrap_or("assistant");
                let mut content_text = String::new();
                match item.get("content") {
                    Some(serde_json::Value::String(s)) => content_text = s.clone(),
                    Some(serde_json::Value::Array(parts)) => {
                        for part in parts {
                            if part.get("type").and_then(serde_json::Value::as_str) == Some("refusal") {
                                let refusal = part.get("refusal").and_then(serde_json::Value::as_str).unwrap_or("model refused");
                                return Err(TranslateError::Parse(format!("model refused: {refusal}")));
                            }
                            let Some(piece) = part.get("text").and_then(serde_json::Value::as_str).filter(|s| !s.is_empty()) else {
                                continue;
                            };
                            if !content_text.is_empty() {
                                content_text.push('\n');
                            }
                            content_text.push_str(piece);
                        }
                    }
                    _ => {}
                }
                if role != "assistant" || content_text.is_empty() {
                    continue;
                }
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&content_text);
                replay_items.push(ResponseItem::Output(item.clone()));
            }
            _ => {}
        }
    }

    if text.trim().is_empty() {
        return Err(TranslateError::Parse("no message/content in responses output".into()));
    }
    Ok(Completion { text, replay_items })
}

fn completion_from_chat_sse(body: &str) -> Result<Completion, TranslateError> {
    let mut text = String::new();
    let mut reasoning = String::new();
    let mut refusal = None;
    for ev in sse_events(body) {
        if ev.data.trim() == "[DONE]" {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&ev.data) else {
            continue;
        };
        if let Some(err) = stream_event_error(ev.event.as_deref(), &v) {
            return Err(err);
        }
        apply_chat_delta(&v, &mut text, &mut refusal, &mut reasoning);
    }
    if let Some(refusal) = refusal.filter(|s| !s.is_empty()) {
        return Err(TranslateError::Parse(format!("model refused: {refusal}")));
    }
    if text.trim().is_empty() {
        return Err(TranslateError::Parse("no choices/content in response".into()));
    }
    Ok(chat_completion_result(text, (!reasoning.is_empty()).then_some(reasoning)))
}

fn apply_chat_delta(v: &serde_json::Value, text: &mut String, refusal: &mut Option<String>, reasoning: &mut String) {
    let Some(choices) = v.get("choices").and_then(|c| c.as_array()) else {
        return;
    };
    for choice in choices {
        let delta = choice.get("delta").or_else(|| choice.get("message"));
        let Some(delta) = delta else {
            continue;
        };
        if let Some(r) = delta.get("refusal").and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
            *refusal = Some(r.to_string());
        }
        match delta.get("content") {
            Some(serde_json::Value::String(s)) => text.push_str(s),
            Some(serde_json::Value::Array(parts)) => {
                for part in parts {
                    if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                        text.push_str(t);
                    }
                }
            }
            _ => {}
        }
        if let Some(piece) = chat_reasoning_text(delta.get("reasoning_content").and_then(serde_json::Value::as_str), delta.get("reasoning"))
        {
            reasoning.push_str(&piece);
        }
    }
}

fn completion_from_responses_sse(body: &str) -> Result<Completion, TranslateError> {
    let mut completed: Option<serde_json::Value> = None;
    let mut items: Vec<serde_json::Value> = Vec::new();
    for ev in sse_events(body) {
        if ev.data.trim() == "[DONE]" {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&ev.data) else {
            continue;
        };
        if let Some(err) = stream_event_error(ev.event.as_deref(), &v) {
            return Err(err);
        }
        let ty = ev
            .event
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| v.get("type").and_then(serde_json::Value::as_str))
            .unwrap_or("");
        match ty {
            "response.completed" => {
                completed = Some(v.get("response").cloned().unwrap_or(v));
            }
            "response.output_item.done" => {
                if let Some(item) = v.get("item") {
                    items.push(item.clone());
                }
            }
            _ => {}
        }
    }
    if let Some(resp) = completed {
        return extract_responses_value(&resp);
    }
    if !items.is_empty() {
        return extract_responses_value(&serde_json::json!({ "output": items }));
    }
    Err(TranslateError::Parse("no message/content in responses output".into()))
}

struct SseEvent {
    event: Option<String>,
    data: String,
}

fn stream_event_error(event: Option<&str>, v: &serde_json::Value) -> Option<TranslateError> {
    if let Some(err) = v.get("error").filter(|e| !e.is_null()) {
        return Some(stream_api_error(err));
    }
    if let Some(err) = v.pointer("/response/error").filter(|e| !e.is_null()) {
        return Some(stream_api_error(err));
    }
    let ty = event
        .filter(|s| !s.is_empty())
        .or_else(|| v.get("type").and_then(serde_json::Value::as_str))
        .unwrap_or("");
    match ty {
        "error" | "response.failed" => Some(TranslateError::ApiStatus {
            status: 500,
            body: v.to_string(),
        }),
        "response.incomplete" => Some(TranslateError::Parse(format!(
            "response incomplete: {}",
            v.pointer("/response/incomplete_details/reason")
                .or_else(|| v.get("reason"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
        ))),
        _ => None,
    }
}

fn stream_api_error(err: &serde_json::Value) -> TranslateError {
    let msg = err
        .get("message")
        .and_then(serde_json::Value::as_str)
        .or_else(|| err.as_str())
        .unwrap_or("stream error");
    TranslateError::ApiStatus {
        status: 500,
        body: msg.to_string(),
    }
}

fn sse_events(body: &str) -> Vec<SseEvent> {
    let normalized = body.replace("\r\n", "\n");
    normalized.split("\n\n").filter_map(parse_sse_block).collect()
}

fn parse_sse_block(block: &str) -> Option<SseEvent> {
    let mut event = None;
    let mut data = String::new();
    for line in block.lines() {
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("event:") {
            event = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest);
        }
    }
    if data.is_empty() { None } else { Some(SseEvent { event, data }) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_body_has_stream_and_prompt_cache_key() {
        let api = ApiConfig::default();
        let messages = [ChatMessage::user("hi")];
        let json = serde_json::to_value(chat_completion_body(&api, &messages, "sess-1")).unwrap();
        assert_eq!(json["stream"], true);
        assert_eq!(json["prompt_cache_key"], "sess-1");
        assert_eq!(json["prompt_cache_retention"], "24h");
        assert!(json.get("temperature").is_none());
        assert_eq!(json["response_format"]["type"], "json_schema");
        assert_eq!(json["response_format"]["json_schema"]["name"], "translation_blocks");
        assert_eq!(json["response_format"]["json_schema"]["strict"], true);
    }

    #[test]
    fn chat_body_omits_empty_cache_key_and_response_format_when_off() {
        let api = ApiConfig {
            structured_outputs: false,
            ..ApiConfig::default()
        };
        let messages = [ChatMessage::user("hi")];
        let json = serde_json::to_value(chat_completion_body(&api, &messages, "")).unwrap();
        assert!(json.get("prompt_cache_key").is_none());
        assert_eq!(json["prompt_cache_retention"], "24h");
        assert!(json.get("response_format").is_none());
        assert_eq!(json["stream"], true);
    }

    #[test]
    fn chat_and_responses_bodies_omit_stream_when_off() {
        let api = ApiConfig {
            stream: false,
            ..ApiConfig::default()
        };
        let messages = [ChatMessage::user("hi")];
        let chat = serde_json::to_value(chat_completion_body(&api, &messages, "sess-1")).unwrap();
        assert!(chat.get("stream").is_none());

        let items = [ResponseItem::message("user", "hi")];
        let responses = serde_json::to_value(responses_request_body(&api, &items, "sess-1")).unwrap();
        assert!(responses.get("stream").is_none());
    }

    #[test]
    fn optional_api_params_included_when_set() {
        let api = ApiConfig {
            temperature: Some(0.2),
            reasoning_effort: Some("medium".to_string()),
            ..ApiConfig::default()
        };
        let messages = [ChatMessage::user("hi")];
        let json = serde_json::to_value(chat_completion_body(&api, &messages, "s")).unwrap();
        let temp = json["temperature"].as_f64().unwrap();
        assert!((temp - 0.2).abs() < 1e-5);
        assert_eq!(json["reasoning_effort"], "medium");
        assert!(json.get("top_p").is_none());
    }

    #[test]
    fn responses_body_is_stateless_and_omits_unset_params() {
        let api = ApiConfig {
            model: "grok-4.6".into(),
            ..ApiConfig::default()
        };
        let items = [ResponseItem::message("user", "hi")];
        let json = serde_json::to_value(responses_request_body(&api, &items, "sess-1")).unwrap();
        assert_eq!(json["model"], "grok-4.6");
        assert_eq!(json["stream"], true);
        assert_eq!(json["store"], false);
        assert_eq!(json["include"], serde_json::json!(["reasoning.encrypted_content"]));
        assert_eq!(json["prompt_cache_key"], "sess-1");
        assert_eq!(json["prompt_cache_retention"], "24h");
        assert_eq!(json["input"][0]["role"], "user");
        assert_eq!(json["input"][0]["content"], "hi");
        assert!(json.get("temperature").is_none());
        assert!(json.get("top_p").is_none());
        assert!(json.get("max_output_tokens").is_none());
        assert!(json.get("reasoning").is_none());
        assert!(json.get("previous_response_id").is_none());
        assert!(json.get("max_tokens").is_none());
    }

    #[test]
    fn responses_body_maps_tokens_and_effort() {
        let api = ApiConfig {
            max_tokens: Some(256),
            reasoning_effort: Some("high".into()),
            temperature: Some(0.2),
            ..ApiConfig::default()
        };
        let items = [
            ResponseItem::message("system", "sys"),
            ResponseItem::message("user", "u"),
            ResponseItem::Output(serde_json::json!({
                "type": "reasoning",
                "id": "rs_1",
                "encrypted_content": "enc",
            })),
            ResponseItem::Output(serde_json::json!({
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{ "type": "output_text", "text": "a" }],
            })),
        ];
        let json = serde_json::to_value(responses_request_body(&api, &items, "sess-2")).unwrap();
        assert_eq!(json["max_output_tokens"], 256);
        assert_eq!(json["reasoning"]["effort"], "high");
        assert!((json["temperature"].as_f64().unwrap() - 0.2).abs() < 1e-5);
        assert!(json.get("reasoning_effort").is_none());
        assert_eq!(json["input"][2]["type"], "reasoning");
        assert_eq!(json["input"][2]["encrypted_content"], "enc");
        assert_eq!(json["input"][2]["id"], "rs_1");
        assert_eq!(json["input"][3]["type"], "message");
        assert_eq!(json["input"][3]["id"], "msg_1");
        assert_eq!(json["input"][3]["content"][0]["text"], "a");
    }

    #[test]
    fn responses_body_structured_outputs_toggle() {
        let items = [ResponseItem::message("user", "hi")];
        let on = serde_json::to_value(responses_request_body(&ApiConfig::default(), &items, "sess-1")).unwrap();
        assert_eq!(on["text"]["format"]["type"], "json_schema");
        assert_eq!(on["text"]["format"]["name"], "translation_blocks");
        assert_eq!(on["text"]["format"]["strict"], true);

        let off_api = ApiConfig {
            structured_outputs: false,
            ..ApiConfig::default()
        };
        let off = serde_json::to_value(responses_request_body(&off_api, &items, "sess-1")).unwrap();
        assert!(off.get("text").is_none());
    }

    #[test]
    fn chat_json_body_without_sse_is_accepted() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"hello"}}]}"#;
        let completion = completion_from_http_body(HttpApi::ChatCompletions, body).unwrap();
        assert_eq!(completion.text, "hello");
        assert_eq!(completion.replay_items.len(), 1);
        assert!(matches!(
            &completion.replay_items[0],
            ResponseItem::Message { role, content, reasoning_content } if role == "assistant" && content == "hello" && reasoning_content.is_none()
        ));
    }

    #[test]
    fn responses_json_body_without_sse_is_accepted() {
        let body = r#"{"output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi"}]}]}"#;
        let completion = completion_from_http_body(HttpApi::Responses, body).unwrap();
        assert_eq!(completion.text, "hi");
        assert_eq!(completion.replay_items.len(), 1);
    }

    #[test]
    fn chat_sse_accumulates_deltas() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\n\n\
                    data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n\
                    data: [DONE]\n\n";
        let completion = completion_from_http_body(HttpApi::ChatCompletions, body).unwrap();
        assert_eq!(completion.text, "hello");
        assert_eq!(completion.replay_items.len(), 1);
        assert!(matches!(
            &completion.replay_items[0],
            ResponseItem::Message { content, reasoning_content, .. } if content == "hello" && reasoning_content.is_none()
        ));
    }

    #[test]
    fn chat_sse_reports_refusal() {
        let body = "data: {\"choices\":[{\"delta\":{\"refusal\":\"I can't help with that.\"}}]}\n\n";
        let err = completion_from_http_body(HttpApi::ChatCompletions, body).unwrap_err();
        assert!(err.to_string().contains("I can't help with that."), "{err}");
        assert!(!err.is_retryable());
    }

    #[test]
    fn chat_sse_keeps_deltas_when_a_later_event_has_message() {
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\n\n\
                    data: {\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":null}}]}\n\n\
                    data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n\
                    data: [DONE]\n\n";
        let completion = completion_from_http_body(HttpApi::ChatCompletions, body).unwrap();
        assert_eq!(completion.text, "hello");
        assert_eq!(completion.replay_items.len(), 1);
    }

    #[test]
    fn chat_sse_error_event_is_retryable() {
        let body = "data: {\"error\":{\"message\":\"overloaded\",\"type\":\"server_error\"}}\n\n";
        let err = completion_from_http_body(HttpApi::ChatCompletions, body).unwrap_err();
        assert!(err.to_string().contains("overloaded"), "{err}");
        assert!(err.is_retryable());
    }

    #[test]
    fn responses_sse_uses_completed_snapshot() {
        let body = "event: response.completed\n\
                    data: {\"type\":\"response.completed\",\"response\":{\"output\":[\
                      {\"type\":\"reasoning\",\"encrypted_content\":\"encblob\"},\
                      {\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"hi\"}]}\
                    ]}}\n\n";
        let completion = completion_from_http_body(HttpApi::Responses, body).unwrap();
        assert_eq!(completion.text, "hi");
        assert_eq!(completion.replay_items.len(), 2);
    }

    #[test]
    fn responses_sse_falls_back_to_output_item_done() {
        let body = "event: response.output_item.done\n\
                    data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"reasoning\",\"encrypted_content\":\"enc\"}}\n\n\
                    event: response.output_item.done\n\
                    data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"ok\"}]}}\n\n";
        let completion = completion_from_http_body(HttpApi::Responses, body).unwrap();
        assert_eq!(completion.text, "ok");
        assert_eq!(completion.replay_items.len(), 2);
    }

    #[test]
    fn responses_sse_failed_does_not_commit_partial_items() {
        let body = "event: response.output_item.done\n\
                    data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"partial\"}]}}\n\n\
                    event: response.failed\n\
                    data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"message\":\"server overloaded\"}}}\n\n";
        let err = completion_from_http_body(HttpApi::Responses, body).unwrap_err();
        assert!(err.to_string().contains("server overloaded"), "{err}");
        assert!(err.is_retryable());
    }

    #[test]
    fn responses_sse_incomplete_is_not_success() {
        let body = "event: response.output_item.done\n\
                    data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"cut\"}]}}\n\n\
                    event: response.incomplete\n\
                    data: {\"type\":\"response.incomplete\",\"response\":{\"incomplete_details\":{\"reason\":\"max_output_tokens\"}}}\n\n";
        let err = completion_from_http_body(HttpApi::Responses, body).unwrap_err();
        assert!(err.to_string().contains("max_output_tokens"), "{err}");
        assert!(!err.is_retryable());
    }

    #[test]
    fn chat_reasoning_json_variants() {
        let cases = [
            (r#"{"choices":[{"message":{"role":"assistant","content":"hi","reasoning_content":"think step"}}]}"#, Some("think step")),
            (r#"{"choices":[{"message":{"role":"assistant","content":"hi","reasoning":"alt think"}}]}"#, Some("alt think")),
            (
                r#"{"choices":[{"message":{"role":"assistant","content":"hi","reasoning_content":"primary","reasoning":"secondary"}}]}"#,
                Some("primary"),
            ),
            (r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}]}"#, None),
        ];
        for (body, expected) in cases {
            let c = completion_from_http_body(HttpApi::ChatCompletions, body).unwrap();
            assert_eq!(c.text, "hi");
            assert!(
                matches!(&c.replay_items[0], ResponseItem::Message { reasoning_content, .. } if reasoning_content.as_deref() == expected),
                "body={body}"
            );
        }
    }

    #[test]
    fn chat_sse_reasoning_variants() {
        for (body, expected) in [
            (
                "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"think \"}}]}\n\n\
                  data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"step\"}}]}\n\n\
                  data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
                  data: [DONE]\n\n",
                "think step",
            ),
            (
                "data: {\"choices\":[{\"delta\":{\"reasoning\":\"r1\"}}]}\n\n\
                  data: {\"choices\":[{\"delta\":{\"reasoning\":\" r2\"}}]}\n\n\
                  data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
                  data: [DONE]\n\n",
                "r1 r2",
            ),
        ] {
            let c = completion_from_http_body(HttpApi::ChatCompletions, body).unwrap();
            assert_eq!(c.text, "hi");
            assert!(
                matches!(&c.replay_items[0], ResponseItem::Message { reasoning_content: Some(s), .. } if s == expected),
                "expected {expected}"
            );
        }
    }

    #[test]
    fn responses_reasoning_keep_or_drop() {
        let cases = [
            (
                r#"{"output":[{"type":"reasoning","id":"rs_1","summary":[{"type":"summary_text","text":"thinking"}]},{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi"}]}]}"#,
                2,
                true,
            ),
            (
                r#"{"output":[{"type":"reasoning","id":"rs_1","content":[{"type":"reasoning_text","text":"step by step"}]},{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi"}]}]}"#,
                2,
                true,
            ),
            (
                r#"{"output":[{"type":"reasoning","id":"rs_1","encrypted_content":"","summary":[{"type":"summary_text","text":"x"}]},{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi"}]}]}"#,
                2,
                true,
            ),
            (
                r#"{"output":[{"type":"reasoning","id":"rs_1"},{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi"}]}]}"#,
                1,
                false,
            ),
            (
                r#"{"output":[{"type":"reasoning","id":"rs_1","summary":[]},{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi"}]}]}"#,
                1,
                false,
            ),
            (
                r#"{"output":[{"type":"reasoning","id":"rs_1","content":[]},{"type":"message","role":"assistant","content":[{"type":"output_text","text":"hi"}]}]}"#,
                1,
                false,
            ),
        ];
        for (body, len, has_reasoning) in cases {
            let c = completion_from_http_body(HttpApi::Responses, body).unwrap();
            assert_eq!(c.replay_items.len(), len, "body={body}");
            if has_reasoning {
                assert!(
                    matches!(&c.replay_items[0], ResponseItem::Output(v) if v.get("type").and_then(|x| x.as_str()) == Some("reasoning")),
                    "body={body}"
                );
            } else {
                assert!(
                    matches!(&c.replay_items[0], ResponseItem::Output(v) if v.get("type").and_then(|x| x.as_str()) == Some("message")),
                    "body={body}"
                );
            }
        }
        let body = "event: response.output_item.done\n\
                    data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"reasoning\",\"id\":\"rs_1\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"think\"}]}}\n\n\
                    event: response.output_item.done\n\
                    data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"ok\"}]}}\n\n";
        let c = completion_from_http_body(HttpApi::Responses, body).unwrap();
        assert_eq!(c.text, "ok");
        assert_eq!(c.replay_items.len(), 2);
    }

    #[test]
    fn chat_history_serializes_reasoning_for_next_turn() {
        let msgs = vec![
            ChatMessage {
                role: "system".into(),
                content: "sys".into(),
                reasoning_content: None,
            },
            ChatMessage {
                role: "assistant".into(),
                content: "hello".into(),
                reasoning_content: Some("prior think".into()),
            },
        ];
        let api = ApiConfig {
            send_reasoning_content: false,
            ..ApiConfig::default()
        };
        let json = serde_json::to_value(chat_completion_body(&api, &msgs, "sess-1")).unwrap();
        assert!(json["messages"][1].get("reasoning_content").is_none());
        let json = serde_json::to_value(chat_completion_body(&ApiConfig::default(), &msgs, "sess-1")).unwrap();
        assert_eq!(json["messages"][1]["reasoning_content"], "prior think");
        assert!(json["messages"][0].get("reasoning_content").is_none());
    }
}
