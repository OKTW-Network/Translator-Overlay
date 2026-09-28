//! Claude Code CLI client (`claude -p --input-format stream-json --output-format stream-json`).
//!
//! Uses the user's own installed and logged-in Claude Code. This module never reads, stores,
//! refreshes or forwards Claude credentials: no `--bare` (it skips OAuth), no `CLAUDE_CONFIG_DIR`
//! override, no auth env vars. Claude Code manages its login by itself.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin},
    sync::mpsc,
    time::timeout,
};
use tokio_util::sync::CancellationToken;

use crate::{
    TranslateError,
    cli::{
        TempCwd,
        rpc::{StderrTail, StdioChild, map_auth_failure, spawn_stdio},
    },
};

const AUTH_HINT: &str = "Claude Code is not logged in. Run `claude auth login` in a terminal.";
const SYSTEM_PROMPT_FILE: &str = "system-prompt.md";
const SETTINGS_FILE: &str = "settings.json";
/// Only used when reasoning effort asks for no thinking.
const NO_THINKING_SETTINGS: &str = r#"{"alwaysThinkingEnabled":false}"#;
const EFFORT_LEVELS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
/// `system/api_retry` categories that another attempt will not fix.
const FATAL_RETRY_ERRORS: [&str; 6] = [
    "authentication_failed",
    "oauth_org_not_allowed",
    "account_on_hold",
    "billing_error",
    "model_not_found",
    "invalid_request",
];
/// Used by the model picker when `initialize` advertises nothing.
const FALLBACK_MODELS: [&str; 4] = ["sonnet", "haiku", "opus", "fable"];
const INIT_REQUEST_ID: &str = "translator-overlay-init";

/// `--settings` file contents for a reasoning effort, when the effort turns thinking off.
fn settings_for_effort(effort: Option<&str>) -> Option<&'static str> {
    let effort = effort.map(str::trim)?;
    matches!(effort, "none" | "minimal" | "off").then_some(NO_THINKING_SETTINGS)
}

pub fn spawn_args(model: &str, effort: Option<&str>, cwd: &Path) -> Vec<String> {
    let mut args: Vec<String> = [
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--no-session-persistence",
        "--tools",
        "",
        "--strict-mcp-config",
        "--disable-slash-commands",
        "--safe-mode",
        "--permission-mode",
        "dontAsk",
        "--permission-prompts",
        "none",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    args.push("--system-prompt-file".into());
    args.push(cwd.join(SYSTEM_PROMPT_FILE).to_string_lossy().into_owned());
    if !model.trim().is_empty() {
        args.push("--model".into());
        args.push(model.trim().into());
    }
    match effort.map(str::trim).filter(|s| !s.is_empty()) {
        Some(e) if EFFORT_LEVELS.contains(&e) => {
            args.push("--effort".into());
            args.push(e.into());
        }
        Some(_) if settings_for_effort(effort).is_some() => {
            args.push("--settings".into());
            args.push(cwd.join(SETTINGS_FILE).to_string_lossy().into_owned());
        }
        Some(e) => tracing::warn!(effort = e, "Claude Code does not support this reasoning effort; using its default"),
        None => {}
    }
    args
}

pub fn user_line(text: &str) -> String {
    serde_json::json!({
        "type": "user",
        "message": { "role": "user", "content": [{ "type": "text", "text": text }] },
        "parent_tool_use_id": null,
        "session_id": ""
    })
    .to_string()
}

#[derive(Debug, PartialEq, Eq)]
enum TurnStep {
    Pending,
    /// Turn finished; carries the `result` text as a fallback when no deltas streamed.
    Done(String),
    Failed(String),
}

fn apply_event(event: &Value, text: &mut String, saw_tool: &mut bool) -> TurnStep {
    let kind = event.get("type").and_then(Value::as_str).unwrap_or("");
    let subtype = event.get("subtype").and_then(Value::as_str).unwrap_or("");
    match kind {
        "stream_event" => {
            let Some(inner) = event.get("event") else {
                return TurnStep::Pending;
            };
            match inner.get("type").and_then(Value::as_str).unwrap_or("") {
                "content_block_delta" if inner.pointer("/delta/type").and_then(Value::as_str) == Some("text_delta") => {
                    if let Some(chunk) = inner.pointer("/delta/text").and_then(Value::as_str) {
                        text.push_str(chunk);
                    }
                }
                "content_block_start" if is_tool_block(inner.get("content_block")) => *saw_tool = true,
                _ => {}
            }
            TurnStep::Pending
        }
        "assistant" => {
            if let Some(blocks) = event.pointer("/message/content").and_then(Value::as_array)
                && blocks.iter().any(|b| is_tool_block(Some(b)))
            {
                *saw_tool = true;
            }
            TurnStep::Pending
        }
        "system" if subtype == "init" => {
            let session_id = event.get("session_id").and_then(Value::as_str).unwrap_or("");
            let model = event.get("model").and_then(Value::as_str).unwrap_or("");
            tracing::debug!(session_id, model, "Claude Code session started");
            TurnStep::Pending
        }
        "system" if subtype == "api_retry" => {
            let error = event.get("error").and_then(Value::as_str).unwrap_or("unknown");
            if FATAL_RETRY_ERRORS.contains(&error) {
                return TurnStep::Failed(format!("Claude Code API error: {error}"));
            }
            let attempt = event.get("attempt").and_then(Value::as_u64);
            let status = event.get("error_status").and_then(Value::as_u64);
            tracing::warn!(error, ?attempt, ?status, "Claude Code retrying API request");
            TurnStep::Pending
        }
        "result" => {
            let usage = event.get("usage");
            let tokens = |key: &str| usage.and_then(|u| u.get(key)).and_then(Value::as_u64).unwrap_or(0);
            let cost_usd = event.get("total_cost_usd").and_then(Value::as_f64).unwrap_or(0.0);
            tracing::debug!(
                input = tokens("input_tokens"),
                cache_read = tokens("cache_read_input_tokens"),
                cache_write = tokens("cache_creation_input_tokens"),
                output = tokens("output_tokens"),
                cost_usd,
                "Claude Code turn finished"
            );
            let result = event.get("result").and_then(Value::as_str).unwrap_or("").to_string();
            let is_error = event.get("is_error").and_then(Value::as_bool).unwrap_or(false);
            if !is_error && subtype == "success" {
                return TurnStep::Done(result);
            }
            let detail = event
                .get("errors")
                .and_then(Value::as_array)
                .map(|errs| errs.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("; "))
                .unwrap_or_default();
            let message = [result.as_str(), detail.as_str(), subtype]
                .into_iter()
                .find(|s| !s.trim().is_empty())
                .unwrap_or("error");
            TurnStep::Failed(format!("Claude Code turn failed: {message}"))
        }
        _ => TurnStep::Pending,
    }
}

fn is_tool_block(block: Option<&Value>) -> bool {
    block
        .and_then(|b| b.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|t| t.ends_with("tool_use"))
}

fn models_from_initialize(response: &Value) -> Vec<String> {
    let Some(models) = response.get("models").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = Vec::new();
    for id in models.iter().filter_map(|m| m.get("value").and_then(Value::as_str)).map(str::trim) {
        // "default" means "no --model"; the empty model field already does that.
        if !id.is_empty() && id != "default" && !ids.iter().any(|seen| seen == id) {
            ids.push(id.to_string());
        }
    }
    ids
}

pub struct ClaudeSession {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: mpsc::UnboundedReceiver<Value>,
    stderr_tail: StderrTail,
}

impl ClaudeSession {
    /// Spawn the long-lived process. Claude Code emits `system/init` with the first turn, so no handshake.
    pub fn connect(program: &Path, model: &str, effort: Option<&str>, cwd: &Path, system: &str) -> Result<Self, TranslateError> {
        std::fs::write(cwd.join(SYSTEM_PROMPT_FILE), system)
            .map_err(|e| TranslateError::CliProtocol(format!("write isolated {SYSTEM_PROMPT_FILE}: {e}")))?;
        if let Some(settings) = settings_for_effort(effort) {
            std::fs::write(cwd.join(SETTINGS_FILE), settings)
                .map_err(|e| TranslateError::CliProtocol(format!("write isolated {SETTINGS_FILE}: {e}")))?;
        }
        let args = spawn_args(model, effort, cwd);
        let StdioChild {
            child,
            stdin,
            stdout,
            stderr_tail,
        } = spawn_stdio(program, &args, cwd, &[("DISABLE_AUTOUPDATER", "1")])?;

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(line) {
                    Ok(value) => {
                        if tx.send(value).is_err() {
                            break;
                        }
                    }
                    Err(e) => tracing::warn!(target: "translator_translate::cli", error = %e, "ignored non-JSON Claude Code stdout line"),
                }
            }
        });

        Ok(Self {
            child,
            stdin: Some(stdin),
            rx,
            stderr_tail,
        })
    }

    pub async fn prompt(
        &mut self,
        user: &str,
        cancel: &CancellationToken,
        timeout: Duration,
        on_text: &mut impl FnMut(&str),
    ) -> Result<String, TranslateError> {
        self.write_line(&user_line(user)).await?;
        let mut text = String::new();
        let mut saw_tool = false;
        loop {
            let event = self.next_event(cancel, timeout).await?;
            let before = text.len();
            let step = apply_event(&event, &mut text, &mut saw_tool);
            if saw_tool {
                return Err(TranslateError::CliProtocol("Claude Code session invoked a tool".into()));
            }
            match step {
                TurnStep::Pending => {
                    if text.len() != before {
                        on_text(&text);
                    }
                }
                TurnStep::Done(result) => {
                    if text.trim().is_empty() {
                        text = result;
                    }
                    if text.trim().is_empty() {
                        return Err(TranslateError::CliProtocol("Claude Code produced no assistant text".into()));
                    }
                    return Ok(text);
                }
                TurnStep::Failed(message) => return Err(map_auth_failure(TranslateError::CliProtocol(message), AUTH_HINT)),
            }
        }
    }

    /// Model ids from the `initialize` control request. Makes no model call.
    async fn models(&mut self, cancel: &CancellationToken, timeout: Duration) -> Result<Vec<String>, TranslateError> {
        let request = serde_json::json!({
            "type": "control_request",
            "request_id": INIT_REQUEST_ID,
            "request": { "subtype": "initialize" }
        });
        self.write_line(&request.to_string()).await?;
        loop {
            let event = self.next_event(cancel, timeout).await?;
            if event.get("type").and_then(Value::as_str) != Some("control_response")
                || event.pointer("/response/request_id").and_then(Value::as_str) != Some(INIT_REQUEST_ID)
            {
                continue;
            }
            if event.pointer("/response/subtype").and_then(Value::as_str) != Some("success") {
                let error = event.pointer("/response/error").and_then(Value::as_str).unwrap_or("unknown error");
                return Err(TranslateError::CliProtocol(format!("Claude Code initialize failed: {error}")));
            }
            // Only `models` is read; the rest of the response (account details) is ignored.
            return Ok(event.pointer("/response/response").map(models_from_initialize).unwrap_or_default());
        }
    }

    async fn write_line(&mut self, line: &str) -> Result<(), TranslateError> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(TranslateError::CliProtocol("Claude Code stdin closed".into()));
        };
        let mut buf = line.to_string();
        buf.push('\n');
        let written = async {
            stdin.write_all(buf.as_bytes()).await?;
            stdin.flush().await
        }
        .await;
        written.map_err(|e| TranslateError::CliExit(self.with_stderr(format!("write stdin: {e}"))))
    }

    async fn next_event(&mut self, cancel: &CancellationToken, idle: Duration) -> Result<Value, TranslateError> {
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(TranslateError::Cancelled),
            () = tokio::time::sleep(idle) => Err(TranslateError::CliProtocol("CLI turn timed out".into())),
            msg = self.rx.recv() => match msg {
                Some(v) => Ok(v),
                None => Err(TranslateError::CliExit(self.with_stderr("CLI closed stdout".into()))),
            },
        }
    }

    fn with_stderr(&self, message: String) -> String {
        let tail = self.stderr_tail.joined();
        if tail.is_empty() { message } else { format!("{message}: {tail}") }
    }

    /// The turn cannot be resumed cleanly; `CliBackend` recreates the session next time.
    pub fn cancel_turn(&mut self) {
        let _ = self.child.start_kill();
    }

    /// Close stdin so Claude Code exits on its own, then kill if it lingers.
    pub async fn close(&mut self) {
        self.stdin = None;
        if matches!(timeout(Duration::from_secs(2), self.child.wait()).await, Ok(Ok(_))) {
            return;
        }
        let _ = self.child.start_kill();
        let _ = timeout(Duration::from_secs(2), self.child.wait()).await;
    }

    pub fn kill(&mut self) {
        self.stdin = None;
        let _ = self.child.start_kill();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) | Err(_) => return,
                Ok(None) if Instant::now() >= deadline => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    }
}

pub async fn list_models(program: &Path, cancel: &CancellationToken, timeout: Duration) -> Result<Vec<String>, TranslateError> {
    let cwd = TempCwd::create()?;
    let mut session = ClaudeSession::connect(program, "", None, cwd.path(), "You are a translation engine.")?;
    let result = session.models(cancel, timeout).await;
    session.close().await;
    let ids = result?;
    Ok(if ids.is_empty() {
        FALLBACK_MODELS.iter().map(|s| (*s).to_string()).collect()
    } else {
        ids
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn delta(text: &str) -> Value {
        serde_json::json!({
            "type": "stream_event",
            "event": { "type": "content_block_delta", "index": 1, "delta": { "type": "text_delta", "text": text } }
        })
    }

    #[test]
    fn spawn_args_are_locked_down_and_never_bypass_auth_or_permissions() {
        let args = spawn_args("sonnet", Some("low"), Path::new("C:/tmp/iso"));
        for flag in [
            "-p",
            "--no-session-persistence",
            "--strict-mcp-config",
            "--disable-slash-commands",
            "--safe-mode",
            "--include-partial-messages",
        ] {
            assert!(args.iter().any(|a| a == flag), "missing {flag}");
        }
        assert!(args.windows(2).any(|w| w[0] == "--tools" && w[1].is_empty()));
        assert!(args.windows(2).any(|w| w[0] == "--input-format" && w[1] == "stream-json"));
        assert!(args.windows(2).any(|w| w[0] == "--permission-mode" && w[1] == "dontAsk"));
        assert!(args.windows(2).any(|w| w[0] == "--model" && w[1] == "sonnet"));
        assert!(args.windows(2).any(|w| w[0] == "--effort" && w[1] == "low"));
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--system-prompt-file" && w[1].ends_with(SYSTEM_PROMPT_FILE))
        );
        for banned in [
            "--bare",
            "--dangerously-skip-permissions",
            "--allow-dangerously-skip-permissions",
            "bypassPermissions",
        ] {
            assert!(args.iter().all(|a| a != banned), "must not pass {banned}");
        }
    }

    #[test]
    fn effort_maps_to_effort_flag_thinking_off_or_nothing() {
        let cwd = Path::new("C:/tmp/iso");
        let args = spawn_args("", Some("none"), cwd);
        assert!(args.windows(2).any(|w| w[0] == "--settings" && w[1].ends_with(SETTINGS_FILE)));
        assert!(args.iter().all(|a| a != "--effort"));
        assert_eq!(settings_for_effort(Some("minimal")), Some(NO_THINKING_SETTINGS));
        assert_eq!(settings_for_effort(Some("high")), None);

        let args = spawn_args("", Some("bogus"), cwd);
        assert!(args.iter().all(|a| a != "--effort" && a != "--settings"));
        let args = spawn_args("  ", None, cwd);
        assert!(args.iter().all(|a| a != "--model" && a != "--effort" && a != "--settings"));
    }

    #[test]
    fn user_line_is_one_stream_json_user_message() {
        let line = user_line("line1\nline2");
        assert!(!line.contains('\n'));
        let v: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["type"], "user");
        assert_eq!(v["message"]["role"], "user");
        assert_eq!(v["message"]["content"][0]["type"], "text");
        assert_eq!(v["message"]["content"][0]["text"], "line1\nline2");
    }

    #[test]
    fn accumulates_text_deltas_and_ignores_thinking() {
        let mut text = String::new();
        let mut saw_tool = false;
        let thinking = serde_json::json!({
            "type": "stream_event",
            "event": { "type": "content_block_delta", "index": 0, "delta": { "type": "thinking_delta", "thinking": "hmm" } }
        });
        assert_eq!(apply_event(&thinking, &mut text, &mut saw_tool), TurnStep::Pending);
        assert_eq!(apply_event(&delta("{\"b\":[[0,\""), &mut text, &mut saw_tool), TurnStep::Pending);
        assert_eq!(apply_event(&delta("你好\"]]}"), &mut text, &mut saw_tool), TurnStep::Pending);
        let result = serde_json::json!({
            "type": "result", "subtype": "success", "is_error": false, "result": "{\"b\":[[0,\"你好\"]]}",
            "usage": { "input_tokens": 10, "cache_read_input_tokens": 5 }
        });
        assert_eq!(apply_event(&result, &mut text, &mut saw_tool), TurnStep::Done("{\"b\":[[0,\"你好\"]]}".into()));
        assert_eq!(text, "{\"b\":[[0,\"你好\"]]}");
        assert!(!saw_tool);
    }

    #[test]
    fn flags_tool_use_from_stream_and_assistant_message() {
        let mut text = String::new();
        let mut saw_tool = false;
        let start = serde_json::json!({
            "type": "stream_event",
            "event": { "type": "content_block_start", "index": 0, "content_block": { "type": "tool_use", "name": "Bash" } }
        });
        apply_event(&start, &mut text, &mut saw_tool);
        assert!(saw_tool);

        let mut saw_tool = false;
        let assistant = serde_json::json!({
            "type": "assistant",
            "message": { "content": [{ "type": "server_tool_use", "name": "web_search" }] }
        });
        apply_event(&assistant, &mut text, &mut saw_tool);
        assert!(saw_tool);
    }

    #[test]
    fn error_result_maps_to_login_hint() {
        let mut text = String::new();
        let mut saw_tool = false;
        let result = serde_json::json!({
            "type": "result", "subtype": "success", "is_error": true,
            "result": "Not logged in · Please run /login"
        });
        let TurnStep::Failed(message) = apply_event(&result, &mut text, &mut saw_tool) else {
            panic!("expected failure");
        };
        let err = map_auth_failure(TranslateError::CliProtocol(message), AUTH_HINT);
        assert!(err.to_string().contains("claude auth login"), "{err}");

        let other = serde_json::json!({ "type": "result", "subtype": "error_max_turns", "is_error": true });
        let TurnStep::Failed(message) = apply_event(&other, &mut text, &mut saw_tool) else {
            panic!("expected failure");
        };
        assert!(message.contains("error_max_turns"));
        assert!(
            !map_auth_failure(TranslateError::CliProtocol(message), AUTH_HINT)
                .to_string()
                .contains("claude auth login")
        );
    }

    #[test]
    fn fatal_api_retry_fails_fast_and_transient_waits() {
        let mut text = String::new();
        let mut saw_tool = false;
        let auth = serde_json::json!({ "type": "system", "subtype": "api_retry", "attempt": 1, "error": "authentication_failed" });
        let TurnStep::Failed(message) = apply_event(&auth, &mut text, &mut saw_tool) else {
            panic!("expected failure");
        };
        assert!(
            map_auth_failure(TranslateError::CliProtocol(message), AUTH_HINT)
                .to_string()
                .contains("claude auth login")
        );

        let overloaded = serde_json::json!({ "type": "system", "subtype": "api_retry", "attempt": 1, "error": "overloaded" });
        assert_eq!(apply_event(&overloaded, &mut text, &mut saw_tool), TurnStep::Pending);
    }

    #[test]
    fn result_text_is_fallback_when_nothing_streamed() {
        let mut text = String::new();
        let mut saw_tool = false;
        let result = serde_json::json!({ "type": "result", "subtype": "success", "is_error": false, "result": "{\"b\":[]}" });
        assert_eq!(apply_event(&result, &mut text, &mut saw_tool), TurnStep::Done("{\"b\":[]}".into()));
        assert!(text.is_empty());
    }

    #[test]
    fn models_from_initialize_skips_default_and_duplicates() {
        let response = serde_json::json!({
            "models": [
                { "value": "default", "resolvedModel": "claude-opus-5-5" },
                { "value": "sonnet" },
                { "value": " haiku " },
                { "value": "sonnet" },
                { "displayName": "no value" }
            ]
        });
        assert_eq!(models_from_initialize(&response), ["sonnet", "haiku"]);
        assert!(models_from_initialize(&serde_json::json!({})).is_empty());
    }

    /// Live end-to-end check against the locally installed, logged-in Claude Code.
    /// `cargo test -p translator-translate -- --ignored claude_cli_smoke --nocapture`
    #[tokio::test]
    #[ignore = "spawns the real Claude Code CLI and uses the user's plan"]
    async fn claude_cli_smoke() {
        use translator_core::{ApiConfig, ModelProvider};

        use crate::{ChatMessage, cli::CliBackend, parse_translation_blocks};

        let api = ApiConfig {
            provider: ModelProvider::ClaudeCli,
            model: std::env::var("CLAUDE_SMOKE_MODEL").unwrap_or_else(|_| "haiku".into()),
            reasoning_effort: Some("none".into()),
            ..ApiConfig::default()
        };
        let models = super::list_models(&crate::cli::resolve_program(&api).unwrap(), &CancellationToken::new(), Duration::from_secs(30))
            .await
            .expect("list models");
        println!("models: {models:?}");
        assert!(models.iter().any(|m| m == "haiku" || m == "sonnet"));

        let system = "You translate game text to Traditional Chinese (zh-TW). The user sends {\"b\":[[id,\"text\"],...]}. \
                      Reply with {\"b\":[[id,\"translation\"],...]} only.";
        let timeout = Duration::from_secs(90);
        let mut backend = CliBackend::new();
        let mut messages = vec![
            ChatMessage {
                role: "system".into(),
                content: system.into(),
                reasoning_content: None,
            },
            ChatMessage::user(r#"{"b":[[0,"Hello, traveler. The gate is closed tonight."],[1,"Come back at dawn."]]}"#),
        ];

        let mut partials = 0usize;
        let t0 = Instant::now();
        let first = backend
            .complete(&api, &messages, &CancellationToken::new(), timeout, 0, &mut |_| partials += 1)
            .await
            .expect("turn 1");
        println!("turn 1 ({:?}, {partials} partials): {first}", t0.elapsed());
        assert!(partials >= 1, "expected streamed partial text");
        assert_eq!(parse_translation_blocks(&first).expect("turn 1 json").len(), 2);
        let pid = session_pid(&backend);

        messages.push(ChatMessage::assistant(&first));
        messages.push(ChatMessage::user(r#"{"b":[[2,"The traveler knocks again."]]}"#));
        assert!(matches!(crate::cli::plan_turn(&backend.mirrored, &messages).unwrap(), crate::cli::SessionPlan::Append { .. }));
        let mut partials = 0usize;
        let t0 = Instant::now();
        let second = backend
            .complete(&api, &messages, &CancellationToken::new(), timeout, 0, &mut |_| partials += 1)
            .await
            .expect("turn 2");
        println!("turn 2 ({:?}, {partials} partials): {second}", t0.elapsed());
        assert_eq!(parse_translation_blocks(&second).expect("turn 2 json").len(), 1);
        assert_eq!(session_pid(&backend), pid, "append-only turn must reuse the same Claude Code process");

        // Cancel mid-stream: the session is dropped and the next turn recreates it with a bootstrap.
        messages.push(ChatMessage::assistant(&second));
        let mut cancelled = messages.clone();
        cancelled.push(ChatMessage::user(
            r#"{"b":[[3,"A long speech follows, full of promises about the harvest, the river, and the old bridge."]]}"#,
        ));
        let cancel = CancellationToken::new();
        let trigger = cancel.clone();
        let err = backend
            .complete(&api, &cancelled, &cancel, timeout, 0, &mut |_| trigger.cancel())
            .await
            .expect_err("turn 3 cancelled");
        assert!(err.is_cancelled(), "{err}");
        assert!(backend.live.is_none());

        messages.push(ChatMessage::user(r#"{"b":[[4,"Goodbye."]]}"#));
        let t0 = Instant::now();
        let fourth = backend
            .complete(&api, &messages, &CancellationToken::new(), timeout, 0, &mut |_| {})
            .await
            .expect("turn 4 after cancel");
        println!("turn 4 after recreate ({:?}): {fourth}", t0.elapsed());
        assert_eq!(parse_translation_blocks(&fourth).expect("turn 4 json").len(), 1);
        assert_ne!(session_pid(&backend), pid);
        backend.close().await;
    }

    fn session_pid(backend: &crate::cli::CliBackend) -> Option<u32> {
        match backend.live.as_ref() {
            Some(crate::cli::LiveSession::Claude(s)) => s.child.id(),
            _ => None,
        }
    }
}
