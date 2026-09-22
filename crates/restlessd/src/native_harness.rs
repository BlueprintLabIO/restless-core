//! Explicit native-auth boundary: OAuth belongs to the native CLI's private company profile.
//! API keys remain referenced in Infisical and are injected only into the selected harness.
use crate::{credential, runtime};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
const HELPER: &str = "/usr/local/bin/restless-harness-auth";
pub const CODEX_HOME: &str = "/company/home/.restless/harness-auth/codex";
pub const CLAUDE_HOME: &str = "/company/home/.restless/harness-auth/claude-agent";
const SOURCE: &str = include_str!("../../../tools/harness-auth/auth.mjs");
pub fn validate(harness: &str) -> Result<()> {
    if !["codex", "claude-agent"].contains(&harness) {
        bail!("Unknown native harness");
    }
    Ok(())
}
async fn install(company: &str) -> Result<()> {
    // Installing the helper is idempotent, and also supports already-running pinned images.
    use tokio::io::AsyncWriteExt;
    let mut child = tokio::process::Command::new("docker")
        .args(["exec", "-i", &runtime::container_name(company), "sh", "-c", "umask 022; tmp=/usr/local/bin/restless-harness-auth.tmp.$$; cat > \"$tmp\"; chmod 755 \"$tmp\"; mv \"$tmp\" /usr/local/bin/restless-harness-auth"])
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).kill_on_drop(true).spawn()?;
    child
        .stdin
        .take()
        .context("helper input")?
        .write_all(SOURCE.as_bytes())
        .await?;
    if !child.wait().await?.success() {
        bail!("Company computer must be running");
    }
    Ok(())
}
pub async fn command(company: &str, harness: &str, action: &str) -> Result<Value> {
    validate(harness)?;
    if !["login", "status", "logout", "cancel"].contains(&action) {
        bail!("Unknown authentication action");
    }
    install(company).await?;
    let container = runtime::container_name(company);
    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("exec");
    if action == "login" {
        cmd.arg("-d");
    }
    cmd.args(["-u", "company", &container, "node", HELPER, harness, action])
        .kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(25), cmd.output()).await??;
    if !output.status.success() {
        bail!("Native authentication is unavailable. Check the Company computer and retry.");
    }
    if action == "login" {
        return Ok(json!({"state":"starting"}));
    }
    serde_json::from_slice(&output.stdout).context("Invalid native authentication status")
}
pub async fn view(config: &runtime::CompanyConfig, harness: &str) -> Value {
    let connection = config.native_harnesses.get(harness);
    let mode = connection
        .map(|c| c.mode.as_str())
        .unwrap_or("disconnected");
    let status = if mode == "api_key" {
        match connection.and_then(|c| c.credential_reference.as_deref()) {
            Some(reference) if credential::resolve_reference(reference).await.is_ok() => {
                json!({"state":"key_saved","message":"API key stored. Provider acceptance is checked on the next model request."})
            }
            _ => json!({"state":"disconnected"}),
        }
    } else if mode == "oauth" {
        command(&config.name, harness, "status").await.unwrap_or_else(|_| json!({"state":"unavailable","message":"Start the Company computer to check native sign-in."}))
    } else {
        json!({"state":"disconnected"})
    };
    json!({"harness":harness,"configured":connection.is_some(),"mode":mode,"model":connection.map(|c| c.model.as_str()).unwrap_or(if harness=="codex" {"gpt-5.4"} else {"claude-sonnet-4-6"}),"auth":status})
}
pub async fn session_access(
    company: &str,
    actor: &str,
    model: &str,
) -> Result<crate::model_gateway::AgentGatewayAuth> {
    let root = restlessd::appliance::MachineProfile::from_env()?.state_root;
    let config = runtime::CompanyConfig::load(&root, company)?.for_agent(actor);
    let harness = if model.starts_with("native-codex-") {
        "codex"
    } else if model.starts_with("native-claude-") {
        "claude-agent"
    } else {
        bail!("Unknown native model route")
    };
    let connection = config
        .native_harnesses
        .get(harness)
        .context("Connect this harness first")?;
    let selected = if harness == "codex" {
        runtime::AgentHarness::Codex
    } else {
        runtime::AgentHarness::ClaudeAgent
    };
    if config.native_model(selected).as_deref() != Some(model) {
        bail!("Harness settings changed; retry with the current model");
    }
    let (token_env, token, billing) = match connection.mode.as_str() {
        "api_key" => (
            if harness == "codex" {
                "OPENAI_API_KEY"
            } else {
                "ANTHROPIC_API_KEY"
            },
            credential::resolve_reference(
                connection
                    .credential_reference
                    .as_deref()
                    .context("Harness API key missing")?,
            )
            .await?,
            crate::model_gateway::ModelBilling::NativeApi,
        ),
        "oauth" => {
            if command(company, harness, "status").await?["state"] != "connected" {
                bail!("Sign in to {harness} in Company → Intelligence provider first");
            }
            (
                "RESTLESS_NATIVE_AUTH",
                "oauth".into(),
                crate::model_gateway::ModelBilling::Subscription,
            )
        }
        _ => bail!("Connect {harness} in Company → Intelligence provider first"),
    };
    Ok(crate::model_gateway::AgentGatewayAuth {
        token_env: token_env.into(),
        token,
        runtime_url: "https://api.openai.com/v1".into(),
        billing,
    })
}
/// Runs independently for every company at startup/creation; one failure never blocks others.
pub async fn startup_doctor(
    root: std::path::PathBuf,
    config: runtime::CompanyConfig,
    capabilities: crate::capability::CapabilityIssuer,
    org: restless_orgintel::OrgIntel,
) {
    let (documents, setup_failed) = tokio::join!(
        async {
            let owner_config = crate::owner::OwnerConfig::from_env()?;
            if let Some(issuer) = owner_config.local_documents_issuer() {
                crate::local_documents::ensure(&root, &config.name, &org, &issuer).await?;
                Ok::<_, anyhow::Error>(json!({"status":"ready"}))
            } else {
                Ok(json!({"status":"managed_externally"}))
            }
        },
        async {
            async {
                runtime::up(&config, false).await?;
                let bridge = capabilities.issue_runtime_bridge(&config.name)?;
                runtime::install_runtime_bridge_capability(&config.name, &bridge).await
            }
            .await
            .is_err()
        }
    );
    let documents = match documents {
        Ok(status) => status,
        Err(error) => {
            tracing::warn!(company=%config.name, %error, "automatic Documents setup failed");
            json!({"status":"unavailable", "error":format!("{error:#}")})
        }
    };
    let result = runtime::doctor(&config.name).await;
    let report = match result {
        Ok(report) => {
            json!({"ran_at":chrono::Utc::now(),"setup_failed":setup_failed,"documents":documents,"report":report})
        }
        Err(_) => {
            json!({"ran_at":chrono::Utc::now(),"documents":documents,"error":"Automatic computer setup or Doctor failed. Open Company Doctor for current diagnostics."})
        }
    };
    let dir = root.join("diagnostics");
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(
            dir.join(format!("{}-startup-doctor.json", config.name)),
            report.to_string(),
        );
    }
    tracing::info!(company=%config.name, "automatic startup Doctor completed");
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_harness_models_and_credentials_are_independent() {
        use crate::runtime::{AgentHarness, CompanyConfig};
        let config: CompanyConfig = toml::from_str(
            r#"
name = "native_auth_test"
model = "anthropic/claude-sonnet-4-6"
coordination_harness = "codex"
worker_harness = "claude-agent"
[credentials]
"model.inference.anthropic" = "env:DIRECT_TEST"
[native_harnesses.codex]
mode = "oauth"
model = "gpt-5.4"
[native_harnesses.claude-agent]
mode = "api_key"
model = "claude-opus-4-6"
credential_reference = "infisical:/companies/native_auth_test/HARNESS_CLAUDE_API_KEY"
"#,
        )
        .unwrap();
        config.validate_harness_models().unwrap();
        assert_eq!(
            config.native_model(AgentHarness::Codex).as_deref(),
            Some("native-codex-oauth/gpt-5.4")
        );
        assert_eq!(
            config.native_model(AgentHarness::ClaudeAgent).as_deref(),
            Some("native-claude-api/claude-opus-4-6")
        );
        assert!(config.native_model(AgentHarness::RestlessManaged).is_none());
        assert_eq!(
            config.credentials["model.inference.anthropic"],
            "env:DIRECT_TEST"
        );
        assert_ne!(
            config.native_harnesses["claude-agent"]
                .credential_reference
                .as_deref(),
            Some("env:DIRECT_TEST")
        );
    }
    #[test]
    fn only_native_harness_ids_are_accepted() {
        for id in ["codex", "claude-agent"] {
            assert!(super::validate(id).is_ok());
        }
        for id in ["../codex", "claude", "restless-managed", "codex;true"] {
            assert!(super::validate(id).is_err());
        }
    }
}
