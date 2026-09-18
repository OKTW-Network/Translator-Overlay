//! OpenCode CLI ACP client (`opencode acp`).

use std::{fs, path::Path, time::Duration};

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    TranslateError,
    cli::{
        TempCwd,
        rpc::{AcpSession, JsonRpcChild, acp_initialize_params, map_auth_failure, models_after_connect},
    },
};

const AUTH_HINT: &str = "OpenCode CLI is not authenticated. Run `opencode auth login`.";

/// Isolated config must be `ask` (not `deny`) so tools stay advertised and ACP can reject.
/// Skill is denied so `<available_skills>` is omitted. Build prompt is `TRANSLATE.md`, not `AGENTS.md`.
const OPENCODE_JSON: &str = r#"{"tools":{"skill":false},"permission":{"*":"ask"},"agent":{"build":{"prompt":"{file:./TRANSLATE.md}","permission":{"*":"ask","skill":"deny"}},"plan":{"permission":{"*":"ask","skill":"deny"}},"explore":{"permission":{"*":"ask","skill":"deny"}},"general":{"permission":{"*":"ask","skill":"deny"}}},"experimental":{"continue_loop_on_deny":true}}"#;

pub async fn connect(
    program: &Path,
    model: &str,
    reasoning_effort: Option<&str>,
    cwd: &Path,
    system: &str,
    cancel: &CancellationToken,
    timeout: Duration,
) -> Result<(AcpSession, Value), TranslateError> {
    fs::write(cwd.join("TRANSLATE.md"), system).map_err(|e| TranslateError::CliProtocol(format!("write isolated TRANSLATE.md: {e}")))?;
    fs::write(cwd.join("opencode.json"), OPENCODE_JSON)
        .map_err(|e| TranslateError::CliProtocol(format!("write isolated opencode.json: {e}")))?;
    let mut rpc = JsonRpcChild::spawn(program, &["acp".into()], cwd, &[], true).await?;
    let created = async {
        rpc.request("initialize", acp_initialize_params(), cancel, timeout)
            .await
            .map_err(|e| map_auth_failure(e, AUTH_HINT))?;
        rpc.request(
            "session/new",
            serde_json::json!({
                "cwd": cwd.to_string_lossy(),
                "mcpServers": []
            }),
            cancel,
            timeout,
        )
        .await
        .map_err(|e| map_auth_failure(e, AUTH_HINT))
    }
    .await;
    let (session_id, created) = match created.and_then(|created| AcpSession::id_from(&created).map(|id| (id, created))) {
        Ok(pair) => pair,
        Err(e) => {
            rpc.kill_and_wait().await;
            return Err(e);
        }
    };
    let mut session = AcpSession::new(rpc, session_id, false, Some((program.to_path_buf(), cwd.to_path_buf())));

    if !model.trim().is_empty() {
        session.set_config_option("model", model.trim(), cancel, timeout).await?;
    }

    if let Some(effort) = reasoning_effort.map(str::trim).filter(|s| !s.is_empty())
        && let Err(e) = session.set_config_option("effort", effort, cancel, timeout).await
    {
        tracing::warn!(error = %e, "OpenCode effort option rejected; using default");
    }

    Ok((session, created))
}

pub async fn list_models(program: &Path, cancel: &CancellationToken, timeout: Duration) -> Result<Vec<String>, TranslateError> {
    let cwd = TempCwd::create()?;
    let (session, created) = connect(program, "", None, cwd.path(), "You are a translation engine.", cancel, timeout).await?;
    models_after_connect(session, created).await
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
        assert_eq!(v["tools"]["skill"], false);
        assert_eq!(v["permission"]["*"], "ask");
        assert_eq!(v["agent"]["build"]["prompt"], "{file:./TRANSLATE.md}");
        for name in ["build", "plan", "explore", "general"] {
            assert_eq!(v["agent"][name]["permission"]["*"], "ask");
            assert_eq!(v["agent"][name]["permission"]["skill"], "deny");
        }
        assert_eq!(v["experimental"]["continue_loop_on_deny"], true);
    }
}
