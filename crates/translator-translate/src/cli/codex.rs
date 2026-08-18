//! Codex app-server client (`codex app-server` over stdio).

use std::{path::Path, time::Duration};

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    TranslateError,
    cli::{rpc::JsonRpcChild, translation_output_schema},
};

const THREAD_SANDBOX_MODE: &str = "read-only";
const TURN_SANDBOX_POLICY_TYPE: &str = "readOnly";

pub fn spawn_args() -> Vec<String> {
    vec!["app-server".into()]
}

pub fn thread_start_params(model: &str, cwd: &Path, system: &str) -> Value {
    serde_json::json!({
        "model": model,
        "cwd": cwd.to_string_lossy(),
        "sandbox": THREAD_SANDBOX_MODE,
        "approvalPolicy": "untrusted",
        "ephemeral": true,
        "developerInstructions": system,
        "baseInstructions": system,
        "serviceName": "translator-overlay",
        "personality": "none",
    })
}

pub fn turn_start_params(thread_id: &str, user: &str, effort: Option<&str>) -> Value {
    let mut params = serde_json::json!({
        "threadId": thread_id,
        "input": [{ "type": "text", "text": user }],
        "outputSchema": translation_output_schema(),
        "sandboxPolicy": { "type": TURN_SANDBOX_POLICY_TYPE },
    });
    if let Some(effort) = effort.map(str::trim).filter(|s| !s.is_empty()) {
        params["effort"] = Value::from(effort);
    }
    params
}

pub struct CodexSession {
    rpc: JsonRpcChild,
    thread_id: String,
    turn_id: Option<String>,
}

impl CodexSession {
    pub async fn connect(
        program: &Path,
        model: &str,
        cwd: &Path,
        system: &str,
        cancel: &CancellationToken,
        timeout: Duration,
    ) -> Result<Self, TranslateError> {
        let args = spawn_args();
        let mut rpc = JsonRpcChild::spawn(program, &args, cwd, &[], false).await?;

        rpc.request(
            "initialize",
            serde_json::json!({
                "clientInfo": {
                    "name": "translator-overlay",
                    "title": "Translator Overlay",
                    "version": "0.1.0"
                }
            }),
            cancel,
            timeout,
        )
        .await?;
        rpc.notify("initialized", serde_json::json!({})).await?;

        let created = rpc
            .request("thread/start", thread_start_params(model, cwd, system), cancel, timeout)
            .await?;
        let thread_id = created
            .pointer("/thread/id")
            .and_then(Value::as_str)
            .ok_or_else(|| TranslateError::CliProtocol("thread/start missing thread.id".into()))?
            .to_string();

        Ok(Self {
            rpc,
            thread_id,
            turn_id: None,
        })
    }

    pub async fn prompt(
        &mut self,
        user: &str,
        effort: Option<&str>,
        cancel: &CancellationToken,
        timeout: Duration,
    ) -> Result<String, TranslateError> {
        let mut text = String::new();
        let mut saw_tool = false;
        let mut completed = None;
        let mut seen_turn_id = None;

        let started = self
            .rpc
            .request_with_notes("turn/start", turn_start_params(&self.thread_id, user, effort), cancel, timeout, |method, params| {
                collect_codex_event(method, params, &mut text, &mut saw_tool);
                if let Some(id) = extract_turn_id(params) {
                    seen_turn_id = Some(id);
                }
                if method == "turn/completed" {
                    completed = Some(params.clone());
                }
            })
            .await?;

        if let Some(id) = extract_turn_id(&started) {
            seen_turn_id = Some(id);
        }
        self.turn_id = seen_turn_id;

        let completed = match completed {
            Some(params) => params,
            None => {
                let mut later_turn_id = None;
                let params = self
                    .rpc
                    .wait_notification(
                        cancel,
                        timeout,
                        |method, _| method == "turn/completed",
                        |method, params| {
                            collect_codex_event(method, params, &mut text, &mut saw_tool);
                            if let Some(id) = extract_turn_id(params) {
                                later_turn_id = Some(id);
                            }
                        },
                    )
                    .await?;
                if later_turn_id.is_some() {
                    self.turn_id = later_turn_id;
                }
                params
            }
        };

        if saw_tool {
            return Err(TranslateError::CliProtocol("Codex session invoked a tool".into()));
        }

        let status = completed
            .pointer("/turn/status")
            .and_then(Value::as_str)
            .or_else(|| completed.get("status").and_then(Value::as_str))
            .unwrap_or("completed");
        if matches!(status, "interrupted" | "failed" | "error") {
            return Err(TranslateError::CliProtocol(format!("Codex turn {status}")));
        }

        if text.trim().is_empty() {
            return Err(TranslateError::CliProtocol("Codex turn produced no assistant text".into()));
        }
        Ok(text)
    }

    pub async fn cancel_turn(&mut self) {
        let mut params = serde_json::json!({ "threadId": self.thread_id });
        if let Some(turn_id) = &self.turn_id {
            params["turnId"] = Value::from(turn_id.as_str());
        }
        let _ = self
            .rpc
            .request("turn/interrupt", params, &CancellationToken::new(), Duration::from_secs(2))
            .await;
    }

    pub async fn close(&mut self) {
        self.rpc.kill_and_wait().await;
    }

    pub fn kill(&mut self) {
        self.rpc.shutdown();
    }
}

fn extract_turn_id(value: &Value) -> Option<String> {
    value
        .pointer("/turn/id")
        .and_then(Value::as_str)
        .or_else(|| value.get("turnId").and_then(Value::as_str))
        .map(str::to_string)
}

fn collect_codex_event(method: &str, params: &Value, text: &mut String, saw_tool: &mut bool) {
    match method {
        "item/agentMessage/delta" => {
            if let Some(chunk) = params.pointer("/delta/text").and_then(Value::as_str) {
                text.push_str(chunk);
            } else if let Some(chunk) = params.get("delta").and_then(Value::as_str) {
                text.push_str(chunk);
            } else if let Some(chunk) = params.get("text").and_then(Value::as_str) {
                text.push_str(chunk);
            }
        }
        "item/completed" | "item/started" => {
            let item = params.get("item").unwrap_or(params);
            let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
            match kind {
                "agent_message" => {
                    if let Some(full) = item.get("text").and_then(Value::as_str)
                        && text.is_empty()
                    {
                        text.push_str(full);
                    }
                }
                "command_execution" | "file_change" | "mcp_tool_call" | "web_search" => *saw_tool = true,
                _ => {}
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn thread_start_is_read_only_and_ephemeral() {
        let params = thread_start_params("gpt-5.6", Path::new("C:/tmp/iso"), "sys");
        assert_eq!(params["sandbox"], "read-only");
        assert_eq!(params["ephemeral"], true);
        assert_eq!(params["approvalPolicy"], "untrusted");
        assert_eq!(params["developerInstructions"], "sys");
        assert_ne!(params["sandbox"], "danger-full-access");
    }

    #[test]
    fn turn_start_sends_only_user_text() {
        let params = turn_start_params("thr_1", "---BEGIN_UNTRUSTED_OCR---\n{}\n---END_UNTRUSTED_OCR---", Some("low"));
        assert_eq!(params["threadId"], "thr_1");
        assert_eq!(params["input"].as_array().map(Vec::len), Some(1));
        assert_eq!(params["input"][0]["type"], "text");
        assert!(params["outputSchema"].is_object());
        assert_eq!(params["effort"], "low");
        assert_eq!(params["sandboxPolicy"]["type"], "readOnly");
        assert_ne!(params["sandboxPolicy"]["type"], THREAD_SANDBOX_MODE);
    }

    #[test]
    fn collects_agent_message_without_history() {
        let mut text = String::new();
        let mut saw_tool = false;
        collect_codex_event(
            "item/completed",
            &serde_json::json!({
                "item": { "id": "item_3", "type": "agent_message", "text": "{\"blocks\":[]}" }
            }),
            &mut text,
            &mut saw_tool,
        );
        assert_eq!(text, "{\"blocks\":[]}");
        assert!(!saw_tool);
    }

    #[test]
    fn extracts_turn_id_from_result_and_notification() {
        assert_eq!(extract_turn_id(&serde_json::json!({ "turn": { "id": "turn_1" } })).as_deref(), Some("turn_1"));
        assert_eq!(extract_turn_id(&serde_json::json!({ "turnId": "turn_2", "threadId": "thr" })).as_deref(), Some("turn_2"));
        assert_eq!(extract_turn_id(&serde_json::json!({ "threadId": "thr" })), None);
    }
}
