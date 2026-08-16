//! Newline-delimited JSON-RPC over a child process stdio.

use std::{
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};

use serde_json::{Map, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::mpsc,
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

pub fn is_permission_method(method: &str) -> bool {
    let m = method.to_ascii_lowercase();
    m.contains("permission") || m.contains("elicitation") || m.contains("requestapproval")
}

pub fn deny_permission_result(method: &str) -> Value {
    let m = method.to_ascii_lowercase();
    if m.contains("commandexecution") {
        // Interrupts the turn instead of letting the agent try another tool.
        serde_json::json!({ "decision": "cancel" })
    } else if m.contains("approval") || m.contains("permissions") {
        serde_json::json!({ "decision": "abort" })
    } else {
        serde_json::json!({ "outcome": { "outcome": "cancelled" } })
    }
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
                        if let Some(incoming) = classify_rpc(value) {
                            if tx.send(incoming).is_err() {
                                break;
                            }
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
        let deadline = Instant::now() + timeout;
        loop {
            match self.next_incoming(cancel, deadline).await? {
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
        let deadline = Instant::now() + timeout;
        loop {
            match self.next_incoming(cancel, deadline).await? {
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

    async fn next_incoming(&mut self, cancel: &CancellationToken, deadline: Instant) -> Result<Incoming, TranslateError> {
        let remain = deadline.saturating_duration_since(Instant::now());
        if remain.is_zero() {
            return Err(TranslateError::CliProtocol("CLI turn timed out".into()));
        }
        tokio::select! {
            biased;
            () = cancel.cancelled() => Err(TranslateError::Cancelled),
            () = tokio::time::sleep(remain) => Err(TranslateError::CliProtocol("CLI turn timed out".into())),
            msg = self.rx.recv() => msg.ok_or_else(|| TranslateError::CliExit("CLI closed stdout".into())),
        }
    }

    async fn auto_handle(&mut self, incoming: Incoming) -> Result<(), TranslateError> {
        match incoming {
            Incoming::ServerRequest { id, method, params } => {
                tracing::warn!(method, params = %params, "denying CLI server request");
                let result = deny_permission_result(&method);
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
}

impl Drop for JsonRpcChild {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let acp = deny_permission_result("session/request_permission");
        assert_eq!(acp["outcome"]["outcome"], "cancelled");
        let cmd = deny_permission_result("item/commandExecution/requestApproval");
        assert_eq!(cmd["decision"], "cancel");
        let patch = deny_permission_result("item/fileChange/requestApproval");
        assert_eq!(patch["decision"], "abort");
    }

    #[test]
    fn argv_builders_never_yolo() {
        let args = crate::cli::grok::spawn_args("grok-4.5", Some("low"));
        assert!(args.iter().all(|a| !a.contains("always-approve") && !a.contains("yolo")));
        assert!(args.contains(&"stdio".into()));
        assert!(args.contains(&"--disallowed-tools".into()));
        let args = crate::cli::codex::spawn_args();
        assert_eq!(args, ["app-server"]);
    }
}
