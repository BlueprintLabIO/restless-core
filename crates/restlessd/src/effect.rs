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

use crate::gateway::GatewayHandle;
use crate::runtime::CompanyConfig;

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
    gateway: &GatewayHandle,
    org: &restless_orgintel::OrgIntel,
    capability: &str,
    args: serde_json::Value,
    key: &str,
    actor: &str,
) -> Result<Receipt> {
    if key.trim().is_empty() {
        bail!("effect request needs an idempotency key");
    }
    // Replay: a retry with a known key gets the stored receipt, never a
    // second run of the world.
    if let Some(stored) = org.find_event_body("effect", "idempotency_key", key).await? {
        let mut receipt: Receipt = serde_json::from_value(stored)
            .context("stored effect receipt is not a receipt")?;
        receipt.replayed = true;
        return Ok(receipt);
    }

    let persona_path = root
        .join("simulators")
        .join(&config.name)
        .join(format!("{capability}.md"));
    let persona = std::fs::read_to_string(&persona_path).with_context(|| {
        format!("no simulated provider for {capability} at {}", persona_path.display())
    })?;

    let prompt = format!(
        "{persona}\n\n---\n# The effect request before you now\n\
         capability: {capability}\nrequested by: {actor}\narguments:\n{args}\n\n\
         Respond with the JSON outcome object only, no prose.",
        args = serde_json::to_string_pretty(&args)?
    );
    let minted = gateway.mint_token(config, "simulator")?;
    let text = call_world_model(&minted, &config.model, &prompt).await?;
    let outcome = extract_json_object(&text)
        .with_context(|| format!("world model answered unparseably: {}", &text[..text.len().min(300)]))?;

    let receipt = Receipt {
        id: Uuid::new_v4(),
        capability: capability.to_string(),
        args_digest: format!("{:x}", sha2::Sha256::digest(args.to_string().as_bytes())),
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

/// Play the world through the company's own gateway route: one Responses
/// call, purpose-tokened and ceiling-charged like any other model use.
async fn call_world_model(
    minted: &crate::gateway::MintedToken,
    model: &str,
    prompt: &str,
) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/responses", minted.base_url_host))
        .bearer_auth(&minted.token)
        .json(&serde_json::json!({ "model": model, "input": prompt }))
        .send()
        .await
        .context("world model call")?;
    let status = response.status();
    let body: serde_json::Value = response.json().await.context("world model response body")?;
    if !status.is_success() {
        bail!("world model call failed: {status} {body}");
    }
    // OpenAI Responses shape: output[] -> message -> content[] -> output_text.
    for item in body["output"].as_array().into_iter().flatten() {
        for content in item["content"].as_array().into_iter().flatten() {
            if content["type"].as_str() == Some("output_text") {
                if let Some(text) = content["text"].as_str() {
                    return Ok(text.to_string());
                }
            }
        }
    }
    bail!("world model response had no output_text: {body}")
}

/// The persona's answer is judgement; finding its JSON envelope is
/// deterministic (frame 2, same as the termination envelope).
fn extract_json_object(text: &str) -> Option<serde_json::Value> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    serde_json::from_str(&text[start..=end]).ok()
}
