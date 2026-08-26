//! Translation client: OpenAI-compatible HTTP or long-lived Grok/Codex CLI sessions.

mod cache;
mod cli;

use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_util::sync::CancellationToken;
use translator_core::{ApiConfig, ChatMessage, HttpApi, OcrBlock, ResponseItem, TranslatedBlock, TranslationConfig};

pub use crate::cache::{CacheResolve, TranslationCache};

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
    #[error("CLI not found: {0}")]
    CliNotFound(String),
    #[error("CLI exited: {0}")]
    CliExit(String),
    #[error("CLI protocol: {0}")]
    CliProtocol(String),
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
            Self::CliExit(_) => true,
            Self::CliProtocol(msg) => msg.contains("timed out") || msg.contains("closed stdout"),
            Self::Cancelled | Self::MissingApiKey | Self::CliNotFound(_) | Self::Parse(_) | Self::Other(_) => false,
        }
    }
}

/// True when another attempt should run after `attempt` (0-based completed tries).
pub fn should_retry(error: &TranslateError, attempt: u32, max_retries: u32) -> bool {
    !error.is_cancelled() && error.is_retryable() && attempt < max_retries
}

fn new_session_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}-{n:x}-{}", std::process::id())
}

/// Model output: assistant text plus Responses items to replay on the next turn.
#[derive(Debug, Clone)]
pub struct Completion {
    pub text: String,
    pub replay_items: Vec<ResponseItem>,
}

/// In-memory multi-turn conversation reused across translations.
///
/// `items` is the source of truth (authored messages + Responses output). Chat Completions / CLI
/// use [`Self::messages`], which drops pass-through output items.
#[derive(Debug, Clone)]
pub struct Conversation {
    pub items: Vec<ResponseItem>,
    /// Client-generated id sent as `x-grok-session-id` / `prompt_cache_key`. Rotates on history rewrite.
    pub session_id: String,
}

impl Conversation {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            session_id: new_session_id(),
        }
    }

    fn rotate_session_id(&mut self) {
        self.session_id = new_session_id();
    }

    pub fn messages(&self) -> Vec<ChatMessage> {
        self.items
            .iter()
            .filter_map(|item| match item {
                ResponseItem::Message { role, content } => Some(ChatMessage {
                    role: role.clone(),
                    content: content.clone(),
                }),
                ResponseItem::Output(_) => None,
            })
            .collect()
    }

    pub fn turn_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| match item {
                ResponseItem::Message { role, .. } if role == "assistant" => true,
                ResponseItem::Output(v) => {
                    v.get("type").and_then(serde_json::Value::as_str) == Some("message")
                        && v.get("role").and_then(serde_json::Value::as_str).unwrap_or("assistant") == "assistant"
                }
                _ => false,
            })
            .count()
    }

    pub fn ensure_system(&mut self, system: impl Into<String>) {
        let system = system.into();
        match self.items.first_mut() {
            Some(ResponseItem::Message { role, content }) if role == "system" => {
                if *content != system {
                    *content = system;
                    self.rotate_session_id();
                }
            }
            _ => self.items.insert(0, ResponseItem::message("system", system)),
        }
    }

    pub fn push_user(&mut self, content: impl Into<String>) {
        self.items.push(ResponseItem::message("user", content));
    }

    pub fn push_assistant(&mut self, content: impl Into<String>) {
        self.items.push(ResponseItem::message("assistant", content));
    }

    /// Append a successful model turn. `replay_items` (Responses) replace a plain assistant message on the tape.
    pub fn commit_completion(&mut self, completion: &Completion) {
        if completion.replay_items.is_empty() {
            self.items.push(ResponseItem::message("assistant", &completion.text));
        } else {
            self.items.extend(completion.replay_items.iter().cloned());
        }
    }

    /// Drop all messages (e.g. new capture target) and mint a new session id.
    pub fn clear(&mut self) {
        self.items.clear();
        self.rotate_session_id();
    }

    /// When turn count reaches `max_turns`, keep only the system message plus
    /// the last `history_max_items` user turns (with their reasoning/assistant items).
    /// Rotates `session_id` when turns are actually dropped.
    pub fn compress_if_needed(&mut self, max_turns: usize, history_max_items: usize) {
        if self.turn_count() < max_turns {
            return;
        }
        let users: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, it)| matches!(it, ResponseItem::Message { role, .. } if role == "user").then_some(i))
            .collect();
        let keep = users
            .get(users.len().saturating_sub(history_max_items))
            .copied()
            .unwrap_or(self.items.len());
        let sys = matches!(self.items.first(), Some(ResponseItem::Message { role, .. }) if role == "system");
        let from = if sys { keep.max(1) } else { keep };
        if from <= usize::from(sys) {
            return;
        }
        self.items.drain(usize::from(sys)..from);
        self.rotate_session_id();
    }

    /// Compress + system prompt + user payload for a new translate request.
    /// Returns a snapshot used by the HTTP/CLI job; the live conversation stays on the caller.
    pub fn begin_translate_request(&mut self, translation_cfg: &TranslationConfig, blocks: &[OcrBlock]) -> Self {
        self.compress_if_needed(translation_cfg.conversation_max_turns, translation_cfg.history_max_items);
        self.ensure_system(default_system_prompt(translation_cfg));
        self.push_user(user_payload_from_blocks(blocks));
        self.clone()
    }

    /// Pop a pending user turn after a failed or cancelled request.
    pub fn rollback_user_turn(&mut self) {
        if matches!(self.items.last(), Some(ResponseItem::Message { role, .. }) if role == "user") {
            self.items.pop();
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
    Ok(merge_translations_detailed(source, response_json)?.blocks)
}

/// Same as [`merge_translations`], plus which block ids the model actually returned.
pub fn merge_translations_detailed(source: &[OcrBlock], response_json: &str) -> Result<MergeOutcome, TranslateError> {
    let parsed = parse_translation_blocks(response_json)?;
    let model_ids: HashSet<u32> = parsed.iter().map(|b| b.id).collect();

    let mut blocks = Vec::with_capacity(source.len());
    for src in source {
        let translation = parsed
            .iter()
            .find(|b| b.id == src.id)
            .map(|b| b.translation.clone())
            .unwrap_or_else(|| src.text.clone());
        blocks.push(TranslatedBlock {
            id: src.id,
            source: src.text.clone(),
            translation,
            confidence: src.confidence,
            bbox: src.bbox,
            source_lines: src.source_lines.max(1),
        });
    }
    Ok(MergeOutcome { blocks, model_ids })
}

/// Parsed model translations plus the ids present in the JSON (not fallbacks).
#[derive(Debug, Clone)]
pub struct MergeOutcome {
    pub blocks: Vec<TranslatedBlock>,
    pub model_ids: HashSet<u32>,
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

struct CliHandle {
    backend: tokio::sync::Mutex<cli::CliBackend>,
    epoch: AtomicU64,
}

impl std::fmt::Debug for CliHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliHandle")
            .field("epoch", &self.epoch.load(Ordering::Relaxed))
            .finish()
    }
}

/// HTTP or long-lived CLI translation backend.
#[derive(Debug, Clone)]
pub struct TranslateClient {
    http: reqwest::Client,
    api: ApiConfig,
    cli: Arc<CliHandle>,
}

impl TranslateClient {
    pub fn new(api: ApiConfig) -> Self {
        Self {
            http: build_http_client(&api),
            api,
            cli: Arc::new(CliHandle {
                backend: tokio::sync::Mutex::new(cli::CliBackend::new()),
                epoch: AtomicU64::new(0),
            }),
        }
    }

    pub fn update_api(&mut self, api: ApiConfig) {
        // Rebuild client so timeout reflects the latest config.
        self.http = build_http_client(&api);
        let session_changed = self.api.provider != api.provider
            || self.api.cli_path != api.cli_path
            || self.api.model != api.model
            || self.api.reasoning_effort != api.reasoning_effort
            || self.api.service_tier != api.service_tier
            || self.api.http_api != api.http_api;
        self.api = api;
        if session_changed {
            self.reset_session();
        }
    }

    /// Drop the long-lived Grok/Codex session (Reset / new capture target).
    pub fn reset_session(&self) {
        self.cli.epoch.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut backend) = self.cli.backend.try_lock() {
            backend.shutdown();
        }
    }

    pub fn api(&self) -> &ApiConfig {
        &self.api
    }

    pub async fn complete_cancellable(&self, conv: &Conversation, cancel: &CancellationToken) -> Result<Completion, TranslateError> {
        if !self.api.provider.is_cli() && self.api.http_api == HttpApi::Responses {
            let body = self.api.responses_request_body(&conv.items, &conv.session_id);
            let text = self
                .http_post_json(
                    &format!("{}/responses", self.api.base_url.trim_end_matches('/')),
                    &body,
                    Some(conv.session_id.as_str()),
                    cancel,
                )
                .await?;
            return extract_responses_completion(&text);
        }
        let messages = conv.messages();
        let text = if self.api.provider.is_cli() {
            self.cli_complete(&messages, cancel).await?
        } else {
            extract_assistant_content(
                &self
                    .http_post_json(
                        &format!("{}/chat/completions", self.api.base_url.trim_end_matches('/')),
                        &self.api.request_body(&messages),
                        None,
                        cancel,
                    )
                    .await?,
            )?
        };
        Ok(Completion {
            text,
            replay_items: Vec::new(),
        })
    }

    async fn http_post_json<T: Serialize>(
        &self,
        url: &str,
        body: &T,
        session_id: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<String, TranslateError> {
        if self.api.api_key.trim().is_empty() {
            return Err(TranslateError::MissingApiKey);
        }
        if cancel.is_cancelled() {
            return Err(TranslateError::Cancelled);
        }

        let mut req = self.http.post(url).bearer_auth(&self.api.api_key);
        if let Some(session_id) = session_id.filter(|s| !s.is_empty()) {
            req = req.header("x-grok-session-id", session_id);
        }
        let send = req.json(body).send();

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
        Ok(text)
    }

    async fn cli_complete(&self, messages: &[ChatMessage], cancel: &CancellationToken) -> Result<String, TranslateError> {
        if cancel.is_cancelled() {
            return Err(TranslateError::Cancelled);
        }
        let timeout = if self.api.request_timeout_secs == 0 {
            Duration::from_secs(3600)
        } else {
            Duration::from_secs(self.api.request_timeout_secs)
        };
        let epoch = self.cli.epoch.load(Ordering::SeqCst);
        let mut backend = self.cli.backend.lock().await;
        if epoch != self.cli.epoch.load(Ordering::SeqCst) {
            backend.close().await;
        }
        backend.complete(&self.api, messages, cancel, timeout, epoch).await
    }

    pub async fn complete_with_retry_on(
        &self,
        conv: &Conversation,
        cancel: &CancellationToken,
        mut on_retry: impl FnMut(u32, u32, &TranslateError, u64),
    ) -> Result<Completion, TranslateError> {
        let max_retries = self.api.max_retries;
        let mut backoff_ms = self.api.retry_backoff_ms.max(50);
        let mut attempt = 0u32;

        loop {
            match self.complete_cancellable(conv, cancel).await {
                Ok(content) => return Ok(content),
                Err(e) if should_retry(&e, attempt, max_retries) => {
                    attempt += 1;
                    tracing::warn!(
                        attempt,
                        max_retries,
                        backoff_ms,
                        error = %e,
                        "translate request failed; retrying"
                    );
                    on_retry(attempt, max_retries, &e, backoff_ms);
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

fn extract_responses_completion(response_json: &str) -> Result<Completion, TranslateError> {
    let root: serde_json::Value = serde_json::from_str(response_json).map_err(|e| TranslateError::Parse(e.to_string()))?;
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
                if item
                    .get("encrypted_content")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|s| !s.is_empty())
                {
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
    translate_blocks_cancellable(client, conversation, translation_cfg, blocks, None, false, &CancellationToken::new()).await
}

/// Same as [`translate_blocks`], but aborts when `cancel` is triggered.
///
/// When `cache` is `Some` and `force` is false, cached block texts are omitted
/// from the HTTP payload. On failure / cancel the pending user message is popped
/// so the conversation stays consistent for a later retry.
pub async fn translate_blocks_cancellable(
    client: &TranslateClient,
    conversation: &mut Conversation,
    translation_cfg: &TranslationConfig,
    blocks: &[OcrBlock],
    mut cache: Option<&mut TranslationCache>,
    force: bool,
    cancel: &CancellationToken,
) -> Result<Vec<TranslatedBlock>, TranslateError> {
    if blocks.is_empty() {
        return Ok(Vec::new());
    }

    let resolved = cache.as_mut().map(|c| c.resolve(blocks, translation_cfg, force));
    if let Some(resolved) = resolved.as_ref()
        && resolved.misses.is_empty()
    {
        return Ok(TranslationCache::stitch(blocks, &resolved.hits, &[]));
    }

    let to_send: &[OcrBlock] = resolved.as_ref().map(|r| r.misses.as_slice()).unwrap_or(blocks);
    let prepared = conversation.begin_translate_request(translation_cfg, to_send);

    let completion = match client.complete_with_retry_on(&prepared, cancel, |_, _, _, _| {}).await {
        Ok(c) => c,
        Err(e) => {
            conversation.rollback_user_turn();
            return Err(e);
        }
    };

    match merge_translations_detailed(to_send, &completion.text) {
        Ok(outcome) => {
            conversation.commit_completion(&completion);
            if let (Some(cache), Some(resolved)) = (cache.as_mut(), resolved.as_ref()) {
                cache.store_model_pairs(&resolved.misses, &outcome.blocks, &outcome.model_ids, translation_cfg);
            }
            let translated = match resolved.as_ref() {
                Some(resolved) => TranslationCache::stitch(blocks, &resolved.hits, &outcome.blocks),
                None => outcome.blocks,
            };
            Ok(translated)
        }
        Err(e) => {
            conversation.rollback_user_turn();
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
    fn changing_service_tier_resets_cli_session() {
        let api = ApiConfig {
            provider: translator_core::ModelProvider::CodexCli,
            ..ApiConfig::default()
        };
        let mut client = TranslateClient::new(api.clone());
        let initial_epoch = client.cli.epoch.load(Ordering::SeqCst);

        let mut updated = api;
        updated.service_tier = translator_core::ServiceTier::Priority;
        client.update_api(updated);

        assert_eq!(client.cli.epoch.load(Ordering::SeqCst), initial_epoch + 1);
    }

    #[test]
    fn changing_http_api_resets_session() {
        let api = ApiConfig::default();
        let mut client = TranslateClient::new(api.clone());
        let initial_epoch = client.cli.epoch.load(Ordering::SeqCst);

        let mut updated = api;
        updated.http_api = HttpApi::Responses;
        client.update_api(updated);

        assert_eq!(client.cli.epoch.load(Ordering::SeqCst), initial_epoch + 1);
    }

    #[test]
    fn compress_keeps_recent_pairs() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        for i in 0..5 {
            conv.push_user(format!("u{i}"));
            conv.push_assistant(format!("a{i}"));
        }
        assert_eq!(conv.turn_count(), 5);
        conv.compress_if_needed(5, 2);
        // system + 2 pairs (4 messages) = 5
        let messages = conv.messages();
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0].role, "system");
        assert_eq!(conv.turn_count(), 2);
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
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        conv.push_user("u");
        conv.push_assistant("a");
        let old_session = conv.session_id.clone();
        conv.clear();
        assert!(conv.messages().is_empty());
        assert!(conv.items.is_empty());
        assert_eq!(conv.turn_count(), 0);
        assert!(!conv.session_id.is_empty());
        assert_ne!(conv.session_id, old_session);
    }

    #[test]
    fn new_conversation_has_session_id() {
        let conv = Conversation::empty();
        assert!(!conv.session_id.is_empty());
        assert_ne!(Conversation::empty().session_id, conv.session_id);
    }

    #[test]
    fn append_keeps_session_id() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        let id = conv.session_id.clone();
        conv.push_user("u");
        conv.push_assistant("a");
        assert_eq!(conv.session_id, id);
    }

    #[test]
    fn rollback_keeps_session_id() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        conv.push_user("u");
        let id = conv.session_id.clone();
        conv.rollback_user_turn();
        assert_eq!(conv.session_id, id);
        assert_eq!(conv.messages().len(), 1);
        assert_eq!(conv.items.len(), 1);
    }

    #[test]
    fn compress_without_drop_keeps_session_id() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        for i in 0..2 {
            conv.push_user(format!("u{i}"));
            conv.push_assistant(format!("a{i}"));
        }
        let id = conv.session_id.clone();
        conv.compress_if_needed(2, 8);
        assert_eq!(conv.session_id, id);
        assert_eq!(conv.turn_count(), 2);
    }

    #[test]
    fn compress_drop_rotates_session_and_keeps_reasoning() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        for i in 0..5 {
            conv.push_user(format!("u{i}"));
            conv.items.push(ResponseItem::Output(serde_json::json!({
                "type": "reasoning",
                "encrypted_content": format!("enc{i}"),
            })));
            conv.push_assistant(format!("a{i}"));
        }
        let old_session = conv.session_id.clone();
        conv.compress_if_needed(5, 2);
        assert_ne!(conv.session_id, old_session);
        assert_eq!(conv.turn_count(), 2);
        assert_eq!(conv.messages().len(), 5);
        let reasoning: Vec<_> = conv
            .items
            .iter()
            .filter_map(|i| match i {
                ResponseItem::Output(v) => v.get("encrypted_content").and_then(serde_json::Value::as_str),
                _ => None,
            })
            .collect();
        assert_eq!(reasoning, ["enc3", "enc4"]);
        assert!(!conv.items.iter().any(
            |i| matches!(i, ResponseItem::Output(v) if v.get("encrypted_content").and_then(serde_json::Value::as_str) == Some("enc0"))
        ));
    }

    #[test]
    fn ensure_system_change_rotates_session_id() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys-a");
        let id = conv.session_id.clone();
        conv.ensure_system("sys-a");
        assert_eq!(conv.session_id, id);
        conv.ensure_system("sys-b");
        assert_ne!(conv.session_id, id);
    }

    #[test]
    fn extract_responses_output_with_reasoning() {
        let json = r#"{
          "id": "resp_1",
          "output": [
            {
              "type": "reasoning",
              "id": "rs_1",
              "status": "completed",
              "encrypted_content": "encblob",
              "summary": []
            },
            {
              "type": "message",
              "id": "msg_1",
              "role": "assistant",
              "status": "completed",
              "content": [{"type": "output_text", "text": "{\"blocks\":[]}"}]
            }
          ]
        }"#;
        let completion = extract_responses_completion(json).unwrap();
        assert_eq!(completion.text, "{\"blocks\":[]}");
        assert_eq!(completion.replay_items.len(), 2);
        assert!(matches!(
            &completion.replay_items[0],
            ResponseItem::Output(v) if v.get("encrypted_content").and_then(serde_json::Value::as_str) == Some("encblob")
        ));
        assert!(matches!(
            &completion.replay_items[1],
            ResponseItem::Output(v)
                if v.get("id").and_then(serde_json::Value::as_str) == Some("msg_1")
                    && v.get("role").and_then(serde_json::Value::as_str) == Some("assistant")
        ));
    }

    #[test]
    fn extract_responses_output_without_reasoning() {
        let json = r#"{
          "output": [
            {
              "type": "message",
              "role": "assistant",
              "content": [{"type": "output_text", "text": "hello"}]
            }
          ]
        }"#;
        let completion = extract_responses_completion(json).unwrap();
        assert_eq!(completion.text, "hello");
        assert_eq!(completion.replay_items.len(), 1);
        assert!(matches!(&completion.replay_items[0], ResponseItem::Output(_)));
    }

    #[test]
    fn turn_count_includes_output_assistant() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        conv.push_user("u");
        conv.items.push(ResponseItem::Output(serde_json::json!({
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": "a" }],
        })));
        assert_eq!(conv.turn_count(), 1);
    }

    #[test]
    fn extract_skips_empty_and_non_assistant_messages() {
        let json = r#"{
          "output": [
            {"type": "message", "role": "assistant", "content": []},
            {"type": "message", "role": "user", "content": [{"type": "output_text", "text": "nope"}]},
            {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "hello"}]}
          ]
        }"#;
        let completion = extract_responses_completion(json).unwrap();
        assert_eq!(completion.text, "hello");
        assert_eq!(completion.replay_items.len(), 1);
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
    fn merge_detailed_reports_model_ids() {
        let source = vec![
            OcrBlock {
                id: 1,
                text: "A".into(),
                confidence: 1.0,
                bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
                source_lines: 1,
            },
            OcrBlock {
                id: 2,
                text: "B".into(),
                confidence: 1.0,
                bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
                source_lines: 1,
            },
        ];
        let json = r#"{"blocks":[{"id":1,"translation":"T"}]}"#;
        let out = merge_translations_detailed(&source, json).unwrap();
        assert!(out.model_ids.contains(&1));
        assert!(!out.model_ids.contains(&2));
        assert_eq!(out.blocks[1].translation, "B");
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
        assert!(!TranslateError::CliNotFound("grok".into()).is_retryable());
        assert!(TranslateError::CliExit("closed stdout".into()).is_retryable());
        assert!(TranslateError::CliProtocol("CLI turn timed out".into()).is_retryable());
        assert!(!TranslateError::CliProtocol("Grok session invoked a tool".into()).is_retryable());
    }

    #[test]
    fn should_retry_api_status_until_budget() {
        let err = TranslateError::ApiStatus {
            status: 429,
            body: "rate limited".into(),
        };
        assert!(should_retry(&err, 0, 2));
        assert!(should_retry(&err, 1, 2));
        assert!(!should_retry(&err, 2, 2));
        assert!(!should_retry(&TranslateError::Cancelled, 0, 2));
        assert!(!should_retry(&TranslateError::MissingApiKey, 0, 2));
    }

    #[test]
    fn retry_hook_invoked_for_retryable_api_error() {
        let err = TranslateError::ApiStatus {
            status: 503,
            body: "unavailable".into(),
        };
        let mut notices = Vec::new();
        let max_retries = 2u32;
        let mut attempt = 0u32;
        while should_retry(&err, attempt, max_retries) {
            attempt += 1;
            notices.push((attempt, max_retries, err.to_string()));
        }
        assert_eq!(notices.len(), 2);
        assert_eq!(notices[0], (1, 2, "API returned status 503: unavailable".into()));
        assert_eq!(notices[1].0, 2);
    }
}
