//! Governance for ordinary runtime commands.
//!
//! Restless does not own an email API, Git API, or provider catalogue. An
//! actor chooses an installed CLI and argv in the persistent company computer.
//! Authority adds the small constitutional layer around that process:
//! approval, scoped secret injection, idempotency, and a generic JSON receipt.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _};
use uuid::Uuid;

use crate::runtime::CompanyConfig;
use restlessd::runtime_transport::{
    CompanyPath, RuntimeEffectPhase, RuntimeEnvironment, RuntimeProcess, RuntimeProcessAuthority,
    RuntimeProcessSpec, RuntimeTransport,
};

/// Effect children are the only runtime processes that receive live secret
/// values. Serialising them prevents one actor-selected effect command from
/// inspecting a concurrent effect child's `/proc/<pid>/environ` under the
/// shared effect UID. Material effects are low volume; measured demand, not a
/// speculative pool, should justify a more elaborate isolation scheme.
static EFFECT_CHILD_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Reap only governed-effect children left by a previous daemon lifetime,
/// then clean only UUID-shaped staging directories. This runs before the
/// scheduler and owner gateway start, so a new secret-bearing child can never
/// overlap an unsupervised child under the shared effect UID. Authority intent
/// remains `unknown`; killing a local process does not infer the external
/// result.
pub async fn sweep_orphans(root: &Path, runtime_transport: &Arc<dyn RuntimeTransport>) {
    let backend = restlessd::hosted_runtime::RuntimeBackendKind::from_entry_mode(
        std::env::var("RESTLESS_ENTRY_MODE").ok().as_deref(),
    );
    match backend {
        Ok(restlessd::hosted_runtime::RuntimeBackendKind::LocalDocker) => {
            sweep_local_orphans(root).await
        }
        Ok(restlessd::hosted_runtime::RuntimeBackendKind::HostedRuntimeBridge) => {
            sweep_transport_orphans(root, runtime_transport).await
        }
        Err(error) => tracing::error!(%error, "cannot select governed-effect recovery backend"),
    }
}

async fn sweep_transport_orphans(root: &Path, runtime_transport: &Arc<dyn RuntimeTransport>) {
    let Ok(entries) = std::fs::read_dir(root.join("companies")) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(company) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let activity = match runtime_transport.activity(company).await {
            Ok(activity) => activity,
            Err(error) => {
                tracing::warn!(company, %error, "could not observe orphaned governed effects");
                continue;
            }
        };
        for process in activity.active_processes {
            let RuntimeProcessAuthority::GovernedEffect {
                phase, staging_id, ..
            } = &process.authority
            else {
                continue;
            };
            let mut recovery = process.authority.clone();
            if let RuntimeProcessAuthority::GovernedEffect { phase: target, .. } = &mut recovery {
                *target = if phase.uses_effect_identity() {
                    RuntimeEffectPhase::RecoverEffectProcess
                } else {
                    RuntimeEffectPhase::RecoverCompanyProcess
                };
            }
            if let Err(error) =
                terminate_runtime_pid(runtime_transport.as_ref(), &recovery, process.pid).await
            {
                tracing::warn!(company, process = %process.process_id, %error, "failed to reap orphaned governed effect");
            }
            if let RuntimeProcessAuthority::GovernedEffect { phase, .. } = &mut recovery {
                *phase = RuntimeEffectPhase::RecoverEffectProcess;
            }
            let staging = staging_path(*staging_id);
            if let Err(error) =
                cleanup_staging(runtime_transport.as_ref(), &recovery, &staging).await
            {
                tracing::warn!(company, path = %staging, %error, "failed to clean orphaned governed-effect staging");
            }
        }
    }
}

async fn sweep_local_orphans(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root.join("companies")) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(company) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if !matches!(
            crate::runtime::status(company).await,
            Ok(crate::runtime::ContainerStatus::Running)
        ) {
            continue;
        }
        let container = crate::runtime::container_name(company);
        let mut reaped = false;
        for signal in ["-TERM", "-KILL"] {
            let result = tokio::process::Command::new("docker")
                .args([
                    "exec", "-u", "0:0", &container, "pkill", signal, "-u", "2001",
                ])
                .output()
                .await;
            match result {
                Ok(output) if output.status.success() || output.status.code() == Some(1) => {
                    if signal == "-TERM" && output.status.success() {
                        reaped = true;
                    }
                }
                Ok(output) => tracing::warn!(
                    company,
                    signal,
                    stderr = %String::from_utf8_lossy(&output.stderr).trim(),
                    "failed to reap orphaned governed-effect child"
                ),
                Err(error) => {
                    tracing::warn!(company, signal, %error, "failed to inspect governed-effect children")
                }
            }
            if signal == "-TERM" {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
        if reaped {
            tracing::warn!(
                company,
                "reaped orphaned governed-effect process after daemon restart"
            );
        }

        let mut candidates = Vec::new();
        for args in [
            vec![
                "exec",
                "-u",
                "0:0",
                &container,
                "find",
                "/tmp/restless-effect",
                "-mindepth",
                "1",
                "-maxdepth",
                "1",
                "-type",
                "d",
                "-print",
            ],
            vec![
                "exec",
                "-u",
                "0:0",
                &container,
                "find",
                "/tmp",
                "-mindepth",
                "1",
                "-maxdepth",
                "1",
                "-type",
                "d",
                "-name",
                "restless-effect-stage-*",
                "-print",
            ],
        ] {
            match tokio::process::Command::new("docker")
                .args(args)
                .output()
                .await
            {
                Ok(output) if output.status.success() => candidates.extend(
                    String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .map(str::to_string),
                ),
                Ok(output)
                    if output.status.code() == Some(1)
                        && String::from_utf8_lossy(&output.stderr)
                            .contains("No such file or directory") => {}
                Ok(output) => tracing::warn!(
                    company,
                    stderr = %String::from_utf8_lossy(&output.stderr).trim(),
                    "failed to enumerate governed-effect staging"
                ),
                Err(error) => {
                    tracing::warn!(company, %error, "failed to enumerate governed-effect staging")
                }
            }
        }
        for candidate in candidates {
            if !valid_staging_path(&candidate) {
                tracing::warn!(company, path = %candidate, "refusing unexpected governed-effect staging path");
                continue;
            }
            if let Err(error) = cleanup_local_staging(&container, &candidate).await {
                tracing::warn!(company, path = %candidate, %error, "failed to clean orphaned governed-effect staging");
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Receipt {
    pub id: Uuid,
    pub effect_class: String,
    pub command_digest: String,
    pub tool: String,
    pub purpose: String,
    pub cwd: String,
    pub argv: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub party: Option<String>,
    pub outcome: serde_json::Value,
    pub success: bool,
    pub actor: String,
    pub idempotency_key: String,
    #[serde(default = "default_execution_no")]
    pub execution_no: i32,
    pub created_at: DateTime<Utc>,
    pub replayed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_of: Option<String>,
}

pub struct EffectEnvironment<'a> {
    pub config: &'a CompanyConfig,
    pub authority: &'a crate::authority::AuthorityStore,
    pub org: Option<&'a restless_orgintel::OrgIntel>,
    pub runtime_transport: &'a Arc<dyn RuntimeTransport>,
}

#[allow(clippy::too_many_arguments)]
pub async fn request_effect(
    environment: EffectEnvironment<'_>,
    effect_class: &str,
    party: Option<&str>,
    purpose: &str,
    artifacts: Vec<String>,
    cwd: &str,
    argv: Vec<String>,
    secret_bindings: BTreeMap<String, String>,
    key: &str,
    actor: &str,
) -> Result<Receipt> {
    let EffectEnvironment {
        config,
        authority,
        org,
        runtime_transport,
    } = environment;
    if key.trim().is_empty() {
        bail!("effect needs an idempotency key");
    }
    if purpose.trim().is_empty() || purpose.chars().count() > 1_000 {
        bail!("effect purpose must contain between 1 and 1,000 characters");
    }
    if artifacts.len() > 32
        || artifacts
            .iter()
            .any(|artifact| artifact.trim().is_empty() || artifact.chars().count() > 2_048)
    {
        bail!("effect may carry at most 32 non-empty artifact references of 2,048 characters");
    }
    if !valid_identifier(effect_class) {
        bail!("invalid effect class {effect_class:?}");
    }
    if is_finance_identifier(effect_class) {
        bail!(
            "financial effects use the host-side Authority adapter; the generic Runtime child cannot execute {effect_class:?}"
        );
    }
    if !valid_company_cwd(cwd) {
        bail!("effect cwd must be an absolute path under /company");
    }
    let (program, _) = argv.split_first().context("effect command is empty")?;
    if program.trim().is_empty() || argv.iter().any(|value| value.contains('\0')) {
        bail!("effect argv contains an empty program or NUL");
    }
    for name in secret_bindings.keys() {
        if !valid_env_name(name) {
            bail!("invalid child environment name {name:?}");
        }
    }
    if secret_bindings
        .values()
        .any(|binding| is_finance_identifier(binding))
    {
        bail!("finance credential bindings terminate in the host-side Authority adapter");
    }
    if crate::runtime::is_test_company(&config.name) && !secret_bindings.is_empty() {
        bail!(
            "test companies cannot receive live secret bindings; install a fake CLI for the scenario"
        );
    }

    let party = party
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty());
    let command_document = serde_json::json!({
        "class": effect_class,
        "party": party,
        "purpose": purpose,
        "artifacts": artifacts,
        "cwd": cwd,
        "argv": argv,
        "secret_bindings": secret_bindings,
    });
    let command_digest = format!(
        "{:x}",
        Sha256::digest(command_document.to_string().as_bytes())
    );

    let stored_receipt = authority
        .find_body(&config.name, "effect", "idempotency_key", key)
        .await?;
    if let Some(stored) = stored_receipt.as_ref() {
        let mut receipt: Receipt = serde_json::from_value(stored.clone())
            .context("stored effect record is not a generic receipt")?;
        if receipt.command_digest != command_digest {
            bail!("idempotency key {key:?} was already used with a different command");
        }
        if receipt.success {
            receipt.replayed = true;
            authority.emit(&config.name, "effect_replayed", Some(actor), serde_json::json!({
                "effect_class": effect_class, "idempotency_key": key, "receipt_id": receipt.id,
            })).await?;
            return Ok(receipt);
        }
    }
    let latest_intent = authority
        .find_body(&config.name, "effect_intent", "idempotency_key", key)
        .await?;
    if let Some(intent) = latest_intent.as_ref() {
        if intent
            .get("command_digest")
            .and_then(serde_json::Value::as_str)
            != Some(&command_digest)
        {
            bail!("idempotency key {key:?} was already used with a different command");
        }
        let intent_execution = execution_no(intent);
        let receipt_execution = stored_receipt.as_ref().map(execution_no).unwrap_or(0);
        if intent_execution > receipt_execution {
            bail!(
                "effect {key:?} has an unknown outcome from execution {intent_execution}; reconcile it against external evidence before retrying"
            );
        }
    }

    let repeat = match party.as_deref() {
        Some(party) => prior_effect_on(authority, &config.name, effect_class, party, key).await?,
        None => None,
    };
    if let crate::approval::Decision::NeedsOwner(reason) = crate::approval::check(
        config,
        authority,
        effect_class,
        party.as_deref(),
        crate::runtime::is_test_company(&config.name),
    )
    .await?
    {
        authority
            .emit(
                &config.name,
                "approval_required",
                Some(actor),
                serde_json::json!({
                    "effect_class": effect_class,
                    "party": party,
                    "reason": reason,
                    "prepared_command": command_document,
                }),
            )
            .await?;
        if let Some(org) = org {
            let _ = org.ensure_actor("exec", "exec", "exec", "The Exec").await;
        }
        bail!("{reason}");
    }

    let mut secrets = BTreeMap::new();
    for (environment_name, binding) in &secret_bindings {
        let value = crate::credential::resolve(config, binding).await?;
        secrets.insert(
            environment_name.clone(),
            trim_line_endings(&value).to_string(),
        );
    }
    let execution_no = stored_receipt
        .as_ref()
        .map(execution_no)
        .unwrap_or(0)
        .saturating_add(1);
    // Preparation is durably attributed but remains distinct from external
    // intent: a staging failure must not manufacture an unknown-world result.
    let staging_id = Uuid::new_v4();
    let preparation_id = authority
        .emit(
            &config.name,
            "effect_preparation",
            Some(actor),
            serde_json::json!({
                "actor": actor,
                "effect_class": effect_class,
                "idempotency_key": key,
                "execution_no": execution_no,
                "staging_id": staging_id,
                "command_digest": command_digest,
            }),
        )
        .await?;
    let preparation_authority = RuntimeProcessAuthority::GovernedEffect {
        company: config.name.clone(),
        actor: actor.to_owned(),
        effect_class: effect_class.to_owned(),
        authority_id: preparation_id,
        idempotency_key: key.to_owned(),
        execution_no,
        staging_id,
        phase: RuntimeEffectPhase::WorkspaceAccess,
    };
    ensure_effect_workspace_access(runtime_transport.as_ref(), &preparation_authority).await?;
    let (child_argv, staging_dir) = stage_declared_artifacts(
        runtime_transport.as_ref(),
        &preparation_authority,
        &argv,
        &artifacts,
    )
    .await?;
    let intent = serde_json::json!({
        "idempotency_key": key,
        "execution_no": execution_no,
        "command_digest": command_digest,
        "effect_class": effect_class,
        "party": party,
        "purpose": purpose,
        "artifacts": artifacts,
        "command": command_document,
        "staging_id": staging_id,
        "started_at": Utc::now(),
    });
    let intent_id = match authority
        .claim_effect_intent(&config.name, actor, intent)
        .await
    {
        Ok(Some(intent_id)) => intent_id,
        Ok(None) => {
            if let Some(staging_dir) = staging_dir.as_deref() {
                cleanup_staging(
                    runtime_transport.as_ref(),
                    &preparation_authority,
                    staging_dir,
                )
                .await?;
            }
            bail!(
                "effect {key:?} execution {execution_no} is already in flight or awaiting reconciliation"
            );
        }
        Err(error) => {
            if let Some(staging_dir) = staging_dir.as_deref() {
                let _ = cleanup_staging(
                    runtime_transport.as_ref(),
                    &preparation_authority,
                    staging_dir,
                )
                .await;
            }
            return Err(error);
        }
    };
    let mut execution_authority = preparation_authority;
    if let RuntimeProcessAuthority::GovernedEffect {
        authority_id,
        phase,
        ..
    } = &mut execution_authority
    {
        *authority_id = intent_id;
        *phase = RuntimeEffectPhase::Execute;
    }
    let output = run_child(
        runtime_transport.as_ref(),
        &execution_authority,
        cwd,
        child_argv,
        staging_dir,
        &secrets,
    )
    .await?;
    let stdout = redact(&String::from_utf8_lossy(&output.stdout), secrets.values());
    let stderr = redact(&String::from_utf8_lossy(&output.stderr), secrets.values());
    let parsed_stdout = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| serde_json::Value::String(stdout.chars().take(10_000).collect()));
    let outcome = serde_json::json!({
        "status": if output.success() { "succeeded" } else { "failed" },
        "exit_code": output.exit_code,
        "stdout": parsed_stdout,
        "stderr": stderr.chars().take(4_000).collect::<String>(),
    });
    let receipt = Receipt {
        id: Uuid::new_v4(),
        effect_class: effect_class.to_string(),
        command_digest,
        tool: program.clone(),
        purpose: purpose.to_string(),
        cwd: cwd.to_string(),
        argv: argv.clone(),
        artifacts,
        party: party.clone(),
        outcome,
        success: output.success(),
        actor: actor.to_string(),
        idempotency_key: key.to_string(),
        execution_no,
        created_at: Utc::now(),
        replayed: false,
        repeat_of: repeat.clone(),
    };
    authority
        .emit(
            &config.name,
            "effect",
            Some(actor),
            serde_json::to_value(&receipt)?,
        )
        .await?;
    if let Some(earlier_key) = repeat {
        authority.emit(&config.name, "effect_repeat_party", Some(actor), serde_json::json!({
            "effect_class": effect_class, "party": party, "earlier_key": earlier_key, "this_key": key,
        })).await?;
    }
    Ok(receipt)
}

fn is_finance_identifier(value: &str) -> bool {
    value == "finance"
        || value.starts_with("finance.")
        || value.starts_with("finance/")
        || value.contains("/finance/")
}

/// Close the one ambiguity the generic runner cannot decide: the daemon died
/// after recording intent but before recording the child result. Reconciliation
/// is accepted only when another successful generic effect receipt points at
/// the external system's own status observation.
pub async fn reconcile_unknown(
    authority: &crate::authority::AuthorityStore,
    company: &str,
    key: &str,
    expected_execution: i32,
    result: &str,
    evidence_receipt: &str,
    actor: &str,
) -> Result<Receipt> {
    if expected_execution <= 0 {
        bail!("effect execution number must be positive");
    }
    let success = match result {
        "succeeded" => true,
        "failed" => false,
        other => bail!("reconciled result must be succeeded|failed, got {other:?}"),
    };
    let intent = authority
        .find_body(company, "effect_intent", "idempotency_key", key)
        .await?
        .with_context(|| format!("effect {key:?} has no recorded execution intent"))?;
    if execution_no(&intent) != expected_execution {
        bail!(
            "latest execution for {key:?} is {}, not {expected_execution}",
            execution_no(&intent)
        );
    }
    let completed = authority
        .records_of_kind(company, "effect")
        .await?
        .into_iter()
        .any(|record| {
            record
                .body
                .get("idempotency_key")
                .and_then(serde_json::Value::as_str)
                == Some(key)
                && execution_no(&record.body) == expected_execution
        });
    if completed {
        bail!("effect {key:?} execution {expected_execution} already has a receipt");
    }
    let evidence: Receipt = serde_json::from_value(
        authority
            .find_body(company, "effect", "id", evidence_receipt)
            .await?
            .with_context(|| format!("evidence receipt {evidence_receipt:?} does not exist"))?,
    )
    .context("evidence is not a generic effect receipt")?;
    if !evidence.success {
        bail!("evidence receipt must be a successful external status check");
    }
    if evidence.idempotency_key == key {
        bail!("an effect cannot serve as its own reconciliation evidence");
    }

    let command = intent
        .get("command")
        .context("effect intent is missing its command")?;
    let argv: Vec<String> = serde_json::from_value(
        command
            .get("argv")
            .cloned()
            .context("effect intent is missing argv")?,
    )?;
    let tool = argv
        .first()
        .cloned()
        .context("effect intent has empty argv")?;
    let artifacts = intent
        .get("artifacts")
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    let receipt = Receipt {
        id: Uuid::new_v4(),
        effect_class: intent
            .get("effect_class")
            .and_then(serde_json::Value::as_str)
            .context("effect intent is missing class")?
            .to_string(),
        command_digest: intent
            .get("command_digest")
            .and_then(serde_json::Value::as_str)
            .context("effect intent is missing command digest")?
            .to_string(),
        tool,
        purpose: intent
            .get("purpose")
            .and_then(serde_json::Value::as_str)
            .context("effect intent is missing purpose")?
            .to_string(),
        cwd: command
            .get("cwd")
            .and_then(serde_json::Value::as_str)
            .context("effect intent is missing cwd")?
            .to_string(),
        argv,
        artifacts,
        party: intent
            .get("party")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        outcome: serde_json::json!({
            "status": result,
            "reconciled": true,
            "evidence_receipt": evidence.id,
        }),
        success,
        actor: actor.to_string(),
        idempotency_key: key.to_string(),
        execution_no: expected_execution,
        created_at: Utc::now(),
        replayed: false,
        repeat_of: None,
    };
    authority
        .emit(
            company,
            "effect",
            Some(actor),
            serde_json::to_value(&receipt)?,
        )
        .await?;
    authority
        .emit(
            company,
            "effect_reconciled",
            Some(actor),
            serde_json::json!({
                "idempotency_key": key,
                "execution_no": expected_execution,
                "receipt_id": receipt.id,
                "evidence_receipt": evidence.id,
            }),
        )
        .await?;
    Ok(receipt)
}

async fn run_child(
    runtime_transport: &dyn RuntimeTransport,
    authority: &RuntimeProcessAuthority,
    cwd: &str,
    child_argv: Vec<String>,
    staging_dir: Option<String>,
    secrets: &BTreeMap<String, String>,
) -> Result<EffectOutput> {
    let _serial = EFFECT_CHILD_SERIAL.lock().await;
    let environment = secrets
        .iter()
        .map(|(name, value)| RuntimeEnvironment::secret(name.clone(), value.clone()))
        .collect();
    let output =
        run_runtime_process(runtime_transport, authority, cwd, child_argv, environment).await;
    if let Some(staging_dir) = staging_dir {
        if let Err(error) = cleanup_staging(runtime_transport, authority, &staging_dir).await {
            if output.is_ok() {
                tracing::warn!(%error, %staging_dir, "governed artifact staging cleanup failed after child completion");
            }
        }
    }
    output
}

/// Older runtime turns used umask 077, which made the persistent company
/// workspace invisible to the isolated effect UID. Keep credential isolation
/// (UID 2001) while sharing productive files through the company group (GID
/// 2000). New turns use umask 007; this bounded one-time migration repairs
/// files that already exist in a long-lived company computer.
async fn ensure_effect_workspace_access(
    runtime_transport: &dyn RuntimeTransport,
    authority: &RuntimeProcessAuthority,
) -> Result<()> {
    const SCRIPT: &str = r#"
marker=/company/run/.effect-group-access-v1
test -f "$marker" && exit 0
for root do
  test ! -e "$root" || chmod -R g+rwX -- "$root"
done
(umask 117; : > "$marker")
chmod 0660 "$marker"
"#;
    let mut workspace_authority = authority.clone();
    set_effect_phase(
        &mut workspace_authority,
        RuntimeEffectPhase::WorkspaceAccess,
    )?;
    let output = run_runtime_process(
        runtime_transport,
        &workspace_authority,
        "/company",
        vec![
            "sh".into(),
            "-c".into(),
            SCRIPT.into(),
            "effect-workspace-access".into(),
            "/company/repos".into(),
            "/company/worktrees".into(),
            "/company/workspaces".into(),
            "/company/projects".into(),
            "/company/outputs".into(),
            "/company/downloads".into(),
        ],
        Vec::new(),
    )
    .await
    .context("prepare governed-effect workspace access")?;
    require_success(output, "prepare governed-effect workspace access")
}

/// Copy only declared local artifacts that appear as exact argv values into a
/// private directory readable by the isolated effect UID. The governance
/// receipt retains the original paths; only the child envelope sees staging
/// paths. URLs and descriptive artifact references are not copied.
async fn stage_declared_artifacts(
    runtime_transport: &dyn RuntimeTransport,
    authority: &RuntimeProcessAuthority,
    argv: &[String],
    artifacts: &[String],
) -> Result<(Vec<String>, Option<String>)> {
    let local = artifacts
        .iter()
        .filter(|artifact| argv.contains(artifact) && valid_company_path(artifact))
        .collect::<Vec<_>>();
    if local.is_empty() {
        return Ok((argv.to_vec(), None));
    }

    let staging_id = match authority {
        RuntimeProcessAuthority::GovernedEffect { staging_id, .. } => *staging_id,
        _ => bail!("artifact staging requires governed-effect authority"),
    };
    let staging_dir = staging_path(staging_id);
    let mut staging_authority = authority.clone();
    set_effect_phase(&mut staging_authority, RuntimeEffectPhase::ArtifactStage)?;
    let created = run_runtime_process(
        runtime_transport,
        &staging_authority,
        "/company",
        vec![
            "install".into(),
            "-d".into(),
            "-m".into(),
            "0700".into(),
            staging_dir.clone(),
        ],
        Vec::new(),
    )
    .await
    .context("create governed artifact staging directory")?;
    require_success(created, "create governed artifact staging directory")?;

    let staged = async {
        let mut rewritten = argv.to_vec();
        for (index, source) in local.iter().enumerate() {
            let filename = Path::new(source)
                .file_name()
                .and_then(|name| name.to_str())
                .context("declared local artifact has no UTF-8 filename")?;
            let item_dir = format!("{staging_dir}/{index}");
            let destination = format!("{item_dir}/{filename}");
            let output = run_runtime_process(
                runtime_transport,
                &staging_authority,
                "/company",
                vec![
                    "install".into(),
                    "-D".into(),
                    "-m".into(),
                    "0400".into(),
                    (*source).clone(),
                    destination,
                ],
                Vec::new(),
            )
            .await
            .with_context(|| format!("stage declared artifact {source:?}"))?;
            require_success(output, &format!("stage declared artifact {source:?}"))?;
            for value in &mut rewritten {
                if value == *source {
                    *value = format!("{staging_dir}/{index}/{filename}");
                }
            }
        }
        Ok::<_, anyhow::Error>(rewritten)
    }
    .await;

    if let Err(error) = &staged {
        let _ = cleanup_staging(runtime_transport, &staging_authority, &staging_dir).await;
        return Err(anyhow::anyhow!("{error:#}"));
    }
    Ok((staged?, Some(staging_dir)))
}

async fn cleanup_staging(
    runtime_transport: &dyn RuntimeTransport,
    authority: &RuntimeProcessAuthority,
    path: &str,
) -> Result<()> {
    if !valid_staging_path(path) {
        bail!("refusing to clean unexpected governed staging path {path:?}");
    }
    let mut cleanup_authority = authority.clone();
    if !matches!(
        cleanup_authority,
        RuntimeProcessAuthority::GovernedEffect {
            phase: RuntimeEffectPhase::RecoverEffectProcess,
            ..
        }
    ) {
        set_effect_phase(&mut cleanup_authority, RuntimeEffectPhase::ArtifactCleanup)?;
    }
    let output = run_runtime_process(
        runtime_transport,
        &cleanup_authority,
        "/company",
        vec!["rm".into(), "-rf".into(), "--".into(), path.into()],
        Vec::new(),
    )
    .await
    .context("clean governed artifact staging directory")?;
    require_success(output, "clean governed artifact staging directory")
}

async fn cleanup_local_staging(container: &str, path: &str) -> Result<()> {
    if !valid_staging_path(path) {
        bail!("refusing to clean unexpected governed staging path {path:?}");
    }
    let cleanup = tokio::process::Command::new("docker")
        .args(["exec", "-u", "0:0", container, "rm", "-r", "--", path])
        .output()
        .await
        .context("clean governed artifact staging directory")?;
    if !cleanup.status.success() {
        bail!(
            "governed artifact staging cleanup failed: {}",
            String::from_utf8_lossy(&cleanup.stderr).trim()
        );
    }
    Ok(())
}

#[derive(Debug)]
struct EffectOutput {
    exit_code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl EffectOutput {
    fn success(&self) -> bool {
        self.exit_code == Some(0) && self.signal.is_none()
    }
}

async fn read_runtime_output(mut source: impl AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    source.read_to_end(&mut output).await?;
    Ok(output)
}

async fn run_runtime_process(
    runtime_transport: &dyn RuntimeTransport,
    authority: &RuntimeProcessAuthority,
    cwd: &str,
    mut argv: Vec<String>,
    environment: Vec<RuntimeEnvironment>,
) -> Result<EffectOutput> {
    let executable = argv
        .first()
        .cloned()
        .context("effect Runtime command is empty")?;
    argv.remove(0);
    let RuntimeProcess {
        mut stdin,
        stdout,
        stderr,
        control,
        ..
    } = runtime_transport
        .start_process(RuntimeProcessSpec {
            operation_id: Uuid::new_v4(),
            authority: authority.clone(),
            executable,
            arguments: argv,
            working_directory: CompanyPath::parse(cwd.to_owned())
                .map_err(|error| anyhow::anyhow!(error))?,
            environment,
            deadline: Utc::now() + chrono::Duration::hours(23),
        })
        .await
        .map_err(|error| anyhow::anyhow!(error))?;
    stdin
        .shutdown()
        .await
        .context("close effect Runtime input")?;
    let stdout = tokio::spawn(read_runtime_output(stdout));
    let stderr = tokio::spawn(read_runtime_output(stderr));
    let exit = control
        .wait()
        .await
        .map_err(|error| anyhow::anyhow!(error))?;
    Ok(EffectOutput {
        exit_code: exit.code,
        signal: exit.signal,
        stdout: stdout.await.context("join effect stdout")??,
        stderr: stderr.await.context("join effect stderr")??,
    })
}

async fn terminate_runtime_pid(
    runtime_transport: &dyn RuntimeTransport,
    authority: &RuntimeProcessAuthority,
    pid: u32,
) -> Result<()> {
    if pid <= 1 {
        bail!("refusing invalid governed-effect Runtime PID");
    }
    let output = run_runtime_process(
        runtime_transport,
        authority,
        "/company",
        vec![
            "sh".into(),
            "-c".into(),
            "kill -TERM \"$1\" 2>/dev/null || true; sleep 0.1; kill -KILL \"$1\" 2>/dev/null || true".into(),
            "effect-orphan-recovery".into(),
            pid.to_string(),
        ],
        Vec::new(),
    )
    .await?;
    require_success(output, "terminate orphaned governed effect")
}

fn require_success(output: EffectOutput, operation: &str) -> Result<()> {
    if output.success() {
        return Ok(());
    }
    bail!(
        "{operation}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

fn set_effect_phase(
    authority: &mut RuntimeProcessAuthority,
    phase: RuntimeEffectPhase,
) -> Result<()> {
    let RuntimeProcessAuthority::GovernedEffect { phase: target, .. } = authority else {
        bail!("governed effect process requires exact effect authority");
    };
    *target = phase;
    Ok(())
}

fn staging_path(staging_id: Uuid) -> String {
    format!("/tmp/restless-effect/{}", staging_id.simple())
}

fn valid_staging_path(path: &str) -> bool {
    let leaf = path
        .strip_prefix("/tmp/restless-effect/")
        .or_else(|| path.strip_prefix("/tmp/restless-effect-stage-"));
    leaf.is_some_and(|leaf| leaf.len() == 32 && leaf.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn default_execution_no() -> i32 {
    1
}

fn execution_no(body: &serde_json::Value) -> i32 {
    body.get("execution_no")
        .and_then(serde_json::Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or(1)
}

pub(crate) async fn prior_effect_on(
    authority: &crate::authority::AuthorityStore,
    company: &str,
    effect_class: &str,
    party: &str,
    key: &str,
) -> Result<Option<String>> {
    for event in authority.records_of_kind(company, "effect").await? {
        if !crate::reconcile::is_governed_receipt(&event.body) {
            continue;
        }
        let class = event
            .body
            .get("effect_class")
            .or_else(|| event.body.get("capability"));
        if class.and_then(serde_json::Value::as_str) != Some(effect_class) {
            continue;
        }
        if event.body.get("party").and_then(serde_json::Value::as_str) != Some(party) {
            continue;
        }
        if !event
            .body
            .get("success")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_else(|| {
                event
                    .body
                    .get("outcome")
                    .is_some_and(|outcome| outcome.get("error").is_none())
            })
        {
            continue;
        }
        let earlier = event
            .body
            .get("idempotency_key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if !earlier.is_empty() && earlier != key {
            return Ok(Some(earlier.to_string()));
        }
    }
    Ok(None)
}

fn trim_line_endings(value: &str) -> &str {
    value.trim_end_matches(['\r', '\n'])
}

fn redact<'a>(text: &str, secrets: impl Iterator<Item = &'a String>) -> String {
    secrets.fold(text.to_string(), |redacted, secret| {
        if secret.is_empty() {
            redacted
        } else {
            redacted.replace(secret, "[REDACTED]")
        }
    })
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_'))
        && !value.contains("..")
}

fn valid_env_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                byte.is_ascii_uppercase() || byte == b'_'
            } else {
                byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'
            }
        })
}

fn valid_company_cwd(value: &str) -> bool {
    value == "/company"
        || (value.starts_with("/company/") && !value.split('/').any(|part| part == ".."))
}

fn valid_company_path(value: &str) -> bool {
    value.starts_with("/company/") && !value.split('/').any(|part| part == "..")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    use std::sync::Mutex;

    use restlessd::runtime_transport::{
        RuntimeActiveProcess, RuntimeActivity, RuntimeDirectoryEntry, RuntimeDuplex,
        RuntimeFileMetadata, RuntimeProcessControl, RuntimeProcessExit, RuntimeReadiness,
        RuntimeService, RuntimeTransportError,
    };

    #[derive(Default)]
    struct NetworkEffectTransport {
        specifications: Mutex<Vec<RuntimeProcessSpec>>,
        active_processes: Mutex<Vec<RuntimeActiveProcess>>,
    }

    struct CompletedProcess;

    #[async_trait::async_trait]
    impl RuntimeProcessControl for CompletedProcess {
        async fn signal(
            &self,
            _signal: restlessd::runtime_transport::RuntimeSignal,
        ) -> Result<(), RuntimeTransportError> {
            Ok(())
        }

        async fn wait(&self) -> Result<RuntimeProcessExit, RuntimeTransportError> {
            Ok(RuntimeProcessExit {
                code: Some(0),
                signal: None,
                finished_at: Utc::now(),
            })
        }
    }

    #[async_trait::async_trait]
    impl RuntimeTransport for NetworkEffectTransport {
        async fn readiness(
            &self,
            _company: &str,
        ) -> Result<RuntimeReadiness, RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn start_process(
            &self,
            specification: RuntimeProcessSpec,
        ) -> Result<RuntimeProcess, RuntimeTransportError> {
            specification.validate(Utc::now())?;
            let output = if matches!(
                &specification.authority,
                RuntimeProcessAuthority::GovernedEffect {
                    phase: RuntimeEffectPhase::Execute,
                    ..
                }
            ) {
                specification
                    .environment
                    .first()
                    .map(|value| value.value.clone())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            self.specifications.lock().unwrap().push(specification);
            let (stdin, mut stdin_reader) = tokio::io::duplex(1024);
            tokio::spawn(async move {
                let mut body = Vec::new();
                let _ = stdin_reader.read_to_end(&mut body).await;
            });
            let (mut stdout_writer, stdout) = tokio::io::duplex(1024);
            let (mut stderr_writer, stderr) = tokio::io::duplex(1024);
            let stderr_output = output.clone();
            tokio::spawn(async move {
                let _ = stdout_writer.write_all(output.as_bytes()).await;
            });
            tokio::spawn(async move {
                let _ = stderr_writer.write_all(stderr_output.as_bytes()).await;
            });
            Ok(RuntimeProcess {
                process_id: Uuid::new_v4(),
                pid: 42,
                stdin: Box::pin(stdin),
                stdout: Box::pin(stdout),
                stderr: Box::pin(stderr),
                control: Box::new(CompletedProcess),
            })
        }

        async fn stat(
            &self,
            _company: &str,
            _path: &CompanyPath,
        ) -> Result<RuntimeFileMetadata, RuntimeTransportError> {
            Err(RuntimeTransportError::NotFound)
        }

        async fn list(
            &self,
            _company: &str,
            _path: &CompanyPath,
        ) -> Result<Vec<RuntimeDirectoryEntry>, RuntimeTransportError> {
            Err(RuntimeTransportError::NotFound)
        }

        async fn read(
            &self,
            _company: &str,
            _path: &CompanyPath,
            _maximum_bytes: usize,
        ) -> Result<Vec<u8>, RuntimeTransportError> {
            Err(RuntimeTransportError::NotFound)
        }

        async fn atomic_write(
            &self,
            _company: &str,
            _operation_id: Uuid,
            _path: &CompanyPath,
            _contents: &[u8],
            _mode: u32,
        ) -> Result<(), RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn rename(
            &self,
            _company: &str,
            _operation_id: Uuid,
            _source: &CompanyPath,
            _destination: &CompanyPath,
        ) -> Result<(), RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn digest(
            &self,
            _company: &str,
            _path: &CompanyPath,
        ) -> Result<[u8; 32], RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn open_service(
            &self,
            _company: &str,
            _operation_id: Uuid,
            _service: RuntimeService,
            _idle_timeout: std::time::Duration,
        ) -> Result<RuntimeDuplex, RuntimeTransportError> {
            Err(RuntimeTransportError::Unavailable)
        }

        async fn activity(&self, _company: &str) -> Result<RuntimeActivity, RuntimeTransportError> {
            Ok(RuntimeActivity {
                observed_at: Utc::now(),
                active_processes: self.active_processes.lock().unwrap().clone(),
                open_service_streams: 0,
            })
        }
    }

    fn effect_authority(phase: RuntimeEffectPhase) -> RuntimeProcessAuthority {
        RuntimeProcessAuthority::GovernedEffect {
            company: "company".into(),
            actor: "exec".into(),
            effect_class: "customer-contact.email".into(),
            authority_id: 71,
            idempotency_key: "welcome-1".into(),
            execution_no: 1,
            staging_id: Uuid::new_v4(),
            phase,
        }
    }

    #[tokio::test]
    async fn network_effect_path_uses_transport_for_staging_execution_cleanup_and_redacts() {
        let transport = NetworkEffectTransport::default();
        let authority = effect_authority(RuntimeEffectPhase::WorkspaceAccess);
        ensure_effect_workspace_access(&transport, &authority)
            .await
            .unwrap();
        let artifact = "/company/outputs/proposal.pdf".to_owned();
        let (argv, staging) = stage_declared_artifacts(
            &transport,
            &authority,
            &["sender".into(), artifact.clone()],
            std::slice::from_ref(&artifact),
        )
        .await
        .unwrap();
        assert_ne!(argv[1], artifact);
        let secrets = BTreeMap::from([("PROVIDER_TOKEN".into(), "custody-sentinel".into())]);
        let mut execute_authority = authority;
        set_effect_phase(&mut execute_authority, RuntimeEffectPhase::Execute).unwrap();
        let output = run_child(
            &transport,
            &execute_authority,
            "/company",
            argv,
            staging,
            &secrets,
        )
        .await
        .unwrap();
        assert_eq!(
            redact(&String::from_utf8_lossy(&output.stdout), secrets.values()),
            "[REDACTED]"
        );
        assert_eq!(
            redact(&String::from_utf8_lossy(&output.stderr), secrets.values()),
            "[REDACTED]"
        );

        let specifications = transport.specifications.lock().unwrap();
        let phases = specifications
            .iter()
            .filter_map(|specification| match &specification.authority {
                RuntimeProcessAuthority::GovernedEffect { phase, .. } => Some(*phase),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            phases,
            vec![
                RuntimeEffectPhase::WorkspaceAccess,
                RuntimeEffectPhase::ArtifactStage,
                RuntimeEffectPhase::ArtifactStage,
                RuntimeEffectPhase::Execute,
                RuntimeEffectPhase::ArtifactCleanup,
            ]
        );
        assert!(specifications.iter().all(|specification| {
            specification.executable != "docker"
                && !specification
                    .arguments
                    .iter()
                    .any(|value| value == "docker")
        }));
        let execution = specifications
            .iter()
            .find(|specification| {
                matches!(
                    &specification.authority,
                    RuntimeProcessAuthority::GovernedEffect {
                        phase: RuntimeEffectPhase::Execute,
                        ..
                    }
                )
            })
            .unwrap();
        let debug = format!("{:?}", execution.environment);
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("custody-sentinel"));
        assert!(execution
            .environment
            .iter()
            .all(|variable| variable.sensitive));
    }

    #[tokio::test]
    async fn network_orphan_recovery_targets_only_exact_governed_effect_authority() {
        let transport = Arc::new(NetworkEffectTransport::default());
        let authority = effect_authority(RuntimeEffectPhase::Execute);
        transport
            .active_processes
            .lock()
            .unwrap()
            .push(RuntimeActiveProcess {
                process_id: Uuid::new_v4(),
                pid: 4242,
                authority: authority.clone(),
                started_at: Utc::now(),
            });
        let root = std::env::temp_dir().join(format!("restless-effect-sweep-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        std::fs::write(root.join("companies/company.toml"), "name = \"company\"\n").unwrap();
        let erased: Arc<dyn RuntimeTransport> = transport.clone();

        sweep_transport_orphans(&root, &erased).await;

        let specifications = transport.specifications.lock().unwrap();
        assert_eq!(specifications.len(), 2);
        assert!(specifications.iter().all(|specification| matches!(
            &specification.authority,
            RuntimeProcessAuthority::GovernedEffect {
                authority_id: 71,
                phase: RuntimeEffectPhase::RecoverEffectProcess,
                ..
            }
        )));
        assert_eq!(
            specifications[0].arguments.last().map(String::as_str),
            Some("4242")
        );
        assert!(specifications.iter().all(|specification| {
            specification.executable != "docker"
                && !specification
                    .arguments
                    .iter()
                    .any(|argument| argument == "docker")
        }));
        drop(specifications);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn governance_identifiers_and_paths_are_bounded() {
        assert!(valid_identifier("customer-contact.email"));
        assert!(!valid_identifier("../email"));
        assert!(valid_env_name("RESEND_API_KEY"));
        assert!(!valid_env_name("Resend-Key"));
        assert!(valid_company_cwd("/company/worktrees/offer"));
        assert!(!valid_company_cwd("/company/../etc"));
        assert!(valid_company_path("/company/outputs/sample.pdf"));
        assert!(!valid_company_path("/company/../etc/passwd"));
        assert!(valid_staging_path(
            "/tmp/restless-effect/0123456789abcdef0123456789abcdef"
        ));
        assert!(valid_staging_path(
            "/tmp/restless-effect-stage-0123456789abcdef0123456789abcdef"
        ));
        assert!(!valid_staging_path("/tmp/restless-effect/../company"));
        assert!(is_finance_identifier("finance.payment"));
        assert!(is_finance_identifier("finance/airwallex/submit"));
        assert!(!is_finance_identifier("customer-contact.email"));
    }

    #[test]
    fn exact_secret_values_are_removed_from_receipts() {
        let secrets = BTreeMap::<String, String>::from([("TOKEN".into(), "secret-value".into())]);
        assert_eq!(
            redact("before secret-value after", secrets.values()),
            "before [REDACTED] after"
        );
    }

    #[test]
    fn git_helper_reads_ephemeral_password_without_persisting_it() {
        let helper = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../infra/company-image/git-credential-restless");
        let mut child = Command::new("sh")
            .arg(&helper)
            .arg("get")
            .env("RESTLESS_GIT_PASSWORD", "sentinel-password")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start credential helper");
        child
            .stdin
            .as_mut()
            .expect("helper stdin")
            .write_all(b"protocol=https\nhost=example.test\n\n")
            .expect("write credential request");
        let output = child.wait_with_output().expect("credential helper output");
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "password=sentinel-password\n\n"
        );

        let store = Command::new("sh")
            .arg(&helper)
            .arg("store")
            .env("RESTLESS_GIT_PASSWORD", "sentinel-password")
            .output()
            .expect("store is a no-op");
        assert!(store.status.success());
        assert!(store.stdout.is_empty());
        assert!(store.stderr.is_empty());

        let absent = Command::new("sh")
            .arg(helper)
            .arg("get")
            .output()
            .expect("missing credential probe");
        assert!(absent.status.success());
        assert!(!String::from_utf8_lossy(&absent.stdout).contains("password="));
        assert!(absent.stderr.is_empty());
    }
}
