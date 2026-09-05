//! Grok Build ACP client (`grok agent stdio`).

use std::{path::Path, time::Duration};

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{TranslateError, cli::rpc::JsonRpcChild};

pub fn spawn_args(model: &str, reasoning_effort: Option<&str>, system: &str) -> Vec<String> {
    // Global flags first: `grok [flags] agent stdio`. Flags after `agent` are rejected.
    let mut args = vec![
        "--no-subagents".into(),
        "--no-memory".into(),
        "--no-plan".into(),
        "--disable-web-search".into(),
        "--no-auto-update".into(),
        "--disallowed-tools".into(),
        "run_terminal_cmd,search_replace,web_search,web_fetch,read_file,grep,list_dir,Agent".into(),
        "--sandbox".into(),
        "read-only".into(),
        "--system-prompt-override".into(),
        system.to_string(),
    ];
    if !model.trim().is_empty() {
        args.push("--model".into());
        args.push(model.trim().into());
    }
    if let Some(effort) = reasoning_effort.map(str::trim).filter(|s| !s.is_empty()) {
        args.push("--effort".into());
        args.push(effort.into());
    }
    args.push("agent".into());
    args.push("stdio".into());
    args
}

fn auth_required_error(detail: &str) -> TranslateError {
    TranslateError::CliProtocol(format!("Grok CLI is not authenticated. Run `grok login` or set XAI_API_KEY. ({detail})"))
}

fn is_auth_failure(message: &str) -> bool {
    let m = message.to_ascii_lowercase();
    m.contains("auth_required")
        || m.contains("authentication")
        || m.contains("unauthor")
        || m.contains("not authenticated")
        || (m.contains("login") && (m.contains("required") || m.contains("needed") || m.contains("run")))
}

fn map_grok_rpc(err: TranslateError) -> TranslateError {
    match err {
        TranslateError::CliProtocol(msg) if is_auth_failure(&msg) => auth_required_error(&msg),
        other => other,
    }
}

pub struct GrokSession {
    rpc: JsonRpcChild,
    session_id: String,
}

impl GrokSession {
    pub async fn connect(
        program: &Path,
        model: &str,
        reasoning_effort: Option<&str>,
        cwd: &Path,
        system: &str,
        cancel: &CancellationToken,
        timeout: Duration,
    ) -> Result<Self, TranslateError> {
        let args = spawn_args(model, reasoning_effort, system);
        let mut rpc = JsonRpcChild::spawn(program, &args, cwd, &[], true).await?;

        rpc.request(
            "initialize",
            serde_json::json!({
                "protocolVersion": 1,
                "clientInfo": { "name": "translator-overlay", "version": "0.1.0" },
                "clientCapabilities": {
                    "fs": { "readTextFile": false, "writeTextFile": false },
                    "terminal": false
                }
            }),
            cancel,
            timeout,
        )
        .await
        .map_err(map_grok_rpc)?;

        let created = rpc
            .request(
                "session/new",
                serde_json::json!({
                    "cwd": cwd.to_string_lossy(),
                    "mcpServers": [],
                    "_meta": {
                        "systemPromptOverride": system,
                        "yoloMode": false
                    }
                }),
                cancel,
                timeout,
            )
            .await
            .map_err(map_grok_rpc)?;

        let session_id = created
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| TranslateError::CliProtocol("session/new missing sessionId".into()))?
            .to_string();

        Ok(Self { rpc, session_id })
    }

    pub async fn prompt(
        &mut self,
        user: &str,
        cancel: &CancellationToken,
        timeout: Duration,
        on_text: &mut impl FnMut(&str),
    ) -> Result<String, TranslateError> {
        let mut text = String::new();
        let mut saw_tool = false;
        self.rpc
            .request_with_notes(
                "session/prompt",
                serde_json::json!({
                    "sessionId": self.session_id,
                    "prompt": [{ "type": "text", "text": user }]
                }),
                cancel,
                timeout,
                |method, params| {
                    let before = text.len();
                    collect_acp_update(method, params, &mut text, &mut saw_tool);
                    if text.len() != before {
                        on_text(&text);
                    }
                },
            )
            .await?;

        if saw_tool {
            return Err(TranslateError::CliProtocol("Grok session invoked a tool".into()));
        }
        if text.trim().is_empty() {
            return Err(TranslateError::CliProtocol("Grok session produced no assistant text".into()));
        }
        Ok(text)
    }

    pub async fn cancel_turn(&mut self) {
        let _ = self
            .rpc
            .notify("session/cancel", serde_json::json!({ "sessionId": self.session_id }))
            .await;
    }

    pub async fn close(&mut self) {
        let _ = self
            .rpc
            .request(
                "session/close",
                serde_json::json!({ "sessionId": self.session_id }),
                &CancellationToken::new(),
                Duration::from_secs(2),
            )
            .await;
        self.rpc.kill_and_wait().await;
    }

    pub fn kill(&mut self) {
        self.rpc.shutdown();
    }
}

fn collect_acp_update(method: &str, params: &Value, text: &mut String, saw_tool: &mut bool) {
    if method != "session/update" && method != "x.ai/session/update" {
        return;
    }
    let update = params.get("update").unwrap_or(params);
    let kind = update.get("sessionUpdate").and_then(Value::as_str).unwrap_or("");
    match kind {
        "agent_message_chunk" => {
            if let Some(chunk) = update.pointer("/content/text").and_then(Value::as_str) {
                text.push_str(chunk);
            } else if let Some(chunk) = update.get("content").and_then(Value::as_str) {
                text.push_str(chunk);
            }
        }
        "tool_call" | "tool_call_update" => *saw_tool = true,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_args_are_global_then_agent_stdio() {
        let args = spawn_args("grok-4.5", Some("low"), "You are a translation engine.");
        let agent = args.iter().position(|a| a == "agent").expect("agent");
        assert_eq!(args.get(agent + 1).map(String::as_str), Some("stdio"));
        assert!(args[..agent].windows(2).any(|w| w[0] == "--model" && w[1] == "grok-4.5"));
        assert!(args[..agent].windows(2).any(|w| w[0] == "--effort" && w[1] == "low"));
        assert!(
            args[..agent]
                .windows(2)
                .any(|w| w[0] == "--system-prompt-override" && w[1] == "You are a translation engine.")
        );
        assert!(args[agent + 1..].iter().all(|a| !a.starts_with("--")));
    }

    #[test]
    fn auth_failures_map_to_login_hint() {
        assert!(is_auth_failure("ACP error: auth_required"));
        assert!(is_auth_failure("authentication required"));
        let err = map_grok_rpc(TranslateError::CliProtocol("auth_required".into()));
        assert!(err.to_string().contains("grok login"));
        assert!(err.to_string().contains("XAI_API_KEY"));
        let other = map_grok_rpc(TranslateError::CliProtocol("session/new missing sessionId".into()));
        assert!(!other.to_string().contains("grok login"));
    }

    #[test]
    fn session_new_override_is_in_params() {
        let system = "You are a translation engine.";
        let params = serde_json::json!({
            "cwd": "C:/tmp/iso",
            "mcpServers": [],
            "_meta": { "systemPromptOverride": system, "yoloMode": false }
        });
        assert_eq!(params["_meta"]["systemPromptOverride"], system);
        assert_eq!(params["_meta"]["yoloMode"], false);
        assert!(params["mcpServers"].as_array().is_some_and(Vec::is_empty));
    }

    #[test]
    fn collects_agent_chunks_and_flags_tools() {
        let mut text = String::new();
        let mut saw_tool = false;
        collect_acp_update(
            "session/update",
            &serde_json::json!({
                "update": {
                    "sessionUpdate": "agent_message_chunk",
                    "content": { "type": "text", "text": "{\"b\"" }
                }
            }),
            &mut text,
            &mut saw_tool,
        );
        collect_acp_update(
            "session/update",
            &serde_json::json!({
                "update": {
                    "sessionUpdate": "agent_message_chunk",
                    "content": { "type": "text", "text": ":[]}" }
                }
            }),
            &mut text,
            &mut saw_tool,
        );
        assert_eq!(text, "{\"b\":[]}");
        assert!(!saw_tool);
        collect_acp_update(
            "session/update",
            &serde_json::json!({ "update": { "sessionUpdate": "tool_call", "title": "Bash" } }),
            &mut text,
            &mut saw_tool,
        );
        assert!(saw_tool);
    }
}
