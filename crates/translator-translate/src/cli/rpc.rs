//! Newline-delimited JSON-RPC over a child process stdio.

use std::{
    collections::HashSet,
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use serde_json::{Map, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::mpsc,
    time::timeout,
};
use tokio_util::sync::CancellationToken;

use crate::TranslateError;

#[derive(Debug, Clone)]
pub enum Incoming {
    Response { id: Value, result: Result<Value, String> },
    ServerRequest { id: Value, method: String, params: Value },
    Notification { method: String, params: Value },
}

pub fn classify_rpc(value: Value) -> Option<Incoming> {
    let obj = value.as_object()?;
    let method = obj.get("method").and_then(Value::as_str).map(str::to_string);
    let id = obj.get("id").cloned();
    let params = obj.get("params").cloned().unwrap_or(Value::Null);

    if let Some(method) = method {
        return Some(if let Some(id) = id {
            Incoming::ServerRequest { id, method, params }
        } else {
            Incoming::Notification { method, params }
        });
    }

    let id = id?;
    if let Some(err) = obj.get("error") {
        let message = err.get("message").and_then(Value::as_str).unwrap_or("JSON-RPC error").to_string();
        return Some(Incoming::Response { id, result: Err(message) });
    }
    Some(Incoming::Response {
        id,
        result: Ok(obj.get("result").cloned().unwrap_or(Value::Null)),
    })
}

pub fn collect_acp_update(method: &str, params: &Value, text: &mut String, saw_tool: &mut bool) {
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

pub fn acp_initialize_params() -> Value {
    serde_json::json!({
        "protocolVersion": 1,
        "clientInfo": { "name": "translator-overlay", "version": env!("CARGO_PKG_VERSION") },
        "clientCapabilities": {
            "fs": { "readTextFile": false, "writeTextFile": false },
            "terminal": false
        }
    })
}

pub fn is_auth_failure(message: &str) -> bool {
    let m = message.to_ascii_lowercase();
    m.contains("auth_required")
        || m.contains("providerautherror")
        || m.contains("authentication")
        || m.contains("unauthor")
        || m.contains("not authenticated")
        || (m.contains("login") && (m.contains("required") || m.contains("needed") || m.contains("run")))
}

pub fn models_from_session_new(value: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    if let Some(opts) = value.get("configOptions").and_then(Value::as_array) {
        for opt in opts {
            let id = opt.get("id").and_then(Value::as_str).unwrap_or("");
            let category = opt.get("category").and_then(Value::as_str).unwrap_or("");
            if id != "model" && category != "model" {
                continue;
            }
            collect_select_values(opt.get("options"), &mut ids, &mut seen);
        }
    }
    ids
}

pub async fn models_after_connect(mut session: AcpSession, created: Value) -> Result<Vec<String>, TranslateError> {
    let ids = models_from_session_new(&created);
    session.close().await;
    if ids.is_empty() {
        return Err(TranslateError::CliProtocol("ACP session advertised no models".into()));
    }
    Ok(ids)
}

fn collect_select_values(options: Option<&Value>, ids: &mut Vec<String>, seen: &mut HashSet<String>) {
    let Some(arr) = options.and_then(Value::as_array) else {
        return;
    };
    for item in arr {
        if let Some(value) = item.get("value").and_then(Value::as_str) {
            push_unique(ids, seen, value);
        }
        if item.get("group").is_some() || item.get("options").is_some() {
            collect_select_values(item.get("options"), ids, seen);
        }
    }
}

fn push_unique(ids: &mut Vec<String>, seen: &mut HashSet<String>, raw: &str) {
    let id = raw.trim();
    if id.is_empty() {
        return;
    }
    if seen.insert(id.to_string()) {
        ids.push(id.to_string());
    }
}

pub fn map_auth_failure(err: TranslateError, hint: &str) -> TranslateError {
    match err {
        TranslateError::CliProtocol(msg) if is_auth_failure(&msg) => TranslateError::CliProtocol(format!("{hint} ({msg})")),
        other => other,
    }
}

pub fn is_permission_method(method: &str) -> bool {
    let m = method.to_ascii_lowercase();
    m.contains("permission") || m.contains("elicitation") || m.contains("requestapproval")
}

pub fn deny_permission_result(method: &str, params: &Value) -> Value {
    let m = method.to_ascii_lowercase();
    if m.contains("commandexecution") {
        // Interrupts the turn instead of letting the agent try another tool.
        return serde_json::json!({ "decision": "cancel" });
    }
    if m.contains("approval") || m.contains("permissions") {
        return serde_json::json!({ "decision": "abort" });
    }
    // ACP: reject the tool but keep the turn alive so the model can answer without executing.
    if let Some(option_id) = reject_option_id(params) {
        return serde_json::json!({
            "outcome": { "outcome": "selected", "optionId": option_id }
        });
    }
    serde_json::json!({ "outcome": { "outcome": "cancelled" } })
}

fn reject_option_id(params: &Value) -> Option<&str> {
    let options = params.get("options")?.as_array()?;
    let id_for = |kind: &str| {
        options.iter().find_map(|option| {
            (option.get("kind").and_then(Value::as_str) == Some(kind))
                .then(|| option.get("optionId").and_then(Value::as_str))
                .flatten()
        })
    };
    id_for("reject_once").or_else(|| id_for("reject_always"))
}

pub struct JsonRpcChild {
    child: Child,
    stdin: ChildStdin,
    rx: mpsc::UnboundedReceiver<Incoming>,
    next_id: i64,
    /// Grok ACP includes `"jsonrpc":"2.0"`; Codex app-server omits it.
    include_jsonrpc: bool,
}

impl JsonRpcChild {
    pub async fn spawn(
        program: &Path,
        args: &[String],
        cwd: &Path,
        extra_env: &[(&str, &str)],
        include_jsonrpc: bool,
    ) -> Result<Self, TranslateError> {
        let mut cmd = Command::new(program);
        cmd.args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .env("GROK_DISABLE_AUTOUPDATER", "1");
        for (k, v) in extra_env {
            cmd.env(k, v);
        }
        apply_no_window(&mut cmd);

        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                TranslateError::CliNotFound(program.display().to_string())
            } else {
                TranslateError::CliProtocol(format!("spawn {}: {e}", program.display()))
            }
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TranslateError::CliProtocol("child stdin missing".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TranslateError::CliProtocol("child stdout missing".into()))?;
        let stderr = child.stderr.take();

        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.trim().is_empty() {
                        tracing::debug!(target: "translator_translate::cli", "{line}");
                    }
                }
            });
        }

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
                        if let Some(incoming) = classify_rpc(value)
                            && tx.send(incoming).is_err()
                        {
                            break;
                        }
                    }
                    Err(e) => tracing::warn!(target: "translator_translate::cli", error = %e, "ignored non-JSON stdout line"),
                }
            }
        });

        Ok(Self {
            child,
            stdin,
            rx,
            next_id: 1,
            include_jsonrpc,
        })
    }

    pub async fn request(
        &mut self,
        method: &str,
        params: Value,
        cancel: &CancellationToken,
        timeout: Duration,
    ) -> Result<Value, TranslateError> {
        self.request_with_notes(method, params, cancel, timeout, |_, _| {}).await
    }

    pub async fn request_with_notes(
        &mut self,
        method: &str,
        params: Value,
        cancel: &CancellationToken,
        timeout: Duration,
        mut on_note: impl FnMut(&str, &Value),
    ) -> Result<Value, TranslateError> {
        let id = Value::from(self.next_id);
        self.next_id += 1;
        self.write_message(rpc_request(self.include_jsonrpc, id.clone(), method, params))
            .await?;
        loop {
            match self.next_incoming(cancel, timeout).await? {
                Incoming::Response { id: rid, result } if rid == id => {
                    return result.map_err(TranslateError::CliProtocol);
                }
                Incoming::Notification { method, params } => on_note(&method, &params),
                other => self.auto_handle(other).await?,
            }
        }
    }

    pub async fn notify(&mut self, method: &str, params: Value) -> Result<(), TranslateError> {
        self.write_message(rpc_notification(self.include_jsonrpc, method, params)).await
    }

    pub async fn wait_notification(
        &mut self,
        cancel: &CancellationToken,
        timeout: Duration,
        mut pred: impl FnMut(&str, &Value) -> bool,
        mut on_note: impl FnMut(&str, &Value),
    ) -> Result<Value, TranslateError> {
        loop {
            match self.next_incoming(cancel, timeout).await? {
                Incoming::Notification { method, params } => {
                    let done = pred(&method, &params);
                    on_note(&method, &params);
                    if done {
                        return Ok(params);
                    }
                }
                other => self.auto_handle(other).await?,
            }
        }
    }

    async fn next_incoming(&mut self, cancel: &CancellationToken, idle: Duration) -> Result<Incoming, TranslateError> {
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(TranslateError::Cancelled),
            () = tokio::time::sleep(idle) => Err(TranslateError::CliProtocol("CLI turn timed out".into())),
            msg = self.rx.recv() => msg.ok_or_else(|| TranslateError::CliExit("CLI closed stdout".into())),
        }
    }

    async fn auto_handle(&mut self, incoming: Incoming) -> Result<(), TranslateError> {
        match incoming {
            Incoming::ServerRequest { id, method, params } => {
                tracing::warn!(method, params = %params, "denying CLI server request");
                let result = deny_permission_result(&method, &params);
                self.write_message(rpc_result(self.include_jsonrpc, id, result)).await
            }
            Incoming::Notification { method, params } => {
                if is_permission_method(&method) {
                    tracing::warn!(method, params = %params, "ignoring CLI permission notification");
                }
                Ok(())
            }
            Incoming::Response { id, result } => {
                tracing::debug!(%id, ?result, "dropped unmatched CLI response");
                Ok(())
            }
        }
    }

    async fn write_message(&mut self, value: Value) -> Result<(), TranslateError> {
        let mut line = serde_json::to_string(&value).map_err(|e| TranslateError::CliProtocol(e.to_string()))?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| TranslateError::CliProtocol(format!("write stdin: {e}")))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| TranslateError::CliProtocol(format!("flush stdin: {e}")))
    }

    pub fn shutdown(&mut self) {
        let _ = self.child.start_kill();
    }

    pub async fn kill_and_wait(&mut self) {
        let _ = self.child.start_kill();
        let _ = timeout(Duration::from_secs(2), self.child.wait()).await;
    }

    /// Wait for a graceful exit; kill only if the child is still alive.
    pub async fn wait_or_kill(&mut self) {
        if matches!(timeout(Duration::from_secs(2), self.child.wait()).await, Ok(Ok(_))) {
            return;
        }
        self.kill_and_wait().await;
    }

    fn kill_and_wait_blocking(&mut self) {
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

impl Drop for JsonRpcChild {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

pub struct AcpSession {
    rpc: JsonRpcChild,
    session_id: String,
    fail_on_tool: bool,
    /// OpenCode: `session/close` detaches; `{bin} session delete {id}` after the child exits.
    cli_session_delete: Option<(PathBuf, PathBuf)>,
}

impl AcpSession {
    pub fn new(rpc: JsonRpcChild, session_id: String, fail_on_tool: bool, cli_session_delete: Option<(PathBuf, PathBuf)>) -> Self {
        Self {
            rpc,
            session_id,
            fail_on_tool,
            cli_session_delete,
        }
    }

    pub fn id_from(created: &Value) -> Result<String, TranslateError> {
        created
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| TranslateError::CliProtocol("session/new missing sessionId".into()))
            .map(str::to_string)
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

        if self.fail_on_tool && saw_tool {
            return Err(TranslateError::CliProtocol("CLI session invoked a tool".into()));
        }
        if text.trim().is_empty() {
            return Err(TranslateError::CliProtocol("CLI session produced no assistant text".into()));
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
        self.rpc.wait_or_kill().await;
        if let Some((p, cwd)) = self.cli_session_delete.take() {
            best_effort_cli_session_delete(&p, &cwd, &self.session_id);
        }
    }

    pub fn kill(&mut self) {
        self.rpc.kill_and_wait_blocking();
        if let Some((p, cwd)) = self.cli_session_delete.take() {
            best_effort_cli_session_delete(&p, &cwd, &self.session_id);
        }
    }

    pub async fn set_config_option(
        &mut self,
        config_id: &str,
        value: &str,
        cancel: &CancellationToken,
        timeout: Duration,
    ) -> Result<(), TranslateError> {
        self.rpc
            .request(
                "session/set_config_option",
                serde_json::json!({
                    "sessionId": self.session_id,
                    "configId": config_id,
                    "value": value
                }),
                cancel,
                timeout,
            )
            .await?;
        Ok(())
    }
}

impl Drop for AcpSession {
    fn drop(&mut self) {
        // session/new may have already persisted; kill() deletes the CLI row.
        self.kill();
    }
}

fn rpc_request(include_jsonrpc: bool, id: Value, method: &str, params: Value) -> Value {
    let mut map = Map::new();
    if include_jsonrpc {
        map.insert("jsonrpc".into(), Value::from("2.0"));
    }
    map.insert("id".into(), id);
    map.insert("method".into(), Value::from(method));
    map.insert("params".into(), params);
    Value::Object(map)
}

fn rpc_notification(include_jsonrpc: bool, method: &str, params: Value) -> Value {
    let mut map = Map::new();
    if include_jsonrpc {
        map.insert("jsonrpc".into(), Value::from("2.0"));
    }
    map.insert("method".into(), Value::from(method));
    map.insert("params".into(), params);
    Value::Object(map)
}

fn rpc_result(include_jsonrpc: bool, id: Value, result: Value) -> Value {
    let mut map = Map::new();
    if include_jsonrpc {
        map.insert("jsonrpc".into(), Value::from("2.0"));
    }
    map.insert("id".into(), id);
    map.insert("result".into(), result);
    Value::Object(map)
}

fn apply_no_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

/// `{program} session delete {session_id}` — OpenCode persists ACP sessions in its DB.
fn best_effort_cli_session_delete(program: &Path, cwd: &Path, session_id: &str) {
    let mut cmd = std::process::Command::new(program);
    cmd.args(["session", "delete", session_id])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = match cmd.spawn() {
        Err(e) => {
            tracing::warn!(error = %e, "CLI session delete failed");
            return;
        }
        Ok(c) => c,
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Err(e) => {
                tracing::warn!(error = %e, "CLI session delete failed");
                return;
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                tracing::warn!("CLI session delete timed out");
                return;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    if !status.success() {
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        tracing::warn!(
            code = ?status.code(),
            stderr = %stderr.trim(),
            "CLI session delete failed"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{codex, grok};

    #[test]
    fn classifies_codex_response_without_jsonrpc() {
        let v = serde_json::json!({"id": 1, "result": {"thread": {"id": "thr_1"}}});
        match classify_rpc(v) {
            Some(Incoming::Response { id, result: Ok(r) }) => {
                assert_eq!(id, Value::from(1));
                assert_eq!(r["thread"]["id"], "thr_1");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn classifies_notification_and_server_request() {
        let note = serde_json::json!({"method": "turn/completed", "params": {"turn": {"status": "ok"}}});
        match classify_rpc(note) {
            Some(Incoming::Notification { method, .. }) => assert_eq!(method, "turn/completed"),
            other => panic!("{other:?}"),
        }
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "session/request_permission",
            "params": {"tool": "bash"}
        });
        match classify_rpc(req) {
            Some(Incoming::ServerRequest { method, id, .. }) => {
                assert_eq!(method, "session/request_permission");
                assert_eq!(id, Value::from(9));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn deny_shapes_are_closed() {
        assert!(is_permission_method("session/request_permission"));
        assert!(is_permission_method("item/permissions/requestApproval"));
        let acp = deny_permission_result("session/request_permission", &Value::Null);
        assert_eq!(acp["outcome"]["outcome"], "cancelled");
        let rejected = deny_permission_result(
            "session/request_permission",
            &serde_json::json!({
                "options": [
                    { "optionId": "allow-once", "name": "Allow once", "kind": "allow_once" },
                    { "optionId": "reject-once", "name": "Reject", "kind": "reject_once" }
                ]
            }),
        );
        assert_eq!(rejected["outcome"]["outcome"], "selected");
        assert_eq!(rejected["outcome"]["optionId"], "reject-once");
        let cmd = deny_permission_result("item/commandExecution/requestApproval", &Value::Null);
        assert_eq!(cmd["decision"], "cancel");
        let patch = deny_permission_result("item/fileChange/requestApproval", &Value::Null);
        assert_eq!(patch["decision"], "abort");
    }

    #[test]
    fn argv_builders_never_yolo() {
        let args = grok::spawn_args("grok-4.5", Some("low"), "sys");
        assert!(args.iter().all(|a| !a.contains("always-approve") && !a.contains("yolo")));
        assert!(args.contains(&"stdio".into()));
        assert!(args.contains(&"--disallowed-tools".into()));
        assert!(args.contains(&"--system-prompt-override".into()));
        let agent = args.iter().position(|a| a == "agent").expect("agent");
        assert_eq!(args.get(agent + 1).map(String::as_str), Some("stdio"));
        let args = codex::spawn_args();
        assert_eq!(args, ["app-server"]);
    }

    #[test]
    fn acp_session_id_from_new() {
        assert_eq!(AcpSession::id_from(&serde_json::json!({"sessionId": "ses_1"})).unwrap(), "ses_1");
        assert!(AcpSession::id_from(&Value::Null).is_err());
    }

    #[test]
    fn models_from_session_new_reads_flat_and_grouped_config_options() {
        let flat = serde_json::json!({
            "sessionId": "s1",
            "configOptions": [
                {
                    "id": "mode",
                    "category": "mode",
                    "options": [{ "value": "ask", "name": "Ask" }]
                },
                {
                    "id": "model",
                    "category": "model",
                    "options": [
                        { "value": "grok-4.6", "name": "Grok 4.6" },
                        { "value": " grok-4.5 ", "name": "Grok 4.5" },
                        { "value": "grok-4.6", "name": "dup" }
                    ]
                }
            ]
        });
        assert_eq!(models_from_session_new(&flat), ["grok-4.6", "grok-4.5"]);

        let grouped = serde_json::json!({
            "configOptions": [{
                "id": "model",
                "options": [{
                    "group": "recommended",
                    "name": "Recommended",
                    "options": [
                        { "value": "opencode/gpt-5", "name": "GPT-5" },
                        { "value": "anthropic/claude-sonnet-4-5", "name": "Sonnet" }
                    ]
                }]
            }]
        });
        assert_eq!(models_from_session_new(&grouped), ["opencode/gpt-5", "anthropic/claude-sonnet-4-5"]);
        assert!(models_from_session_new(&serde_json::json!({"sessionId": "s"})).is_empty());
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
