//! Translation client: OpenAI-compatible HTTP or long-lived Grok/Codex CLI sessions.

mod cache;
mod cli;
mod http;

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
use translator_core::{ApiConfig, HttpApi, OcrBlock, TranslatedBlock, TranslationConfig};

pub use crate::{
    cache::{CacheResolve, TranslationCache},
    http::{ChatMessage, ResponseItem},
};
use crate::{
    cli::CliBackend,
    http::{chat_completion_body, responses_request_body},
};

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
    /// Client-generated id sent as `x-grok-conv-id` / `prompt_cache_key`.
    /// Stable for the conversation lifetime; new id only on [`Self::clear`].
    pub session_id: String,
}

impl Conversation {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            session_id: new_session_id(),
        }
    }

    pub fn messages(&self) -> Vec<ChatMessage> {
        self.items
            .iter()
            .filter_map(|item| match item {
                ResponseItem::Message {
                    role,
                    content,
                    reasoning_content,
                } => Some(ChatMessage {
                    role: role.clone(),
                    content: content.clone(),
                    reasoning_content: reasoning_content.clone(),
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
            Some(ResponseItem::Message { role, content, .. }) if role == "system" => *content = system,
            _ => self.items.insert(0, ResponseItem::message("system", system)),
        }
    }

    pub fn push_user(&mut self, content: impl Into<String>) {
        self.items.push(ResponseItem::message("user", content));
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
        self.session_id = new_session_id();
    }

    /// When turn count reaches `max_turns`, keep only the system message plus
    /// the last `history_max_items` user turns (with their reasoning/assistant items).
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
        "Translate {src}→{dst}. Input {{\"b\":[[id,\"source\"],...]}}. \
         Reply JSON only: {{\"b\":[[id,\"translation\"],...]}} matching ids. \
         No markdown fences, no trailing commas; escape quotes and backslashes. \
         Preserve proper nouns when appropriate.",
        src = cfg.source_lang,
        dst = cfg.target_lang
    )
}

/// Serialize OCR blocks as the user message payload.
pub fn user_payload_from_blocks(blocks: &[OcrBlock]) -> String {
    #[derive(Serialize)]
    struct Payload<'a> {
        b: Vec<(u32, &'a str)>,
    }
    serde_json::to_string(&Payload {
        b: blocks.iter().map(|b| (b.id, b.text.as_str())).collect(),
    })
    .unwrap_or_else(|_| "{}".to_string())
}

#[derive(Debug, Deserialize)]
struct TranslationResponse {
    b: Vec<(serde_json::Value, serde_json::Value)>,
}

/// Merge LLM JSON output with original OCR blocks, plus which ids the model actually returned.
pub fn merge_translations_detailed(source: &[OcrBlock], response_json: &str) -> Result<MergeOutcome, TranslateError> {
    let parsed = parse_translation_blocks(response_json)?;
    let model_ids: HashSet<u32> = parsed.iter().map(|(id, _)| *id).collect();

    let mut blocks = Vec::with_capacity(source.len());
    for src in source {
        let translation = parsed
            .iter()
            .find(|(id, _)| *id == src.id)
            .map(|(_, t)| t.clone())
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

fn parse_translation_blocks(response_json: &str) -> Result<Vec<(u32, String)>, TranslateError> {
    let candidates = json_parse_candidates(response_json);
    let mut last_err = String::new();

    for candidate in &candidates {
        match serde_json::from_str::<TranslationResponse>(candidate) {
            Ok(parsed) => {
                let raw_len = parsed.b.len();
                let pairs: Vec<(u32, String)> = parsed
                    .b
                    .into_iter()
                    .filter_map(|(id, text)| {
                        let id = match id {
                            serde_json::Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok())?,
                            serde_json::Value::String(s) => s.parse().ok()?,
                            _ => return None,
                        };
                        let text = match text {
                            serde_json::Value::String(s) => s,
                            serde_json::Value::Number(n) => n.to_string(),
                            _ => return None,
                        };
                        Some((id, text))
                    })
                    .collect();
                if pairs.is_empty() && raw_len != 0 {
                    last_err = "no valid [id, text] pairs".into();
                    continue;
                }
                return Ok(pairs);
            }
            Err(e) => last_err = e.to_string(),
        }
    }

    let snippet = truncate_for_error(response_json.trim(), 240);
    Err(TranslateError::Parse(format!("{last_err}; content snippet: {snippet:?}")))
}

/// Best-effort pairs from a possibly incomplete `{"b":[[id,"text"],...]}` stream.
///
/// Complete tuples are included as soon as the string closes. The last unfinished
/// `[id, "partial` is included so the current block can paint while it grows.
pub fn peek_translation_pairs(partial: &str) -> Vec<(u32, String)> {
    let s = partial.trim().trim_start_matches('\u{feff}');
    let s = strip_markdown_fence(s);
    let Some(inner) = b_array_inner(&s) else {
        return Vec::new();
    };
    parse_pairs_from_array(inner)
}

fn b_array_inner(s: &str) -> Option<&str> {
    let mut from = 0;
    while let Some(rel) = s[from..].find("\"b\"") {
        let key = from + rel;
        from = key + 3;
        let before = s[..key].trim_end();
        if !(before.ends_with('{') || before.ends_with(',')) {
            continue;
        }
        let Some(after) = s[from..].trim_start().strip_prefix(':') else {
            continue;
        };
        if let Some(rest) = after.trim_start().strip_prefix('[') {
            return Some(rest);
        }
    }
    None
}

fn parse_pairs_from_array(mut s: &str) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    loop {
        s = s.trim_start();
        if s.is_empty() || s.starts_with(']') {
            break;
        }
        if let Some(rest) = s.strip_prefix(',') {
            s = rest;
            continue;
        }
        let Some(rest) = s.strip_prefix('[') else {
            break;
        };
        s = rest.trim_start();
        let (id, rest) = if let Some(parsed) = parse_u32_prefix(s) {
            parsed
        } else {
            match parse_json_string_prefix(s) {
                Some((text, Some(rest))) => match text.parse() {
                    Ok(id) => (id, rest),
                    Err(_) => break,
                },
                _ => break,
            }
        };
        s = rest.trim_start();
        let Some(rest) = s.strip_prefix(',') else {
            break;
        };
        s = rest.trim_start();
        let Some((text, rest)) = parse_json_string_prefix(s).or_else(|| parse_json_number_prefix(s)) else {
            break;
        };
        match rest {
            Some(rest) => {
                out.push((id, text));
                s = rest.trim_start();
                if let Some(rest) = s.strip_prefix(']') {
                    s = rest;
                } else {
                    break;
                }
            }
            None => {
                if !text.is_empty() {
                    out.push((id, text));
                }
                break;
            }
        }
    }
    out
}

fn parse_u32_prefix(s: &str) -> Option<(u32, &str)> {
    let n = s.bytes().take_while(u8::is_ascii_digit).count();
    if n == 0 {
        return None;
    }
    let id = s[..n].parse().ok()?;
    Some((id, &s[n..]))
}

/// JSON number token; `None` rest = still growing at end of buffer.
fn parse_json_number_prefix(s: &str) -> Option<(String, Option<&str>)> {
    let body = s.strip_prefix('-').unwrap_or(s);
    let int_digits = body.bytes().take_while(u8::is_ascii_digit).count();
    if int_digits == 0 {
        return None;
    }
    let mut end = (s.len() - body.len()) + int_digits;
    if s.as_bytes().get(end) == Some(&b'.') {
        let frac = s[end + 1..].bytes().take_while(u8::is_ascii_digit).count();
        if frac == 0 {
            return if end + 1 == s.len() {
                Some((s[..end].to_string(), None))
            } else {
                None
            };
        }
        end += 1 + frac;
    }
    if end == s.len() {
        Some((s[..end].to_string(), None))
    } else {
        Some((s[..end].to_string(), Some(&s[end..])))
    }
}

/// `(unescaped, Some(rest))` when the string closed; `None` rest = still open.
fn parse_json_string_prefix(s: &str) -> Option<(String, Option<&str>)> {
    let bytes = s.as_bytes();
    if bytes.first().copied() != Some(b'"') {
        return None;
    }
    let mut i = 1;
    let mut escape = false;
    while i < bytes.len() {
        let b = bytes[i];
        if escape {
            escape = false;
            i += 1;
            continue;
        }
        if b == b'\\' {
            escape = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            let text = serde_json::from_str(&s[..=i]).ok()?;
            return Some((text, Some(&s[i + 1..])));
        }
        i += 1;
    }
    let end = bytes.len() - usize::from(escape);
    Some((s[1..end].to_string(), None))
}

/// Fence-strip plus balanced `{...}` extraction (ignore prose around JSON).
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

    // 2) Balanced `{...}` extraction (ignore prose around JSON).
    if let Some(obj) = extract_balanced(unfenced.as_str(), '{', '}') {
        push(&mut out, obj.to_string());
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

fn truncate_for_error(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let kept: String = s.chars().take(max_chars).collect();
    format!("{kept}…")
}

struct CliHandle {
    backend: tokio::sync::Mutex<CliBackend>,
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
    config: ApiConfig,
    cli: Arc<CliHandle>,
}

impl TranslateClient {
    pub fn new(config: ApiConfig) -> Self {
        Self {
            http: build_http_client(&config),
            config,
            cli: Arc::new(CliHandle {
                backend: tokio::sync::Mutex::new(CliBackend::new()),
                epoch: AtomicU64::new(0),
            }),
        }
    }

    pub fn update_config(&mut self, config: ApiConfig) {
        // Rebuild client so idle timeout reflects the latest config.
        self.http = build_http_client(&config);
        let session_changed = self.config.provider != config.provider
            || self.config.cli_path != config.cli_path
            || self.config.model != config.model
            || self.config.reasoning_effort != config.reasoning_effort
            || self.config.service_tier != config.service_tier
            || self.config.http_api != config.http_api;
        self.config = config;
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

    pub fn config(&self) -> &ApiConfig {
        &self.config
    }

    pub async fn complete_cancellable(
        &self,
        conv: &Conversation,
        cancel: &CancellationToken,
        on_text: &mut impl FnMut(&str),
    ) -> Result<Completion, TranslateError> {
        if !self.config.provider.is_cli() && self.config.http_api == HttpApi::Responses {
            let body = responses_request_body(&self.config, &conv.items, &conv.session_id);
            return self
                .http_complete(
                    &format!("{}/responses", self.config.base_url.trim_end_matches('/')),
                    &body,
                    Some(conv.session_id.as_str()),
                    HttpApi::Responses,
                    cancel,
                    on_text,
                )
                .await;
        }
        let messages = conv.messages();
        if self.config.provider.is_cli() {
            let text = self.cli_complete(&messages, cancel, on_text).await?;
            return Ok(Completion {
                text,
                replay_items: Vec::new(),
            });
        }
        self.http_complete(
            &format!("{}/chat/completions", self.config.base_url.trim_end_matches('/')),
            &chat_completion_body(&self.config, &messages, &conv.session_id),
            Some(conv.session_id.as_str()),
            HttpApi::ChatCompletions,
            cancel,
            on_text,
        )
        .await
    }

    async fn http_complete<T: Serialize>(
        &self,
        url: &str,
        body: &T,
        session_id: Option<&str>,
        http_api: HttpApi,
        cancel: &CancellationToken,
        on_text: &mut impl FnMut(&str),
    ) -> Result<Completion, TranslateError> {
        if self.config.api_key.trim().is_empty() {
            return Err(TranslateError::MissingApiKey);
        }
        if cancel.is_cancelled() {
            return Err(TranslateError::Cancelled);
        }

        let mut req = self.http.post(url).bearer_auth(&self.config.api_key);
        if let Some(session_id) = session_id.filter(|s| !s.is_empty()) {
            req = req.header("x-grok-conv-id", session_id);
        }
        let send = req.json(body).send();

        let response = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(TranslateError::Cancelled),
            result = send => result?,
        };

        crate::http::consume_http_response(response, http_api, self.config.stream, cancel, on_text).await
    }

    async fn cli_complete(
        &self,
        messages: &[ChatMessage],
        cancel: &CancellationToken,
        on_text: &mut impl FnMut(&str),
    ) -> Result<String, TranslateError> {
        if cancel.is_cancelled() {
            return Err(TranslateError::Cancelled);
        }
        let timeout = if self.config.request_timeout_secs == 0 {
            Duration::from_secs(3600)
        } else {
            Duration::from_secs(self.config.request_timeout_secs)
        };
        let epoch = self.cli.epoch.load(Ordering::SeqCst);
        let mut backend = self.cli.backend.lock().await;
        if epoch != self.cli.epoch.load(Ordering::SeqCst) {
            backend.close().await;
        }
        backend.complete(&self.config, messages, cancel, timeout, epoch, on_text).await
    }

    pub async fn complete_with_retry_on(
        &self,
        conv: &Conversation,
        cancel: &CancellationToken,
        mut on_retry: impl FnMut(u32, u32, &TranslateError, u64),
        on_text: &mut impl FnMut(&str),
    ) -> Result<Completion, TranslateError> {
        let max_retries = self.config.max_retries;
        let mut backoff_ms = self.config.retry_backoff_ms.max(50);
        let mut attempt = 0u32;

        loop {
            match self.complete_cancellable(conv, cancel, on_text).await {
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

fn build_http_client(config: &ApiConfig) -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if config.request_timeout_secs > 0 {
        let idle = Duration::from_secs(config.request_timeout_secs);
        builder = builder.connect_timeout(idle).read_timeout(idle);
    }
    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

/// Join translated block texts for UI / history.
pub fn blocks_to_translated_text(blocks: &[TranslatedBlock]) -> String {
    blocks.iter().map(|b| b.translation.as_str()).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use translator_core::{ApiConfig, ModelProvider, Rect, ServiceTier, TranslationConfig};

    use super::*;
    use crate::http::{completion_from_http_body, extract_responses_completion};

    #[test]
    fn request_omits_unset_params() {
        let api = ApiConfig::default();
        let msgs = [ChatMessage::user("x")];
        let body = chat_completion_body(&api, &msgs, "");
        let v = serde_json::to_value(&body).unwrap();
        assert!(v.get("temperature").is_none());
        assert!(v.get("reasoning_effort").is_none());
        assert!(v.get("prompt_cache_key").is_none());
        assert_eq!(v["prompt_cache_retention"], "24h");
        assert_eq!(v["stream"], true);
    }

    #[test]
    fn changing_service_tier_resets_cli_session() {
        let api = ApiConfig {
            provider: ModelProvider::CodexCli,
            ..ApiConfig::default()
        };
        let mut client = TranslateClient::new(api.clone());
        let initial_epoch = client.cli.epoch.load(Ordering::SeqCst);

        let mut updated = api;
        updated.service_tier = ServiceTier::Priority;
        client.update_config(updated);

        assert_eq!(client.cli.epoch.load(Ordering::SeqCst), initial_epoch + 1);
    }

    #[test]
    fn changing_http_api_resets_session() {
        let api = ApiConfig::default();
        let mut client = TranslateClient::new(api.clone());
        let initial_epoch = client.cli.epoch.load(Ordering::SeqCst);

        let mut updated = api;
        updated.http_api = HttpApi::Responses;
        client.update_config(updated);

        assert_eq!(client.cli.epoch.load(Ordering::SeqCst), initial_epoch + 1);
    }

    #[test]
    fn compress_keeps_recent_pairs() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        for i in 0..5 {
            conv.push_user(format!("u{i}"));
            conv.items.push(ResponseItem::message("assistant", format!("a{i}")));
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
        let json = r#"{"b":[[1,"T1"]]}"#;
        let out = merge_translations_detailed(&source, json).unwrap().blocks;
        assert_eq!(out[0].translation, "T1");
    }

    #[test]
    fn clear_resets_conversation() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        conv.push_user("u");
        conv.items.push(ResponseItem::message("assistant", "a"));
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
        conv.items.push(ResponseItem::message("assistant", "a"));
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
            conv.items.push(ResponseItem::message("assistant", format!("a{i}")));
        }
        let id = conv.session_id.clone();
        conv.compress_if_needed(2, 8);
        assert_eq!(conv.session_id, id);
        assert_eq!(conv.turn_count(), 2);
    }

    #[test]
    fn compress_drop_keeps_session_and_reasoning() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        for i in 0..5 {
            conv.push_user(format!("u{i}"));
            conv.items.push(ResponseItem::Output(serde_json::json!({
                "type": "reasoning",
                "encrypted_content": format!("enc{i}"),
            })));
            conv.items.push(ResponseItem::message("assistant", format!("a{i}")));
        }
        let old_session = conv.session_id.clone();
        conv.compress_if_needed(5, 2);
        assert_eq!(conv.session_id, old_session);
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
    fn ensure_system_change_keeps_session_id() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys-a");
        let id = conv.session_id.clone();
        conv.ensure_system("sys-a");
        assert_eq!(conv.session_id, id);
        conv.ensure_system("sys-b");
        assert_eq!(conv.session_id, id);
    }

    #[test]
    fn conversation_reasoning_roundtrip_and_compress() {
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        conv.push_user("u0");
        conv.commit_completion(&Completion {
            text: "{\"b\":[]}".into(),
            replay_items: vec![ResponseItem::Message {
                role: "assistant".into(),
                content: "{\"b\":[]}".into(),
                reasoning_content: Some("thought".into()),
            }],
        });
        assert_eq!(conv.messages()[2].reasoning_content.as_deref(), Some("thought"));
        assert_eq!(
            serde_json::to_value(chat_completion_body(&ApiConfig::default(), &conv.messages(), &conv.session_id)).unwrap()["messages"][2]["reasoning_content"],
            "thought"
        );
        let mut conv = Conversation::empty();
        conv.ensure_system("sys");
        for i in 0..5 {
            conv.push_user(format!("u{i}"));
            conv.items.push(ResponseItem::Message {
                role: "assistant".into(),
                content: format!("a{i}"),
                reasoning_content: Some(format!("think{i}")),
            });
        }
        conv.compress_if_needed(5, 2);
        assert_eq!(conv.messages().into_iter().filter_map(|m| m.reasoning_content).collect::<Vec<_>>(), ["think3", "think4"]);
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
              "content": [{"type": "output_text", "text": "{\"b\":[]}"}]
            }
          ]
        }"#;
        let completion = extract_responses_completion(json).unwrap();
        assert_eq!(completion.text, "{\"b\":[]}");
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
    fn chat_completion_reports_refusal() {
        let json = r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "content": null,
                "refusal": "I can't help with that."
              }
            }
          ]
        }"#;
        let err = completion_from_http_body(HttpApi::ChatCompletions, json).unwrap_err();
        assert!(err.to_string().contains("I can't help with that."), "{err}");
        assert!(!err.is_retryable());
    }

    #[test]
    fn extract_responses_reports_refusal_part() {
        let json = r#"{
          "output": [
            {
              "type": "message",
              "role": "assistant",
              "content": [{"type": "refusal", "refusal": "I can't help with that."}]
            }
          ]
        }"#;
        let err = extract_responses_completion(json).unwrap_err();
        assert!(err.to_string().contains("I can't help with that."), "{err}");
        assert!(!err.is_retryable());
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
        let json = "```json\n{\"b\":[[0,\"T2\"]]}\n```";
        let out = merge_translations_detailed(&source, json).unwrap().blocks;
        assert_eq!(out[0].translation, "T2");
    }

    #[test]
    fn merge_tolerates_prose_but_rejects_trailing_commas() {
        let source = vec![OcrBlock {
            id: 0,
            text: "Hi".into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let json = r#"Here you go: {"b": [[0, "T3"]]} Hope that helps!"#;
        let out = merge_translations_detailed(&source, json).unwrap().blocks;
        assert_eq!(out[0].translation, "T3");

        let json = r#"{"b": [[0, "T3",],]}"#;
        assert!(merge_translations_detailed(&source, json).is_err());
    }

    #[test]
    fn merge_accepts_string_ids_and_numeric_text() {
        let source = vec![OcrBlock {
            id: 2,
            text: "Bye".into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let json = r#"{"b":[["2","T4"]]}"#;
        let out = merge_translations_detailed(&source, json).unwrap().blocks;
        assert_eq!(out[0].translation, "T4");

        let source = vec![OcrBlock {
            id: 1,
            text: "120".into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let json = r#"{"b":[[1,120]]}"#;
        let out = merge_translations_detailed(&source, json).unwrap().blocks;
        assert_eq!(out[0].translation, "120");

        assert!(merge_translations_detailed(&source, r#"{"b":[[true,"T"]]}"#).is_err());
        assert!(merge_translations_detailed(&source, r#"{"b":[["text",1]]}"#).is_err());
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
        let json = r#"{"b":[[1,"T"]]}"#;
        let out = merge_translations_detailed(&source, json).unwrap();
        assert!(out.model_ids.contains(&1));
        assert!(!out.model_ids.contains(&2));
        assert_eq!(out.blocks[1].translation, "B");
    }

    #[test]
    fn merge_rejects_bare_array() {
        let source = vec![OcrBlock {
            id: 1,
            text: "A".into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let json = r#"[[1,"T5"]]"#;
        assert!(merge_translations_detailed(&source, json).is_err());
        let json = r#"{"blocks":[{"id":1,"translation":"T5"}]}"#;
        assert!(merge_translations_detailed(&source, json).is_err());
    }

    #[test]
    fn user_payload_is_compact_tuples() {
        let source = vec![OcrBlock {
            id: 0,
            text: "Hi".into(),
            confidence: 1.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
        }];
        let payload = user_payload_from_blocks(&source);
        assert_eq!(payload, r#"{"b":[[0,"Hi"]]}"#);
        assert!(!payload.contains('\n'));
    }

    #[test]
    fn default_system_prompt_shows_compact_shape() {
        let prompt = default_system_prompt(&TranslationConfig::default());
        assert!(prompt.contains(r#"{"b":[[id,"translation"],...]}"#), "{prompt}");
        assert!(prompt.contains("No markdown fences"), "{prompt}");
        assert!(prompt.contains("no trailing commas"), "{prompt}");
        assert!(!prompt.contains("\"blocks\""));
        assert!(!prompt.contains("\"translation\":"));
    }

    #[test]
    fn peek_translation_pairs_complete_and_partial() {
        assert!(peek_translation_pairs("").is_empty());
        assert!(peek_translation_pairs("{").is_empty());
        assert!(peek_translation_pairs(r#"{"b":["#).is_empty());
        assert_eq!(peek_translation_pairs(r#"{"b":[[0,"A"]]}"#), [(0, "A".into())]);
        assert_eq!(peek_translation_pairs(r#"{"b":[[0,"A"],[1,"B"#), [(0, "A".into()), (1, "B".into())]);
        assert_eq!(peek_translation_pairs(r#"{"b":[[0,"A\"B"]]}"#), [(0, "A\"B".into())]);
        assert_eq!(peek_translation_pairs(r#"{"b":[[0,"A\"#), [(0, "A".into())]);
        assert_eq!(peek_translation_pairs(r#"{"b":[[2,"X"],[2,"Y"]]}"#), [(2, "X".into()), (2, "Y".into())]);
        assert!(peek_translation_pairs(r#"{"b":[[0,"#).is_empty());
        assert_eq!(peek_translation_pairs(r#"{"b":[[12,"x"]"#), [(12, "x".into())]);
        assert_eq!(peek_translation_pairs(r#"The letter "b" is {"b":[[0,"A"]]}"#), [(0, "A".into())]);
        assert_eq!(peek_translation_pairs("{\n  \"b\" : [[0,\"A\"]]\n}"), [(0, "A".into())]);
        assert_eq!(peek_translation_pairs(r#"{"b":[["0","A"]]}"#), [(0, "A".into())]);
        assert_eq!(peek_translation_pairs(r#"{"b":[[1,120]]}"#), [(1, "120".into())]);
        assert_eq!(peek_translation_pairs(r#"{"b":[[1,12"#), [(1, "12".into())]);
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
