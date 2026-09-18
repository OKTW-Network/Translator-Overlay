//! Grok Build ACP client (`grok agent stdio`).

use std::{path::Path, time::Duration};

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    TranslateError,
    cli::{
        TempCwd,
        rpc::{AcpSession, JsonRpcChild, acp_initialize_params, map_auth_failure, models_from_session_new},
    },
};

const AUTH_HINT: &str = "Grok CLI is not authenticated. Run `grok login` or set XAI_API_KEY.";

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

pub async fn connect(
    program: &Path,
    model: &str,
    reasoning_effort: Option<&str>,
    cwd: &Path,
    system: &str,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<(AcpSession, Value), TranslateError> {
    let args = spawn_args(model, reasoning_effort, system);
    let mut rpc = JsonRpcChild::spawn(program, &args, cwd, &[], true).await?;
    let created = async {
        rpc.request("initialize", acp_initialize_params(), cancel, timeout)
            .await
            .map_err(|e| map_auth_failure(e, AUTH_HINT))?;
        rpc.request(
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
        .map_err(|e| map_auth_failure(e, AUTH_HINT))
    }
    .await;
    match created.and_then(|created| AcpSession::id_from(&created).map(|id| (id, created))) {
        Ok((id, created)) => Ok((AcpSession::new(rpc, id, true, None), created)),
        Err(e) => {
            rpc.kill_and_wait().await;
            Err(e)
        }
    }
}

pub async fn list_models(program: &Path, cancel: &CancellationToken, timeout: Duration) -> Result<Vec<String>, TranslateError> {
    let cwd = TempCwd::create()?;
    let (mut session, created) = connect(program, "", None, cwd.path(), "You are a translation engine.", cancel, timeout).await?;
    let ids = models_from_session_new(&created);
    session.close().await;
    if ids.is_empty() {
        return Err(TranslateError::CliProtocol("ACP session advertised no models".into()));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::rpc::is_auth_failure;

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
    fn spawn_args_omit_empty_model() {
        let args = spawn_args("", None, "sys");
        assert!(args.windows(2).all(|w| w[0] != "--model"));
    }

    #[test]
    fn auth_failures_map_to_login_hint() {
        assert!(is_auth_failure("ACP error: auth_required"));
        assert!(is_auth_failure("authentication required"));
        let err = map_auth_failure(TranslateError::CliProtocol("auth_required".into()), AUTH_HINT);
        assert!(err.to_string().contains("grok login"));
        assert!(err.to_string().contains("XAI_API_KEY"));
        let other = map_auth_failure(TranslateError::CliProtocol("session/new missing sessionId".into()), AUTH_HINT);
        assert!(!other.to_string().contains("grok login"));
    }
}
