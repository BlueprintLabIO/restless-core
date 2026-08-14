//! Real providers behind the effect surface (S03-T1).
//!
//! Sprint 01/02 had exactly one world: a model playing a persona. That was the
//! right shape to build against — `ARCHITECTURE.md §10.8` claims the
//! company-side path is identical for a simulated and a real provider — but the
//! claim was never tested, because there was nothing real to test it with.
//!
//! Three rules this module exists to hold:
//!
//! * **Dispatch is per `(company, capability)`.** A company with no entry keeps
//!   the simulator. That is what makes `aris_test` structurally safe (S03-T7):
//!   the failure mode of a mistake is a simulated send, because the table has
//!   no real entry to find, not because someone remembered a rule.
//! * **The adapter runs host-side.** The provider credential is read in the
//!   daemon at the point of use and never crosses into a company container
//!   (`authority-plane §2.6`, `company-runtime §11.5`). The agent's path is
//!   unchanged: it still calls `restless effect email.send`.
//! * **Our idempotency key is the provider's.** Resend accepts an
//!   `Idempotency-Key` header, so a retry that reaches the provider is
//!   deduplicated there as well as here. Two layers, because the failure we are
//!   guarding is "the daemon died between sending and writing the receipt" —
//!   which our own replay check cannot see.

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;

use crate::runtime::CompanyConfig;

/// Which provider serves one capability for one company.
///
/// `Simulated` is not a fallback that happens when configuration is missing —
/// it is the default world, and a company only leaves it by an explicit entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Simulated,
    Resend,
}

impl Provider {
    /// The string that lands in the receipt. This is the value
    /// `evaluation-dogfood` reads to tell a real outcome from a rehearsed one,
    /// so it is the provider's own name, never a category.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Simulated => "simulated",
            Self::Resend => "resend",
        }
    }
}

/// Resolve the provider for one capability.
///
/// Unknown provider names are a hard error rather than a silent fall back to
/// the simulator: a typo in `email.send = "resned"` must not quietly send
/// nothing while reporting success. Failing closed here is the whole point of
/// the table.
pub fn resolve(config: &CompanyConfig, capability: &str) -> Result<Provider> {
    match config.providers.get(capability) {
        None => Ok(Provider::Simulated),
        Some(name) => match name.as_str() {
            "simulated" => Ok(Provider::Simulated),
            "resend" => Ok(Provider::Resend),
            other => bail!(
                "company {} maps {capability} to unknown provider {other:?}; \
                 known providers are: simulated, resend",
                config.name
            ),
        },
    }
}

/// What a real send needs, parsed from the same `args` the simulator receives.
/// The Exec's request shape does not change when a company goes live — that is
/// §10.8's claim, and this struct is where it is either true or false.
#[derive(Debug, Deserialize)]
struct EmailArgs {
    to: String,
    subject: String,
    /// Plain-text body. Accepts `body` or `text`, because both appear in the
    /// personas the Exec has been writing against for two sprints and breaking
    /// its habit on the first live send would be a gratuitous failure.
    #[serde(alias = "text")]
    body: String,
    /// Optional display name for the sender; the address itself is owner
    /// configuration, never the agent's choice.
    #[serde(default)]
    from_name: Option<String>,
}

/// Send one email through Resend, host-side.
///
/// Returns the outcome object that becomes the receipt's `outcome`. Shape is
/// deliberately close to the simulator's so downstream reconciliation
/// (`reconcile::outcome_of`) reads both without special cases.
pub async fn resend_send(
    config: &CompanyConfig,
    args: &serde_json::Value,
    idempotency_key: &str,
) -> Result<serde_json::Value> {
    let parsed: EmailArgs = serde_json::from_value(args.clone()).context(
        "email.send needs {\"to\", \"subject\", \"body\"} — \
         the same arguments the simulator takes",
    )?;
    let api_key = crate::credential::resolve(config, "email.send")
        .context("resolving the Resend credential")?;
    let from_address = config
        .from_address
        .as_deref()
        .context("company config needs from_address to send real email")?;
    let from = match &parsed.from_name {
        Some(name) => format!("{name} <{from_address}>"),
        None => from_address.to_string(),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        // Our key becomes the provider's, so a retry is deduplicated on both
        // sides of the boundary.
        .header("Idempotency-Key", idempotency_key)
        .json(&serde_json::json!({
            "from": from,
            "to": [parsed.to],
            "subject": parsed.subject,
            "text": parsed.body,
        }))
        .send()
        .await
        .context("POST https://api.resend.com/emails")?;

    let status = response.status();
    let body: serde_json::Value = response
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({ "note": "provider returned a non-JSON body" }));

    if !status.is_success() {
        // Honest status, per cross-layer §4.7: a provider refusal is reported
        // as a failure with the provider's own words, never smoothed into a
        // success or paraphrased into our vocabulary.
        return Ok(serde_json::json!({
            "status": "failed",
            "http_status": status.as_u16(),
            "provider_error": body,
            "note": format!("Resend refused the send with HTTP {}", status.as_u16()),
        }));
    }

    Ok(serde_json::json!({
        "status": "sent",
        "provider_message_id": body.get("id"),
        "to": parsed.to,
        "note": "accepted by Resend for delivery; \
                 delivery itself is a later webhook, not this receipt",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(entries: &[(&str, &str)]) -> CompanyConfig {
        let mut providers = std::collections::BTreeMap::new();
        for (capability, provider) in entries {
            providers.insert((*capability).to_string(), (*provider).to_string());
        }
        CompanyConfig {
            name: "aris".to_string(),
            mission: String::new(),
            spend_ceiling_usd: 30.0,
            org_mode: crate::runtime::OrgMode::default(),
            model: "moonshot/kimi-k3".to_string(),
            providers,
            from_address: None,
            credentials: std::collections::BTreeMap::new(),
        }
    }

    /// The structural guarantee S03-T7 rests on: a company with no entry for a
    /// capability cannot reach a real provider, so a `_test` company's worst
    /// case is a simulated send. This is not a rule someone must remember — it
    /// is the absence of a table row.
    #[test]
    fn no_entry_means_simulated() {
        let config = config_with(&[]);
        assert_eq!(resolve(&config, "email.send").unwrap(), Provider::Simulated);
        // And a live company's OTHER capabilities stay simulated too.
        let live = config_with(&[("email.send", "resend")]);
        assert_eq!(resolve(&live, "email.send").unwrap(), Provider::Resend);
        assert_eq!(resolve(&live, "web.deploy").unwrap(), Provider::Simulated);
    }

    /// A typo must not silently simulate while the run reports success. The
    /// whole value of the receipt is that `provider` tells you which world you
    /// were in; a misspelling that falls back would make it lie.
    #[test]
    fn an_unknown_provider_fails_closed() {
        let config = config_with(&[("email.send", "resned")]);
        let error = resolve(&config, "email.send").unwrap_err().to_string();
        assert!(error.contains("resned"), "{error}");
        assert!(error.contains("simulated, resend"), "{error}");
    }

    /// §10.8's claim in a test: the arguments the simulator has been taking for
    /// two sprints parse for the real adapter unchanged.
    #[test]
    fn the_simulator_argument_shape_parses_for_the_real_provider() {
        let args = serde_json::json!({
            "to": "yaillives@gmail.com",
            "subject": "Your free 11+ practice paper",
            "body": "Hello — here is the sample you asked for."
        });
        let parsed: EmailArgs = serde_json::from_value(args).expect("simulator shape must parse");
        assert_eq!(parsed.to, "yaillives@gmail.com");
        // And the `text` alias, which the personas also use.
        let aliased = serde_json::json!({ "to": "a@b.c", "subject": "s", "text": "t" });
        assert!(serde_json::from_value::<EmailArgs>(aliased).is_ok());
    }
}
