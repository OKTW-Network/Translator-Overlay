//! Codex owns its native login, storage and refresh. This client never handles credentials.
//! Translation context is isolated through app-server settings, not a replacement CODEX_HOME.

use std::{collections::HashSet, path::Path, time::Duration};

use serde_json::Value;
use tokio_util::sync::CancellationToken;
use translator_core::ServiceTier;

use crate::{
    TranslateError,
    cli::{TempCwd, rpc::JsonRpcChild},
    http::translation_json_schema,
};

// Codex 0.154.0 config schema. Apply before app-server startup as well as thread creation:
// disabling a hook after SessionStart, or only asking the model to ignore it, is too late.
const ISOLATION_CONFIG: &[(&str, &str)] = &[
    ("project_doc_max_bytes", "0"),
    ("skills.include_instructions", "false"),
    ("skills.bundled.enabled", "false"),
    ("features.skip_host_skill_discovery", "true"),
    ("features.skill_search", "false"),
    ("features.skill_mcp_dependency_install", "false"),
    ("features.hooks", "false"),
    ("features.plugins", "false"),
    ("features.remote_plugin", "false"),
    ("features.apps", "false"),
    ("features.memories", "false"),
    ("features.memory_tool", "false"),
    ("memories.use_memories", "false"),
    ("memories.generate_memories", "false"),
    ("features.shell_tool", "false"),
    ("features.shell_snapshot", "false"),
    ("features.unified_exec", "false"),
    ("features.browser_use", "false"),
    ("features.computer_use", "false"),
    ("features.in_app_browser", "false"),
    ("features.code_mode", "false"),
    ("features.js_repl", "false"),
    ("features.image_generation", "false"),
    ("features.multi_agent", "false"),
    ("features.request_permissions_tool", "false"),
    ("features.standalone_web_search", "false"),
    ("features.tool_suggest", "false"),
    ("features.view_image", "false"),
    ("features.workspace_dependencies", "false"),
    ("features.sleep_tool", "false"),
    ("features.goals", "false"),
    ("tools.update_plan.enabled", "false"),
    ("tools.experimental_request_user_input.enabled", "false"),
    ("web_search", "\"disabled\""),
    ("include_apps_instructions", "false"),
    ("include_collaboration_mode_instructions", "false"),
    ("notify", "[]"),
];

pub fn spawn_args() -> Vec<String> {
    let mut args = vec!["app-server".into()];
    for (key, value) in ISOLATION_CONFIG {
        args.extend(["-c".into(), format!("{key}={value}")]);
    }
    args
}

fn isolation_error(reason: &str) -> TranslateError {
    TranslateError::CliProtocol(format!(
        "Cannot verify Codex translation isolation ({reason}); no OCR was sent. Codex 0.154.0+ is required. Global AGENTS.md cannot be suppressed in 0.154.0; this integration will not modify it. AGENTS.override.md is allowed with a warning. Check managed policy for other conflicts."
    ))
}

async fn initialize_app_server(rpc: &mut JsonRpcChild, cancel: &CancellationToken, timeout: Duration) -> Result<(), TranslateError> {
    let initialized = rpc
        .request(
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
    let version = initialized["userAgent"]
        .as_str()
        .and_then(|s| s.split_once('/'))
        .and_then(|(_, s)| s.split_whitespace().next())
        .map(|version| version.split('.').map(str::parse::<u32>).collect::<Result<Vec<_>, _>>());
    if !matches!(version, Some(Ok(ref parts)) if parts.len() == 3 && parts.as_slice() >= [0, 154, 0].as_slice()) {
        return Err(isolation_error("unsupported version"));
    }
    rpc.notify("initialized", serde_json::json!({})).await
}

// Both catalog APIs paginate. A repeated cursor is a protocol error, not an endless loop.
async fn read_list(
    rpc: &mut JsonRpcChild,
    method: &str,
    mut params: Value,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<Vec<Value>, TranslateError> {
    let mut items = Vec::new();
    let mut cursors = HashSet::new();
    loop {
        let page = rpc.request(method, params.clone(), cancel, timeout).await?;
        let data = page["data"].as_array().ok_or_else(|| isolation_error("missing catalog data"))?;
        items.extend(data.iter().cloned());
        match page.get("nextCursor") {
            Some(Value::Null) => return Ok(items),
            Some(Value::String(cursor)) if !cursor.is_empty() && cursors.insert(cursor.clone()) => {
                params["cursor"] = Value::from(cursor.as_str());
            }
            _ => return Err(isolation_error("invalid catalog cursor")),
        }
    }
}

pub async fn list_models(program: &Path, cancel: &CancellationToken, timeout: Duration) -> Result<Vec<String>, TranslateError> {
    let cwd = TempCwd::create()?;
    let mut rpc = JsonRpcChild::spawn(program, &spawn_args(), cwd.path(), &[], false).await?;
    let result = async {
        initialize_app_server(&mut rpc, cancel, timeout).await?;
        let items = read_list(&mut rpc, "model/list", serde_json::json!({ "limit": 100, "includeHidden": false }), cancel, timeout).await?;
        let mut ids = Vec::new();
        for item in items {
            if let Some(id) = item.get("model").or(item.get("id")).and_then(Value::as_str).map(str::trim)
                && !id.is_empty()
                && !ids.iter().any(|s| s == id)
            {
                ids.push(id.to_string());
            }
        }
        if ids.is_empty() {
            Err(TranslateError::CliProtocol("model/list returned no models; check your Codex login in the CLI".into()))
        } else {
            Ok(ids)
        }
    }
    .await;
    rpc.kill_and_wait().await;
    result
}

fn thread_start_params(model: &str, cwd: &Path, system: &str, service_tier: ServiceTier) -> Value {
    let config: serde_json::Map<String, Value> = ISOLATION_CONFIG
        .iter()
        .map(|(key, value)| ((*key).into(), serde_json::from_str(value).expect("constant isolation value")))
        .collect();
    serde_json::json!({
        "model": model,
        "cwd": cwd.to_string_lossy(),
        "sandbox": "read-only",
        "approvalPolicy": "never",
        "ephemeral": true,
        "developerInstructions": "",
        "baseInstructions": system,
        "serviceName": "translator-overlay",
        "serviceTier": if service_tier == ServiceTier::Priority { "fast" } else { "default" },
        "personality": "none",
        "config": config,
    })
}

fn turn_start_params(thread_id: &str, user: &str, effort: Option<&str>) -> Value {
    let mut params = serde_json::json!({
        "threadId": thread_id,
        "input": [{ "type": "text", "text": user }],
        "outputSchema": translation_json_schema(),
        "sandboxPolicy": { "type": "readOnly", "networkAccess": false },
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
        service_tier: ServiceTier,
        cwd: &Path,
        system: &str,
        cancel: &CancellationToken,
        timeout: Duration,
    ) -> Result<Self, TranslateError> {
        let rpc = JsonRpcChild::spawn(program, &spawn_args(), cwd, &[], false).await?;
        Self::from_rpc(rpc, thread_start_params(model, cwd, system, service_tier), cancel, timeout).await
    }

    async fn from_rpc(
        mut rpc: JsonRpcChild,
        mut params: Value,
        cancel: &CancellationToken,
        timeout: Duration,
    ) -> Result<Self, TranslateError> {
        let initialized = async {
            initialize_app_server(&mut rpc, cancel, timeout).await?;
            let servers = read_list(
                &mut rpc,
                "mcpServerStatus/list",
                serde_json::json!({ "limit": 100, "detail": "toolsAndAuthOnly" }),
                cancel,
                timeout,
            )
            .await?;
            let mut disabled = serde_json::Map::new();
            for server in servers {
                let name = server["name"].as_str().ok_or_else(|| isolation_error("missing MCP name"))?;
                disabled.insert(name.to_string(), serde_json::json!({ "enabled": false }));
            }
            // An empty map does NOT clear inherited MCP config. Override every discovered name.
            params["config"]["mcp_servers"] = Value::Object(disabled);
            let created = rpc.request("thread/start", params, cancel, timeout).await?;
            // 0.154.0 unconditionally loads global AGENTS through its home provider.
            // The user explicitly permits override files; they remain in model context.
            let sources = created["instructionSources"]
                .as_array()
                .ok_or_else(|| isolation_error("unknown instruction sources"))?;
            for source in sources {
                let is_override = source
                    .as_str()
                    .and_then(|path| path.rsplit(['/', '\\']).next())
                    .is_some_and(|name| name.eq_ignore_ascii_case("AGENTS.override.md"));
                if !is_override {
                    return Err(isolation_error("unexpected instruction sources"));
                }
                tracing::warn!(
                    "Codex loaded AGENTS.override.md; continuing with user instructions in context (explicit isolation exception)"
                );
            }
            if created.pointer("/sandbox/type").and_then(Value::as_str) != Some("readOnly")
                || created.pointer("/thread/ephemeral").and_then(Value::as_bool) != Some(true)
                || created["approvalPolicy"] != "never"
            {
                return Err(isolation_error("thread permissions or persistence differ"));
            }
            let thread_id = created
                .pointer("/thread/id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| isolation_error("missing thread id"))?
                .to_string();
            let servers = read_list(
                &mut rpc,
                "mcpServerStatus/list",
                serde_json::json!({ "threadId": thread_id, "limit": 100, "detail": "toolsAndAuthOnly" }),
                cancel,
                timeout,
            )
            .await?;
            if servers.iter().any(|server| server["runtimeStatus"] != "disabled") {
                return Err(isolation_error("MCP runtime is not disabled"));
            }
            Ok(thread_id)
        }
        .await;
        let thread_id = match initialized {
            Ok(id) => id,
            Err(error) => {
                rpc.kill_and_wait().await;
                return Err(error);
            }
        };
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
        on_text: &mut impl FnMut(&str),
    ) -> Result<String, TranslateError> {
        self.turn_id = None;
        let mut output = TurnOutput::default();
        let turn_id = &mut self.turn_id;
        let thread_id = &self.thread_id;
        let mut on_note = |method: &str, params: &Value| {
            if output.apply(method, params, thread_id, turn_id) {
                on_text(&output.text);
            }
        };
        let started = self
            .rpc
            .request_with_notes("turn/start", turn_start_params(thread_id, user, effort), cancel, timeout, &mut on_note)
            .await?;
        let started_id = started
            .pointer("/turn/id")
            .and_then(Value::as_str)
            .ok_or_else(|| TranslateError::CliProtocol("turn/start missing turn.id".into()))?;
        if self.turn_id.as_deref().is_some_and(|id| id != started_id) {
            return Err(TranslateError::CliProtocol("Codex turn id mismatch".into()));
        }
        self.turn_id = Some(started_id.to_string());
        if output.completed.is_none() {
            let turn_id = &mut self.turn_id;
            self.rpc
                .wait_notification(
                    cancel,
                    timeout,
                    |method, params| {
                        method == "turn/completed"
                            && params["threadId"] == *thread_id
                            && params.pointer("/turn/id").and_then(Value::as_str) == Some(started_id)
                    },
                    |method, params| {
                        if output.apply(method, params, thread_id, turn_id) {
                            on_text(&output.text);
                        }
                    },
                )
                .await?;
        }
        if output.saw_tool {
            return Err(TranslateError::CliProtocol("Codex session invoked a tool".into()));
        }
        if output.completed.as_deref() != Some("completed") {
            return Err(TranslateError::CliProtocol(format!(
                "Codex turn {} (check Codex CLI login if authentication failed)",
                output.completed.as_deref().unwrap_or("missing status")
            )));
        }
        if output.text.trim().is_empty() {
            return Err(TranslateError::CliProtocol("Codex turn produced no assistant text".into()));
        }
        Ok(output.text)
    }

    pub async fn cancel_turn(&mut self) {
        if let Some(turn_id) = &self.turn_id {
            let _ = self
                .rpc
                .request(
                    "turn/interrupt",
                    serde_json::json!({ "threadId": self.thread_id, "turnId": turn_id }),
                    &CancellationToken::new(),
                    Duration::from_secs(2),
                )
                .await;
        }
    }

    pub async fn close(&mut self) {
        self.rpc.kill_and_wait().await;
    }

    pub fn kill(&mut self) {
        self.rpc.shutdown();
    }
}

#[derive(Default)]
struct TurnOutput {
    item_id: Option<String>,
    text: String,
    completed: Option<String>,
    saw_tool: bool,
}

impl TurnOutput {
    // Used both while awaiting turn/start's response and while streaming afterwards.
    fn apply(&mut self, method: &str, params: &Value, thread_id: &str, turn_id: &mut Option<String>) -> bool {
        if params["threadId"] != thread_id {
            return false;
        }
        let Some(id) = params.get("turnId").or_else(|| params.pointer("/turn/id")).and_then(Value::as_str) else {
            return false;
        };
        if turn_id.as_deref().is_some_and(|current| current != id) {
            return false;
        }
        if turn_id.is_none() {
            *turn_id = Some(id.to_string());
        }
        match method {
            "item/started" | "item/completed" => {
                let item = &params["item"];
                match item["type"].as_str() {
                    Some("agentMessage") if item["phase"] != "commentary" => {
                        let Some(id) = item["id"].as_str() else {
                            return false;
                        };
                        let old = self.text.clone();
                        if self.item_id.as_deref() != Some(id) {
                            self.item_id = Some(id.to_string());
                            self.text.clear();
                        }
                        if let Some(text) = item["text"].as_str() {
                            self.text = text.to_string();
                        }
                        return self.text != old;
                    }
                    Some("agentMessage" | "reasoning" | "plan" | "userMessage" | "contextCompaction") => {}
                    _ => self.saw_tool = true,
                }
            }
            "item/agentMessage/delta" if params["itemId"].as_str() == self.item_id.as_deref() && self.item_id.is_some() => {
                if let Some(delta) = params["delta"].as_str() {
                    self.text.push_str(delta);
                    return !delta.is_empty();
                }
            }
            "turn/completed" => {
                self.completed = Some(
                    params
                        .pointer("/turn/status")
                        .and_then(Value::as_str)
                        .unwrap_or("missing status")
                        .to_string(),
                );
            }
            _ => {}
        }
        false
    }
}

#[cfg(test)]
mod tests;
