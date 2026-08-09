//! OpenAI-compatible translation client and conversation context management.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use translator_core::{ApiConfig, ChatMessage, OcrBlock, TranslatedBlock, TranslationConfig};

#[derive(Debug, Error)]
pub enum TranslateError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API returned status {status}: {body}")]
    ApiStatus { status: u16, body: String },
    #[error("failed to parse API response: {0}")]
    Parse(String),
    #[error("API key is empty")]
    MissingApiKey,
    #[error("translation cancelled")]
    Cancelled,
    #[error("{0}")]
    Other(String),
}

impl TranslateError {
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    /// Whether a retry might help (network / 5xx / timeout — not auth / cancel).
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(e) => e.is_timeout() || e.is_connect() || e.is_request(),
            Self::ApiStatus { status, .. } => *status >= 500 || *status == 429,
            Self::Cancelled | Self::MissingApiKey | Self::Parse(_) | Self::Other(_) => false,
        }
    }
}

/// In-memory multi-turn conversation reused across translations.
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    pub messages: Vec<ChatMessage>,
    /// Number of completed user/assistant turn pairs.
    pub turn_count: usize,
}

impl Conversation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_system(&mut self, system: impl Into<String>) {
        if self.messages.first().map(|m| m.role.as_str()) != Some("system") {
            self.messages.insert(0, ChatMessage::system(system));
        } else {
            self.messages[0] = ChatMessage::system(system);
        }
    }

    pub fn push_user(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage::user(content));
    }

    pub fn push_assistant(&mut self, content: impl Into<String>) {
        self.messages.push(ChatMessage::assistant(content));
        self.turn_count += 1;
    }

    /// Drop all messages (e.g. new capture target).
    pub fn clear(&mut self) {
        self.messages.clear();
        self.turn_count = 0;
    }

    /// When `turn_count` reaches `max_turns`, keep only the system message plus
    /// the last `history_max_items` user/assistant pairs and reset the counter.
    pub fn compress_if_needed(&mut self, max_turns: usize, history_max_items: usize) {
        if self.turn_count < max_turns {
            return;
        }
        let system = self.messages.iter().find(|m| m.role == "system").cloned();

        // Collect trailing non-system messages (user/assistant pairs).
        let non_system: Vec<_> = self.messages.iter().filter(|m| m.role != "system").cloned().collect();

        let keep_msgs = history_max_items.saturating_mul(2);
        let start = non_system.len().saturating_sub(keep_msgs);
        let kept = non_system[start..].to_vec();

        self.messages.clear();
        if let Some(sys) = system {
            self.messages.push(sys);
        }
        self.messages.extend(kept);
        self.turn_count = self.messages.iter().filter(|m| m.role == "assistant").count();
    }

    /// Compress + system prompt + user payload for a new translate request.
    ///
    /// Returns a clone of `messages` for the HTTP call and the length after the
    /// user turn (for [`Self::rollback_user_turn`] on failure / cancel).
    pub fn begin_translate_request(&mut self, translation_cfg: &TranslationConfig, blocks: &[OcrBlock]) -> (Vec<ChatMessage>, usize) {
        self.compress_if_needed(translation_cfg.conversation_max_turns, translation_cfg.history_max_items);
        self.ensure_system(default_system_prompt(translation_cfg));
        self.push_user(user_payload_from_blocks(blocks));
        let messages_len_after_user = self.messages.len();
        (self.messages.clone(), messages_len_after_user)
    }

    /// Pop the pending user turn when its length still matches a failed request.
    pub fn rollback_user_turn(&mut self, messages_len_after_user: usize) {
        if self.messages.len() == messages_len_after_user && self.messages.last().map(|m| m.role == "user").unwrap_or(false) {
            self.messages.pop();
        }
    }
}

/// Build default system prompt for structured block translation.
pub fn default_system_prompt(cfg: &TranslationConfig) -> String {
    if let Some(custom) = &cfg.system_prompt {
        return custom.clone();
    }
    format!(
        "You are a translation engine. Translate text from {src} to {dst}.\n\
         Input is JSON with OCR blocks (id, text).\n\
         Reply with a single JSON object only — no markdown fences, no commentary:\n\
         {{\"blocks\":[{{\"id\":0,\"translation\":\"...\"}}]}}\n\
         Escape quotes and backslashes inside strings. No trailing commas.\n\
         Preserve proper nouns when appropriate.",
        src = cfg.source_lang,
        dst = cfg.target_lang
    )
}

/// Serialize OCR blocks as the user message payload.
pub fn user_payload_from_blocks(blocks: &[OcrBlock]) -> String {
    #[derive(Serialize)]
    struct Payload<'a> {
        blocks: Vec<BlockIn<'a>>,
    }
    #[derive(Serialize)]
    struct BlockIn<'a> {
        id: u32,
        text: &'a str,
    }
    let payload = Payload {
        blocks: blocks.iter().map(|b| BlockIn { id: b.id, text: &b.text }).collect(),
    };
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
}

#[derive(Debug, Deserialize)]
struct TranslationResponse {
    blocks: Vec<TranslationBlockOut>,
}

#[derive(Debug, Deserialize)]
struct TranslationBlockOut {
    /// Models sometimes emit string ids (`"0"`).
    #[serde(deserialize_with = "deserialize_block_id")]
    id: u32,
    /// Accept common alternate field names from loose model output.
    #[serde(alias = "text", alias = "translated", alias = "target")]
    translation: String,
}

fn deserialize_block_id<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use std::fmt;

    use serde::de::{self, Visitor};

    struct IdVisitor;
    impl<'de> Visitor<'de> for IdVisitor {
        type Value = u32;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("u32 or stringified u32")
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<u32, E> {
            u32::try_from(v).map_err(E::custom)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<u32, E> {
            u32::try_from(v).map_err(E::custom)
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<u32, E> {
            v.trim().parse().map_err(|_| E::custom(format!("invalid block id: {v}")))
        }
    }

    deserializer.deserialize_any(IdVisitor)
}

/// Merge LLM JSON output with original OCR blocks.
pub fn merge_translations(source: &[OcrBlock], response_json: &str) -> Result<Vec<TranslatedBlock>, TranslateError> {
    let blocks = parse_translation_blocks(response_json)?;

    let mut out = Vec::with_capacity(source.len());
    for src in source {
        let translation = blocks
            .iter()
            .find(|b| b.id == src.id)
            .map(|b| b.translation.clone())
            .unwrap_or_else(|| src.text.clone());
        out.push(TranslatedBlock {
            id: src.id,
            source: src.text.clone(),
            translation,
            confidence: src.confidence,
            bbox: src.bbox,
            source_lines: src.source_lines.max(1),
        });
    }
    Ok(out)
}

fn parse_translation_blocks(response_json: &str) -> Result<Vec<TranslationBlockOut>, TranslateError> {
    let candidates = json_parse_candidates(response_json);
    let mut last_err = String::new();

    for candidate in &candidates {
        match try_parse_blocks(candidate) {
            Ok(blocks) => return Ok(blocks),
            Err(e) => last_err = e,
        }
    }

    let snippet = truncate_for_error(response_json.trim(), 240);
    Err(TranslateError::Parse(format!("{last_err}; content snippet: {snippet:?}")))
}

fn try_parse_blocks(json: &str) -> Result<Vec<TranslationBlockOut>, String> {
    // Preferred shape: {"blocks":[...]}
    if let Ok(parsed) = serde_json::from_str::<TranslationResponse>(json) {
        return Ok(parsed.blocks);
    }
    // Bare array: [{"id":0,"translation":"..."}]
    if let Ok(blocks) = serde_json::from_str::<Vec<TranslationBlockOut>>(json) {
        return Ok(blocks);
    }
    // Single object: {"id":0,"translation":"..."}
    if let Ok(block) = serde_json::from_str::<TranslationBlockOut>(json) {
        return Ok(vec![block]);
    }

    let err = serde_json::from_str::<TranslationResponse>(json)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "unrecognized translation JSON shape".into());
    Err(err)
}

/// Produce increasingly repaired JSON candidates for lenient model output.
fn json_parse_candidates(raw: &str) -> Vec<String> {
    let trimmed = raw.trim().trim_start_matches('\u{feff}');
    let unfenced = strip_markdown_fence(trimmed);
    let mut out = Vec::new();

    let push = |out: &mut Vec<String>, s: String| {
        if !s.is_empty() && !out.iter().any(|x| x == &s) {
            out.push(s);
        }
    };

    // 1) Whole string as-is (after fence strip).
    push(&mut out, unfenced.clone());

    // 2) Balanced `{...}` / `[...]` extraction (ignore prose around JSON).
    if let Some(obj) = extract_balanced(unfenced.as_str(), '{', '}') {
        push(&mut out, obj.to_string());
    }
    if let Some(arr) = extract_balanced(unfenced.as_str(), '[', ']') {
        push(&mut out, arr.to_string());
    }

    // 3) Same candidates with trailing commas removed (common LLM mistake).
    let base_len = out.len();
    for i in 0..base_len {
        let cleaned = strip_trailing_commas(&out[i]);
        if cleaned != out[i] {
            push(&mut out, cleaned);
        }
    }

    out
}

fn strip_markdown_fence(s: &str) -> String {
    let t = s.trim();
    if !t.starts_with("```") {
        return t.to_string();
    }
    let mut lines = t.lines();
    let first = lines.next().unwrap_or("");
    if !first.starts_with("```") {
        return t.to_string();
    }
    let mut body: Vec<&str> = lines.collect();
    if body.last().is_some_and(|l| l.trim().starts_with("```")) {
        body.pop();
    }
    body.join("\n").trim().to_string()
}

/// Extract the first balanced `open`…`close` region, respecting JSON strings.
fn extract_balanced(s: &str, open: char, close: char) -> Option<&str> {
    let start = s.find(open)?;
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut i = start;

    while i < s.len() {
        let ch = s[i..].chars().next()?;
        let ch_len = ch.len_utf8();

        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += ch_len;
            continue;
        }

        match ch {
            '"' => in_string = true,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..i + ch_len]);
                }
            }
            _ => {}
        }
        // Safety: only walk UTF-8 char boundaries.
        let _ = bytes;
        i += ch_len;
    }
    None
}

/// Remove trailing commas before `}` / `]` outside of strings.
fn strip_trailing_commas(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_string = false;
    let mut escape = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            i += 1;
            continue;
        }

        if ch == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                // Skip the trailing comma (keep following whitespace for readability).
                i += 1;
                continue;
            }
        }

        out.push(ch);
        i += 1;
    }
    out
}

fn truncate_for_error(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let kept: String = s.chars().take(max_chars).collect();
    format!("{kept}…")
}

/// HTTP client for OpenAI-compatible chat completions.
#[derive(Debug, Clone)]
pub struct TranslateClient {
    http: reqwest::Client,
    api: ApiConfig,
}

impl TranslateClient {
    pub fn new(api: ApiConfig) -> Self {
        Self {
            http: build_http_client(&api),
            api,
        }
    }

    pub fn update_api(&mut self, api: ApiConfig) {
        // Rebuild client so timeout reflects the latest config.
        self.http = build_http_client(&api);
        self.api = api;
    }

    pub fn api(&self) -> &ApiConfig {
        &self.api
    }

    pub async fn chat_completions(&self, messages: &[ChatMessage]) -> Result<String, TranslateError> {
        self.chat_completions_cancellable(messages, &CancellationToken::new()).await
    }

    pub async fn chat_completions_cancellable(
        &self,
        messages: &[ChatMessage],
        cancel: &CancellationToken,
    ) -> Result<String, TranslateError> {
        if self.api.api_key.trim().is_empty() {
            return Err(TranslateError::MissingApiKey);
        }
        if cancel.is_cancelled() {
            return Err(TranslateError::Cancelled);
        }

        let url = format!("{}/chat/completions", self.api.base_url.trim_end_matches('/'));
        let body = self.api.request_body(messages);

        let send = self.http.post(&url).bearer_auth(&self.api.api_key).json(&body).send();

        let response = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(TranslateError::Cancelled),
            result = send => result?,
        };

        let status = response.status();
        let text = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(TranslateError::Cancelled),
            result = response.text() => result?,
        };

        if !status.is_success() {
            return Err(TranslateError::ApiStatus {
                status: status.as_u16(),
                body: text,
            });
        }

        extract_assistant_content(&text)
    }

    /// Chat completions with automatic retries for transient failures.
    pub async fn chat_completions_with_retry(
        &self,
        messages: &[ChatMessage],
        cancel: &CancellationToken,
    ) -> Result<String, TranslateError> {
        let max_retries = self.api.max_retries;
        let mut backoff_ms = self.api.retry_backoff_ms.max(50);
        let mut attempt = 0u32;

        loop {
            match self.chat_completions_cancellable(messages, cancel).await {
                Ok(content) => return Ok(content),
                Err(e) if e.is_cancelled() => return Err(e),
                Err(e) if e.is_retryable() && attempt < max_retries => {
                    attempt += 1;
                    tracing::warn!(
                        attempt,
                        max_retries,
                        backoff_ms,
                        error = %e,
                        "translate request failed; retrying"
                    );
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => return Err(TranslateError::Cancelled),
                        _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {}
                    }
                    backoff_ms = backoff_ms.saturating_mul(2).min(8_000);
                }
                Err(e) => return Err(e),
            }
        }
    }
}

fn build_http_client(api: &ApiConfig) -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if api.request_timeout_secs > 0 {
        builder = builder.timeout(Duration::from_secs(api.request_timeout_secs));
    }
    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

fn extract_assistant_content(response_json: &str) -> Result<String, TranslateError> {
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
    }

    let root: Root = serde_json::from_str(response_json).map_err(|e| TranslateError::Parse(e.to_string()))?;
    root.choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or_else(|| TranslateError::Parse("no choices/content in response".into()))
}

/// Join translated block texts for UI / history.
pub fn blocks_to_translated_text(blocks: &[TranslatedBlock]) -> String {
    blocks.iter().map(|b| b.translation.as_str()).collect::<Vec<_>>().join("\n")
}

/// High-level translate step with conversation reuse + compression.
pub async fn translate_blocks(
    client: &TranslateClient,
    conversation: &mut Conversation,
    translation_cfg: &TranslationConfig,
    blocks: &[OcrBlock],
) -> Result<Vec<TranslatedBlock>, TranslateError> {
    translate_blocks_cancellable(client, conversation, translation_cfg, blocks, &CancellationToken::new()).await
}

/// Same as [`translate_blocks`], but aborts when `cancel` is triggered.
///
/// On failure / cancel the pending user message is popped so the conversation
/// stays consistent for a later retry.
pub async fn translate_blocks_cancellable(
    client: &TranslateClient,
    conversation: &mut Conversation,
    translation_cfg: &TranslationConfig,
    blocks: &[OcrBlock],
    cancel: &CancellationToken,
) -> Result<Vec<TranslatedBlock>, TranslateError> {
    if blocks.is_empty() {
        return Ok(Vec::new());
    }

    let (messages, messages_len_after_user) = conversation.begin_translate_request(translation_cfg, blocks);

    let content = match client.chat_completions_with_retry(&messages, cancel).await {
        Ok(c) => c,
        Err(e) => {
            conversation.rollback_user_turn(messages_len_after_user);
            return Err(e);
        }
    };

    match merge_translations(blocks, &content) {
        Ok(translated) => {
            conversation.push_assistant(&content);
            Ok(translated)
        }
        Err(e) => {
            conversation.rollback_user_turn(messages_len_after_user);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use translator_core::{ApiConfig, Rect};

    use super::*;

    #[test]
    fn request_omits_unset_params() {
        let api = ApiConfig::default();
        let msgs = [ChatMessage::user("x")];
        let body = api.request_body(&msgs);
        let v = serde_json::to_value(&body).unwrap();
        assert!(v.get("temperature").is_none());
        assert!(v.get("reasoning_effort").is_none());
    }

    #[test]
    fn compress_keeps_recent_pairs() {
        let mut conv = Conversation::new();
        conv.ensure_system("sys");
        for i in 0..5 {
            conv.push_user(format!("u{i}"));
            conv.push_assistant(format!("a{i}"));
        }
        assert_eq!(conv.turn_count, 5);
        conv.compress_if_needed(5, 2);
        // system + 2 pairs (4 messages) = 5
        assert_eq!(conv.messages.len(), 5);
        assert_eq!(conv.messages[0].role, "system");
        assert_eq!(conv.turn_count, 2);
    }

    #[test]
    fn merge_translations_maps_ids() {
        let source = vec![OcrBlock {
            id: 1,
            text: "Hello".into(),
            confidence: 0.9,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let json = r#"{"blocks":[{"id":1,"translation":"T1"}]}"#;
        let out = merge_translations(&source, json).unwrap();
        assert_eq!(out[0].translation, "T1");
    }

    #[test]
    fn clear_resets_conversation() {
        let mut conv = Conversation::new();
        conv.ensure_system("sys");
        conv.push_user("u");
        conv.push_assistant("a");
        conv.clear();
        assert!(conv.messages.is_empty());
        assert_eq!(conv.turn_count, 0);
    }

    #[test]
    fn strips_markdown_fence_around_json() {
        let source = vec![OcrBlock {
            id: 0,
            text: "Hi".into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let json = "```json\n{\"blocks\":[{\"id\":0,\"translation\":\"T2\"}]}\n```";
        let out = merge_translations(&source, json).unwrap();
        assert_eq!(out[0].translation, "T2");
    }

    #[test]
    fn merge_tolerates_trailing_commas_and_prose() {
        let source = vec![OcrBlock {
            id: 0,
            text: "Hi".into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let json = r#"Here you go:
{
  "blocks": [
    {
      "id": 0,
      "translation": "T3",
    },
  ],
}
Hope that helps!"#;
        let out = merge_translations(&source, json).unwrap();
        assert_eq!(out[0].translation, "T3");
    }

    #[test]
    fn merge_accepts_string_ids_and_alias_fields() {
        let source = vec![OcrBlock {
            id: 2,
            text: "Bye".into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let json = r#"{"blocks":[{"id":"2","text":"T4"}]}"#;
        let out = merge_translations(&source, json).unwrap();
        assert_eq!(out[0].translation, "T4");
    }

    #[test]
    fn merge_accepts_bare_array() {
        let source = vec![OcrBlock {
            id: 1,
            text: "A".into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let json = r#"[{"id":1,"translation":"T5"}]"#;
        let out = merge_translations(&source, json).unwrap();
        assert_eq!(out[0].translation, "T5");
    }

    #[test]
    fn cancelled_is_not_retryable() {
        assert!(!TranslateError::Cancelled.is_retryable());
        assert!(TranslateError::Cancelled.is_cancelled());
        assert!(
            TranslateError::ApiStatus {
                status: 503,
                body: "x".into()
            }
            .is_retryable()
        );
        assert!(!TranslateError::MissingApiKey.is_retryable());
    }
}
