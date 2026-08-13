//! The effect surface (sprint 01 T8): `request_effect(capability, args,
//! idempotency_key) -> Receipt`. External effects are a kernel concern
//! (§3.2); this sprint the grant check, capability check and approval gate
//! are deliberately ABSENT (accepted risk, sprint risk register) — what
//! exists is the interface shape they will govern, plus receipts.
//!
//! Providers: a trait, with `Simulated` now and `Http` later; the company-
//! side code path is identical either way (§10.8). A simulator is a
//! MODEL-DRIVEN WORLD, not a system: the persona file at
//! `$RESTLESS_HOME/simulators/<company>/<capability>.md` is a prompt, and
//! the daemon plays the world through the gateway (the company's own spend
//! ceiling pays for its world). Because we author the world, the personas
//! carry the adversarialness — a compliant world teaches nothing (T8).

use std::path::Path;

use anyhow::{Context as _, Result, bail};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::Digest as _;
use uuid::Uuid;

use crate::runtime::{self, CompanyConfig};

/// What the requester gets back, and what the event stream keeps (kind
/// "effect"). The args digest — not the args — is the receipt's record of
/// what was asked; the args themselves are operational detail.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Receipt {
    pub id: Uuid,
    pub capability: String,
    pub args_digest: String,
    pub outcome: serde_json::Value,
    pub provider: String,
    pub actor: String,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
    /// True when this receipt is the stored answer to a repeated key, not a
    /// fresh effect — a retry never re-runs the world.
    pub replayed: bool,
}

/// One effect request. The company-side path here is what a real provider
/// would take; only the world behind it is simulated this sprint.
pub async fn request_effect(
    root: &Path,
    config: &CompanyConfig,
    org: &restless_orgintel::OrgIntel,
    capability: &str,
    args: serde_json::Value,
    key: &str,
    actor: &str,
) -> Result<Receipt> {
    if key.trim().is_empty() {
        bail!("effect request needs an idempotency key");
    }
    // Capabilities are identifiers, not path fragments — one names a persona
    // file below, and this channel is reachable from inside containers.
    // Deterministic input validation; the governance sprint's capability
    // check will subsume it.
    if !valid_capability(capability) {
        bail!("invalid capability name {capability:?}");
    }
    let args_digest = format!("{:x}", sha2::Sha256::digest(args.to_string().as_bytes()));
    // Replay: a retry with a known key gets the stored receipt, never a
    // second run of the world. The same key with DIFFERENT args is not a
    // retry — answering it with the old receipt would silently certify an
    // effect that never happened.
    if let Some(stored) = org.find_event_body("effect", "idempotency_key", key).await? {
        let mut receipt: Receipt = serde_json::from_value(stored)
            .context("stored effect receipt is not a receipt")?;
        if receipt.args_digest != args_digest {
            bail!("idempotency key {key:?} was already used with different arguments");
        }
        receipt.replayed = true;
        return Ok(receipt);
    }

    let persona_path = root
        .join("simulators")
        .join(&config.name)
        .join(format!("{capability}.md"));
    // Discoverability is the difference between "probe, never guess" and
    // brute force. Aris's first run guessed ~95 capability names against a
    // surface of three and blocked on the owner; the surface knew the answer
    // the whole time and did not say it. An error that cannot be acted on is
    // a missing feature, not a message.
    let persona = std::fs::read_to_string(&persona_path).map_err(|_| {
        let available = available_capabilities(root, &config.name);
        if available.is_empty() {
            anyhow::anyhow!(
                "no simulated providers are configured for {} at all — the owner must add \
                 personas before any external effect can run",
                config.name
            )
        } else {
            anyhow::anyhow!(
                "no simulated provider for {capability}. {} offers exactly: {}",
                config.name,
                available.join(", ")
            )
        }
    })?;

    let prompt = format!(
        "{persona}\n\n---\n# The effect request before you now\n\
         capability: {capability}\nrequested by: {actor}\narguments:\n{args}\n\n\
         Respond with the JSON outcome object only, no prose.",
        args = serde_json::to_string_pretty(&args)?
    );
    let text = call_world_model(config, &prompt).await?;
    let outcome = extract_json_object(&text)
        .with_context(|| format!("world model answered unparseably: {}", &text[..text.len().min(300)]))?;

    let receipt = Receipt {
        id: Uuid::new_v4(),
        capability: capability.to_string(),
        args_digest,
        outcome,
        provider: "simulated".to_string(),
        actor: actor.to_string(),
        idempotency_key: key.to_string(),
        created_at: Utc::now(),
        replayed: false,
    };
    org.emit_event("effect", Some(actor), serde_json::to_value(&receipt)?)
        .await?;
    Ok(receipt)
}

/// Every capability this company has a simulated provider for, sorted. The
/// filenames ARE the surface — no registry to drift out of sync with them.
pub fn available_capabilities(root: &std::path::Path, company: &str) -> Vec<String> {
    let dir = root.join("simulators").join(company);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension()?.to_str()? == "md")
                .then(|| path.file_stem()?.to_str().map(str::to_owned))?
        })
        .collect();
    names.sort();
    names
}

/// Play the world as a model, through the same runtime the agents use. This
/// used to POST to the embedded gateway's /responses route; that path died
/// with the codex runtime, so every simulated effect would have failed at the
/// first call. The world now runs on omp in the company's own container.
async fn call_world_model(config: &CompanyConfig, prompt: &str) -> Result<String> {
    let auth = crate::exec::agent_auth(config)?;
    let container = runtime::container_name(&config.name);
    let output = tokio::process::Command::new("docker")
        .args([
            "exec", "-i", "-u", "company", "-w", "/company",
            "-e", "OMP_HOME=/company/home/.omp",
            "-e", &format!("{}={}", auth.provider_key_env, auth.provider_key),
            &container,
            "omp", "-p", "--model", &auth.model, "--no-tools", prompt,
        ])
        .output()
        .await
        .context("spawn world-model call")?;
    if !output.status.success() {
        bail!(
            "world model call failed: {}",
            String::from_utf8_lossy(&output.stderr).chars().take(300).collect::<String>()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The persona's answer is judgement; finding its JSON envelope is
/// deterministic (frame 2, same as the termination envelope).
fn extract_json_object(text: &str) -> Option<serde_json::Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    serde_json::from_str(&text[start..=end]).ok()
}

/// Capability names look like `web.deploy` — lowercase dotted identifiers.
/// Anything else (path separators, traversal, upcase) is not a capability.
fn valid_capability(capability: &str) -> bool {
    !capability.is_empty()
        && capability.len() <= 64
        && capability
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_'))
        && !capability.contains("..")
}

#[cfg(test)]
mod tests {
    /// The capability string becomes a file path component; the boundary
    /// check is what keeps it an identifier.
    #[test]
    fn capability_names_are_identifiers_not_paths() {
        assert!(super::valid_capability("web.deploy"));
        assert!(super::valid_capability("email.send"));
        assert!(super::valid_capability("payments.charge"));
        assert!(!super::valid_capability("../secrets/key"));
        assert!(!super::valid_capability("web/../../etc/passwd"));
        assert!(!super::valid_capability(".."));
        assert!(!super::valid_capability(""));
        assert!(!super::valid_capability("Web.Deploy"));
    }
}
