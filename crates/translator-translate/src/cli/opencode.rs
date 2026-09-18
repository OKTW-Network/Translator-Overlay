//! OpenCode CLI ACP client (`opencode acp`).

use std::{fs, path::Path, time::Duration};

use tokio_util::sync::CancellationToken;

use crate::{
    TranslateError,
    cli::rpc::{AcpSession, JsonRpcChild, acp_initialize_params, map_auth_failure},
};

const AUTH_HINT: &str = "OpenCode CLI is not authenticated. Run `opencode auth login`.";

/// Isolated config must be `ask` (not `deny`) so tools stay advertised and ACP can reject.
const OPENCODE_JSON: &str = r#"{"permission":{"*":"ask"},"agent":{"build":{"permission":"ask"},"plan":{"permission":"ask"},"explore":{"permission":"ask"},"general":{"permission":"ask"}},"experimental":{"continue_loop_on_deny":true}}"#;

pub async fn connect(
    program: &Path,
    model: &str,
    reasoning_effort: Option<&str>,
    cwd: &Path,
    system: &str,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<AcpSession, TranslateError> {
    fs::write(cwd.join("AGENTS.md"), system).map_err(|e| TranslateError::CliProtocol(format!("write isolated AGENTS.md: {e}")))?;
    fs::write(cwd.join("opencode.json"), OPENCODE_JSON)
        .map_err(|e| TranslateError::CliProtocol(format!("write isolated opencode.json: {e}")))?;
    let mut rpc = JsonRpcChild::spawn(program, &["acp".into()], cwd, &[], true).await?;

    rpc.request("initialize", acp_initialize_params(), cancel, timeout)
        .await
        .map_err(|e| map_auth_failure(e, AUTH_HINT))?;

    let created = rpc
        .request(
            "session/new",
            serde_json::json!({
                "cwd": cwd.to_string_lossy(),
                "mcpServers": []
            }),
            cancel,
            timeout,
        )
        .await
        .map_err(|e| map_auth_failure(e, AUTH_HINT))?;

    let session_id = AcpSession::id_from(&created)?;
    let mut session = AcpSession::new(rpc, session_id, false, Some((program.to_path_buf(), cwd.to_path_buf())));

    if !model.trim().is_empty() {
        session.set_config_option("model", model.trim(), cancel, timeout).await?;
    }

    if let Some(effort) = reasoning_effort.map(str::trim).filter(|s| !s.is_empty())
        && let Err(e) = session.set_config_option("effort", effort, cancel, timeout).await
    {
        tracing::warn!(error = %e, "OpenCode effort option rejected; using default");
    }

    Ok(session)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::cli::rpc::is_auth_failure;

    #[test]
    fn auth_failures_map_to_login_hint() {
        assert!(is_auth_failure("ACP error: auth_required"));
        assert!(is_auth_failure("ProviderAuthError"));
        assert!(is_auth_failure("authentication required"));
        let err = map_auth_failure(TranslateError::CliProtocol("auth_required".into()), AUTH_HINT);
        assert!(err.to_string().contains("opencode auth login"));
        let other = map_auth_failure(TranslateError::CliProtocol("session/new missing sessionId".into()), AUTH_HINT);
        assert!(!other.to_string().contains("opencode auth login"));
    }

    #[test]
    fn isolated_config_is_json() {
        let v: Value = serde_json::from_str(OPENCODE_JSON).unwrap();
        assert_eq!(v["permission"]["*"], "ask");
    }
}
