//! Codex app-server client (`codex app-server` over stdio).

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::Value;
use tokio_util::sync::CancellationToken;
use translator_core::ServiceTier;

use crate::{
    TranslateError,
    cli::{TempCwd, rpc::JsonRpcChild},
    http::translation_json_schema,
};

const THREAD_SANDBOX_MODE: &str = "read-only";
const TURN_SANDBOX_POLICY_TYPE: &str = "readOnly";

pub fn spawn_args() -> Vec<String> {
    vec!["app-server".into()]
}

async fn initialize_app_server(rpc: &mut JsonRpcChild, cancel: &CancellationToken, timeout: Duration) -> Result<(), TranslateError> {
    rpc.request(
        "initialize",
        serde_json::json!({
            "clientInfo": {
                "name": "translator-overlay",
                "title": "Translator Overlay",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
        cancel,
        timeout,
    )
    .await?;
    rpc.notify("initialized", serde_json::json!({})).await?;
    Ok(())
}

pub(crate) fn models_from_codex_list(value: &Value) -> Vec<String> {
    let Some(data) = value.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    data.iter()
        .filter_map(|item| {
            item.get("id")
                .or(item.get("model"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .collect()
}

pub async fn list_models(program: &Path, cancel: &CancellationToken, timeout: Duration) -> Result<Vec<String>, TranslateError> {
    let cwd = TempCwd::create()?;
    let codex_home = prepare_isolated_codex_home(cwd.path())?;
    let codex_home_text = codex_home.0.to_string_lossy();
    let mut rpc = JsonRpcChild::spawn(program, &spawn_args(), cwd.path(), &[("CODEX_HOME", codex_home_text.as_ref())], false).await?;
    let result = async {
        initialize_app_server(&mut rpc, cancel, timeout).await?;
        let page = rpc
            .request("model/list", serde_json::json!({ "limit": 100, "includeHidden": false }), cancel, timeout)
            .await?;
        let ids = models_from_codex_list(&page);
        if ids.is_empty() {
            Err(TranslateError::CliProtocol("model/list returned no models".into()))
        } else {
            Ok(ids)
        }
    }
    .await;
    rpc.wait_or_kill().await;
    result
}

pub fn thread_start_params(model: &str, cwd: &Path, system: &str, service_tier: ServiceTier) -> Value {
    let mut params = serde_json::json!({
        "model": model,
        "cwd": cwd.to_string_lossy(),
        "sandbox": THREAD_SANDBOX_MODE,
        "approvalPolicy": "never",
        "ephemeral": true,
        "developerInstructions": system,
        "baseInstructions": system,
        "serviceName": "translator-overlay",
        "personality": "none",
        "config": {
            "web_search": "disabled",
            "features.apps": false,
            "features.apply_patch_freeform": false,
            "features.browser_use": false,
            "features.code_mode": false,
            "features.computer_use": false,
            "features.image_generation": false,
            "features.in_app_browser": false,
            "features.js_repl": false,
            "features.memory_tool": false,
            "features.multi_agent": false,
            "features.plugins": false,
            "features.request_permissions_tool": false,
            "features.shell_tool": false,
            "features.standalone_web_search": false,
            "features.tool_search": false,
            "features.tool_suggest": false,
            "features.unified_exec": false,
            "features.view_image": false,
            "features.web_search": false,
            "features.web_search_request": false,
            "features.workspace_dependencies": false,
            "include_apps_instructions": false,
            "include_collaboration_mode_instructions": false,
        },
    });
    if service_tier == ServiceTier::Priority {
        params["config"]["service_tier"] = Value::from("fast");
        params["config"]["features.fast_mode"] = Value::from(true);
    }
    params
}

pub fn turn_start_params(thread_id: &str, user: &str, effort: Option<&str>) -> Value {
    let mut params = serde_json::json!({
        "threadId": thread_id,
        "input": [{ "type": "text", "text": user }],
        "outputSchema": translation_json_schema(),
        "sandboxPolicy": {
            "type": TURN_SANDBOX_POLICY_TYPE,
            "networkAccess": false,
        },
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
    _codex_home: IsolatedCodexHome,
}

impl CodexSession {
    pub async fn connect(
        program: &Path,
        model: &str,
        service_tier: ServiceTier,
        cwd: &Path,
        system: &str,
        cancel: &CancellationToken,
        timeout: Duration,
    ) -> Result<Self, TranslateError> {
        let codex_home = prepare_isolated_codex_home(cwd)?;
        let codex_home_text = codex_home.0.to_string_lossy();
        let args = spawn_args();
        let mut rpc = JsonRpcChild::spawn(program, &args, cwd, &[("CODEX_HOME", codex_home_text.as_ref())], false).await?;
        initialize_app_server(&mut rpc, cancel, timeout).await?;

        let created = rpc
            .request("thread/start", thread_start_params(model, cwd, system, service_tier), cancel, timeout)
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
            _codex_home: codex_home,
        })
    }

    pub async fn prompt(
        &mut self,
        user: &str,
        effort: Option<&str>,
        cancel: &CancellationToken,
        timeout: Duration,
        on_text: &mut impl FnMut(&str),
    ) -> Result<String, TranslateError> {
        let mut text = String::new();
        let mut saw_tool = false;
        let mut completed = None;
        let mut seen_turn_id = None;

        let started = self
            .rpc
            .request_with_notes("turn/start", turn_start_params(&self.thread_id, user, effort), cancel, timeout, |method, params| {
                let before = text.len();
                collect_codex_event(method, params, &mut text, &mut saw_tool);
                if text.len() != before {
                    on_text(&text);
                }
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
                            let before = text.len();
                            collect_codex_event(method, params, &mut text, &mut saw_tool);
                            if text.len() != before {
                                on_text(&text);
                            }
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

struct IsolatedCodexHome(PathBuf);

impl Drop for IsolatedCodexHome {
    fn drop(&mut self) {
        if let Err(e) = crate::cli::remove_dir_all_once(&self.0) {
            tracing::warn!(path = %self.0.display(), %e, "failed to remove isolated Codex home");
        }
    }
}

fn prepare_isolated_codex_home(cwd: &Path) -> Result<IsolatedCodexHome, TranslateError> {
    let codex_home = IsolatedCodexHome(cwd.with_extension("codex-home"));
    std::fs::create_dir_all(&codex_home.0).map_err(|error| TranslateError::CliProtocol(format!("create isolated Codex home: {error}")))?;

    if let Some(source_home) = source_codex_home() {
        let source_auth = source_home.join("auth.json");
        if source_auth.is_file()
            && let Err(error) = std::fs::copy(&source_auth, codex_home.0.join("auth.json"))
        {
            return Err(TranslateError::CliProtocol(format!("copy Codex authentication: {error}")));
        }
    }
    Ok(codex_home)
}

fn source_codex_home() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| {
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(PathBuf::from)
            .map(|home| home.join(".codex"))
    })
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
                "agentMessage" => {
                    if let Some(full) = item.get("text").and_then(Value::as_str)
                        && text.is_empty()
                    {
                        text.push_str(full);
                    }
                }
                kind if is_active_codex_item(kind) => *saw_tool = true,
                _ => {}
            }
        }
        _ => {}
    }
}

fn is_active_codex_item(kind: &str) -> bool {
    !kind.is_empty() && !matches!(kind, "agentMessage" | "reasoning" | "plan" | "userMessage" | "contextCompaction")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn thread_start_is_read_only_and_ephemeral() {
        let params = thread_start_params("gpt-5.6", Path::new("C:/tmp/iso"), "sys", ServiceTier::Standard);
        assert_eq!(params["sandbox"], THREAD_SANDBOX_MODE);
        assert_eq!(params["ephemeral"], true);
        assert_eq!(params["approvalPolicy"], "never");
        assert_eq!(params["developerInstructions"], "sys");
        assert_eq!(params["config"]["web_search"], "disabled");
        assert_eq!(params["config"]["features.shell_tool"], false);
        assert_eq!(params["config"]["features.plugins"], false);
        assert!(params["config"].get("service_tier").is_none());
        assert!(params["config"].get("features.fast_mode").is_none());
        assert_ne!(params["sandbox"], "danger-full-access");
    }

    #[test]
    fn thread_start_enables_fast_service_tier() {
        let params = thread_start_params("gpt-5.6", Path::new("C:/tmp/iso"), "sys", ServiceTier::Priority);
        assert_eq!(params["config"]["service_tier"], "fast");
        assert_eq!(params["config"]["features.fast_mode"], true);
    }

    #[test]
    fn turn_start_sends_only_user_text() {
        let params = turn_start_params("thr_1", "---BEGIN_UNTRUSTED_OCR---\n{}\n---END_UNTRUSTED_OCR---", Some("low"));
        assert_eq!(params["threadId"], "thr_1");
        assert_eq!(params["input"].as_array().map(Vec::len), Some(1));
        assert_eq!(params["input"][0]["type"], "text");
        assert!(params["outputSchema"].is_object());
        assert_eq!(params["effort"], "low");
        assert_eq!(params["sandboxPolicy"]["type"], TURN_SANDBOX_POLICY_TYPE);
        assert_eq!(params["sandboxPolicy"]["networkAccess"], false);
    }

    #[test]
    fn collects_agent_message_without_history() {
        let mut text = String::new();
        let mut saw_tool = false;
        collect_codex_event(
            "item/completed",
            &serde_json::json!({
                "item": { "id": "item_3", "type": "agentMessage", "text": "{\"b\":[]}" }
            }),
            &mut text,
            &mut saw_tool,
        );
        assert_eq!(text, "{\"b\":[]}");
        assert!(!saw_tool);
    }

    #[test]
    fn rejects_current_and_future_active_item_types() {
        for kind in [
            "commandExecution",
            "fileChange",
            "mcpToolCall",
            "dynamicToolCall",
            "collabAgentToolCall",
            "subAgentActivity",
            "webSearch",
            "imageView",
            "sleep",
            "imageGeneration",
            "hookPrompt",
            "futureToolType",
        ] {
            assert!(is_active_codex_item(kind), "missed active item {kind}");
        }
    }

    #[test]
    fn allows_only_passive_conversation_items() {
        for kind in ["agentMessage", "reasoning", "plan", "userMessage", "contextCompaction"] {
            assert!(!is_active_codex_item(kind), "rejected passive item {kind}");
        }
    }

    #[test]
    fn extracts_turn_id_from_result_and_notification() {
        assert_eq!(extract_turn_id(&serde_json::json!({ "turn": { "id": "turn_1" } })).as_deref(), Some("turn_1"));
        assert_eq!(extract_turn_id(&serde_json::json!({ "turnId": "turn_2", "threadId": "thr" })).as_deref(), Some("turn_2"));
        assert_eq!(extract_turn_id(&serde_json::json!({ "threadId": "thr" })), None);
    }

    #[test]
    fn models_from_codex_list_reads_id_or_model() {
        let page = serde_json::json!({
            "data": [
                { "id": "gpt-5.4", "displayName": "GPT-5.4" },
                { "model": " gpt-5.6 " },
                { "id": "" }
            ]
        });
        assert_eq!(models_from_codex_list(&page), ["gpt-5.4", "gpt-5.6"]);
        assert!(models_from_codex_list(&serde_json::json!({ "data": [] })).is_empty());
    }
}
