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

use anyhow::{bail, Context as _, Result};
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
    /// Who this was done TO, when the arguments name someone. Derived at
    /// request time from the args the caller already sends — not a new field
    /// the Exec must remember, because a field it must remember is a field it
    /// will forget (see: three sprint-01 runs and the spawn envelope).
    #[serde(default)]
    pub party: Option<String>,
    pub outcome: serde_json::Value,
    pub provider: String,
    pub actor: String,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
    /// True when this receipt is the stored answer to a repeated key, not a
    /// fresh effect — a retry never re-runs the world.
    pub replayed: bool,
    /// Set when this company has already completed this capability against
    /// this same party under a different key. Advisory: the effect happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_of: Option<String>,
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
    let party = party_of(&args);
    // Replay: a retry with a known key gets the stored receipt, never a
    // second run of the world. The same key with DIFFERENT args is not a
    // retry — answering it with the old receipt would silently certify an
    // effect that never happened.
    if let Some(stored) = org
        .find_event_body("effect", "idempotency_key", key)
        .await?
    {
        let mut receipt: Receipt =
            serde_json::from_value(stored).context("stored effect receipt is not a receipt")?;
        if receipt.args_digest != args_digest {
            bail!("idempotency key {key:?} was already used with different arguments");
        }
        receipt.replayed = true;
        // A replay leaves a trace of its own, deliberately NOT a second
        // `effect` event: duplicating the receipt would double-count it in
        // reconciliation, which is the opposite of the point. Without this,
        // idempotency is the one mechanism protecting against double-charging
        // that nobody can audit — a company claimed "idempotent re-run
        // verified" and the record could neither confirm nor refute it,
        // because a suppressed duplicate looked identical to one that never
        // happened.
        org.emit_event(
            "effect_replayed",
            Some(actor),
            serde_json::json!({
                "capability": capability,
                "idempotency_key": key,
                "receipt_id": receipt.id,
            }),
        )
        .await?;
        return Ok(receipt);
    }

    // Idempotency guards a repeated REQUEST; it cannot guard a repeated
    // DECISION. Observed live: the same customer was charged twice under
    // `w2-sale-greg` and `pay-p12` — two honest keys, two wakes, one party,
    // nothing to notice. This does not block: the Exec may legitimately act on
    // the same party twice. It tells the Exec, immediately, so the second time
    // is a choice rather than an accident (frame: escalate, never unlock).
    let repeat = match &party {
        Some(party) => prior_effect_on(org, capability, party, key).await?,
        None => None,
    };

    // S03-T1. Dispatch happens HERE, after idempotency and repeat-party
    // detection and before the world runs — so a real provider inherits every
    // guarantee the simulator already had, rather than growing its own. This
    // is the single line that decides whether a company acts on the world or
    // rehearses, and everything above it is identical either way (§10.8).
    let provider = crate::provider::resolve(config, capability)?;
    // S03-T5. The approval gate sits between dispatch and execution, so it
    // governs exactly the effects that can reach a person and none of the ones
    // that cannot. A refusal is an error the Exec reads immediately and can act
    // on — it names the party and the command that unblocks it, because an
    // error that cannot be acted on is a missing feature, not a message.
    if let crate::approval::Decision::NeedsOwner(reason) =
        crate::approval::check(config, org, capability, party.as_deref(), &provider).await?
    {
        org.add_actor("exec", "exec", "The Exec").await.ok();
        org.emit_event(
            "approval_required",
            Some(actor),
            serde_json::json!({
                "capability": capability,
                "party": party,
                "provider": provider.name(),
                "reason": reason,
            }),
        )
        .await?;
        org.send_message(actor, None, &format!("approval required: {reason}"))
            .await?;
        bail!("{reason}");
    }
    match provider {
        // The daemon performs it. An adapter exists because the credential must
        // stay host-side, not because adapters are the model.
        crate::provider::Provider::Resend => {
            let outcome = crate::provider::resend_send(config, &args, key).await?;
            return finish_effect(
                org,
                capability,
                args_digest,
                party,
                outcome,
                provider.name(),
                actor,
                key,
                repeat,
            )
            .await;
        }
        // S04-T3. The daemon pushes; the credential stays on this side of the
        // container boundary. Restricted to `repo.push` because this transport
        // does one thing, and a provider entry that silently applied to another
        // capability would be a catalogue growing by accident.
        crate::provider::Provider::Git => {
            if capability != "repo.push" {
                bail!(
                    "company {} maps {capability} to the git provider, which serves only repo.push",
                    config.name
                );
            }
            let outcome = crate::provider::git_push(config, &args, key).await?;
            return finish_effect(
                org,
                capability,
                args_digest,
                party,
                outcome,
                provider.name(),
                actor,
                key,
                repeat,
            )
            .await;
        }
        // The company performed it itself and is attesting to the outcome.
        // This is `authority-plane §2.2`'s general case: accountability attaches
        // to the consequence, not to the transport, so a listing published
        // through the company's own browser earns the same receipt, idempotency
        // key, party and reconciliation as an HTTP call.
        //
        // The attestation is arbitrary JSON on purpose. We do not know the shape
        // of every consequential action in the world, and a schema here would be
        // a provider catalogue wearing a different hat.
        crate::provider::Provider::SelfReported => {
            let outcome = args.get("outcome").cloned().ok_or_else(|| {
                anyhow::anyhow!(
                    "{capability} is a self-reported capability for {}: you perform it yourself, \
                     then record what happened. Include an \"outcome\" object in --args describing \
                     the result — what you did, what came back, and any identifier the other side \
                     gave you. It is recorded as YOUR attestation, not as confirmed fact.",
                    config.name
                )
            })?;
            return finish_effect(
                org,
                capability,
                args_digest,
                party,
                outcome,
                provider.name(),
                actor,
                key,
                repeat,
            )
            .await;
        }
        crate::provider::Provider::Simulated => {}
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
    let outcome = extract_json_object(&text).with_context(|| {
        format!(
            "world model answered unparseably: {}",
            &text[..text.len().min(300)]
        )
    })?;

    let receipt = Receipt {
        id: Uuid::new_v4(),
        capability: capability.to_string(),
        args_digest,
        party: party.clone(),
        outcome,
        provider: "simulated".to_string(),
        actor: actor.to_string(),
        idempotency_key: key.to_string(),
        created_at: Utc::now(),
        replayed: false,
        repeat_of: repeat.clone(),
    };
    write_receipt(org, receipt, actor, repeat, key).await
}

/// Build and record the receipt for a real provider's outcome.
///
/// Exists so the live path and the simulated path converge on **one** writer.
/// The alternative — a second receipt-writing block for real providers — is how
/// the party-repeat guard and the reconciliation ledger would quietly stop
/// applying to exactly the effects that matter most.
#[allow(clippy::too_many_arguments)]
async fn finish_effect(
    org: &restless_orgintel::OrgIntel,
    capability: &str,
    args_digest: String,
    party: Option<String>,
    outcome: serde_json::Value,
    provider: &str,
    actor: &str,
    key: &str,
    repeat: Option<String>,
) -> Result<Receipt> {
    let receipt = Receipt {
        id: Uuid::new_v4(),
        capability: capability.to_string(),
        args_digest,
        party,
        outcome,
        provider: provider.to_string(),
        actor: actor.to_string(),
        idempotency_key: key.to_string(),
        created_at: Utc::now(),
        replayed: false,
        repeat_of: repeat.clone(),
    };
    write_receipt(org, receipt, actor, repeat, key).await
}

/// Emit the receipt and, when this effect repeats a party, the advisory beside
/// it. One writer for both worlds.
async fn write_receipt(
    org: &restless_orgintel::OrgIntel,
    receipt: Receipt,
    actor: &str,
    repeat: Option<String>,
    key: &str,
) -> Result<Receipt> {
    org.emit_event("effect", Some(actor), serde_json::to_value(&receipt)?)
        .await?;
    if let Some(earlier) = repeat {
        tracing::warn!(capability = %receipt.capability, party = ?receipt.party,
            earlier_key = %earlier,
            "repeat effect on the same party under a different idempotency key");
        org.emit_event(
            "effect_repeat_party",
            Some(actor),
            serde_json::json!({
                "capability": receipt.capability,
                "party": receipt.party,
                "earlier_key": earlier,
                "this_key": key,
            }),
        )
        .await?;
    }
    Ok(receipt)
}

/// Who an effect is aimed at, read from the arguments the caller already
/// sends. Deterministic key lookup, never inference: if none of these fields
/// is present the effect simply has no party and the guard stays silent.
fn party_of(args: &serde_json::Value) -> Option<String> {
    for key in [
        "customer",
        "to",
        "party",
        "recipient",
        "email",
        "customer_email",
    ] {
        if let Some(value) = args.get(key).and_then(|value| value.as_str()) {
            let value = value.trim().to_lowercase();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// The idempotency key of an earlier SUCCESSFUL effect of the same capability
/// on the same party, if one exists under a different key.
pub(crate) async fn prior_effect_on(
    org: &restless_orgintel::OrgIntel,
    capability: &str,
    party: &str,
    key: &str,
) -> Result<Option<String>> {
    for event in org.events_of_kind("effect").await? {
        if event.body.get("capability").and_then(|v| v.as_str()) != Some(capability) {
            continue;
        }
        if event.body.get("party").and_then(|v| v.as_str()) != Some(party) {
            continue;
        }
        let earlier = event
            .body
            .get("idempotency_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if earlier == key || earlier.is_empty() {
            continue;
        }
        let failed = event
            .body
            .get("outcome")
            .is_some_and(|outcome| outcome.get("error").is_some());
        if !failed {
            return Ok(Some(earlier.to_string()));
        }
    }
    Ok(None)
}

/// Every capability this company has a simulated provider for, sorted. The
/// filenames ARE the surface — no registry to drift out of sync with them.
pub fn available_capabilities(root: &std::path::Path, company: &str) -> Vec<String> {
    let dir = root.join("simulators").join(company);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
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
    let mut args: Vec<String> = vec![
        "exec".into(),
        "-i".into(),
        "-u".into(),
        "company".into(),
        "-w".into(),
        "/company".into(),
        "-e".into(),
        "OMP_HOME=/company/home/.omp".into(),
        "-e".into(),
        format!("{}={}", auth.provider_key_env, auth.provider_key),
    ];
    // The base-URL override travels with the key, exactly as it does on the
    // agent path (`acp.rs:365`). Dropping it here was a real bug with a real
    // cost: a Kimi For Coding key authenticates against `api.kimi.com/coding/v1`
    // and 401s against the provider default, so every simulated effect failed
    // with "Invalid Authentication" while the Exec — which forwards it — worked
    // fine. Sprint 03 recorded the world model 401ing twice and read it as a
    // dead key. Two paths building the same auth, one honouring it.
    if let Some((name, value)) = &auth.provider_base_url {
        args.push("-e".into());
        args.push(format!("{name}={value}"));
    }
    args.extend([
        container,
        "omp".into(),
        "-p".into(),
        "--model".into(),
        auth.model.clone(),
        "--no-tools".into(),
        prompt.to_string(),
    ]);
    let output = tokio::process::Command::new("docker")
        .args(&args)
        .output()
        .await
        .context("spawn world-model call")?;
    if !output.status.success() {
        bail!(
            "world model call failed: {}",
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(300)
                .collect::<String>()
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
