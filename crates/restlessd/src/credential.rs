//! Credential indirection (S03-T4).
//!
//! Sprint 01 put the model key into the company container via `docker exec -e`
//! and sprint 02 kept it there, both times as a named, accepted regression. The
//! first live provider is where that stops being acceptable: a Resend key
//! inside the runtime is a key an agent can read, print, or mail.
//!
//! So consequential credentials resolve **here, in the daemon, at the point of
//! use** — never in a container, never on the volume, never in the image
//! (`authority-plane §2.6`, `company-runtime §11.5`).
//!
//! ## Why an indirection rather than just reading the env
//!
//! `authority-plane §8.2` specifies a `credential_reference`: config names
//! *which* credential a capability uses, not the secret itself. Today every
//! reference resolves from the daemon's environment, which is exactly what the
//! model key already does and is honest for one operator with two keys.
//! `§8.1` says explicitly not to adopt a secret manager "before the workload
//! requires it", and one Resend key does not.
//!
//! The indirection costs one indirection and buys the swap: pointing
//! `env:RESEND_API_KEY` at `infisical:/aris/resend` later is a config change in
//! one file, not a change to any adapter. That is the cheap-early half of a
//! boundary whose expensive-late half we have already paid twice.

use anyhow::{Context as _, Result, bail};

use crate::runtime::CompanyConfig;

/// Resolve the credential a capability needs, as a secret value.
///
/// The return is deliberately a plain `String` that callers hand straight to a
/// client and drop. It is never logged, never written to the event stream, and
/// never placed in a receipt — the receipt records that an effect happened, not
/// what authorised it.
pub fn resolve(config: &CompanyConfig, capability: &str) -> Result<String> {
    let reference = config.credentials.get(capability).with_context(|| {
        format!(
            "company {} has no credential_reference for {capability}; \
             add one under [credentials] before this capability can reach a real provider",
            config.name
        )
    })?;
    resolve_reference(reference)
}

/// Resolve one `scheme:locator` reference.
///
/// Only `env:` exists today. An unknown scheme is an error rather than a
/// fallback, for the same reason an unknown provider is: a credential that
/// silently resolves to nothing produces an authentication failure at the
/// provider, which is indistinguishable from a revoked key and sends the owner
/// looking in the wrong place. We learned that one on a Moonshot key that was
/// alive the whole time.
fn resolve_reference(reference: &str) -> Result<String> {
    let (scheme, locator) = reference.split_once(':').with_context(|| {
        format!("credential reference {reference:?} must be `scheme:locator`, e.g. env:RESEND_API_KEY")
    })?;
    match scheme {
        "env" => {
            let value = std::env::var(locator).with_context(|| {
                format!("{locator} is not set in the daemon's environment")
            })?;
            if value.trim().is_empty() {
                bail!("{locator} is set but empty");
            }
            Ok(value)
        }
        other => bail!(
            "unknown credential scheme {other:?} in {reference:?}; \
             this build resolves only `env:` (see authority-plane §8.1 on deferring a secret manager)"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(entries: &[(&str, &str)]) -> CompanyConfig {
        let mut credentials = std::collections::BTreeMap::new();
        for (capability, reference) in entries {
            credentials.insert((*capability).to_string(), (*reference).to_string());
        }
        CompanyConfig {
            name: "aris".to_string(),
            mission: String::new(),
            spend_ceiling_usd: 30.0,
            org_mode: crate::runtime::OrgMode::default(),
            model: "moonshot/kimi-k3".to_string(),
            providers: std::collections::BTreeMap::new(),
            from_address: None,
            credentials,
        }
    }

    /// An unset variable must say WHICH variable. The sprint-02 Moonshot
    /// incident cost hours because "authentication failed" was
    /// indistinguishable from a dead key, and the answer was a base URL.
    #[test]
    fn a_missing_credential_names_what_is_missing() {
        let config = config_with(&[("email.send", "env:DEFINITELY_NOT_SET_12345")]);
        let error = format!("{:#}", resolve(&config, "email.send").unwrap_err());
        assert!(error.contains("DEFINITELY_NOT_SET_12345"), "{error}");
    }

    /// A capability with no reference is not a capability that silently uses
    /// someone else's key.
    #[test]
    fn no_reference_is_an_error_not_a_default() {
        let config = config_with(&[]);
        let error = format!("{:#}", resolve(&config, "email.send").unwrap_err());
        assert!(error.contains("credential_reference"), "{error}");
    }

    /// The swap this indirection exists for: an unknown scheme fails loudly
    /// rather than resolving to nothing.
    #[test]
    fn an_unknown_scheme_fails_rather_than_resolving_empty() {
        assert!(resolve_reference("infisical:/aris/resend").is_err());
        assert!(resolve_reference("no-scheme-at-all").is_err());
    }

    /// And the happy path actually reads the environment.
    #[test]
    fn env_scheme_reads_the_daemon_environment() {
        // SAFETY: single-threaded test process, variable is test-local.
        unsafe { std::env::set_var("RESTLESS_TEST_CRED", "sk-test-value") };
        assert_eq!(resolve_reference("env:RESTLESS_TEST_CRED").unwrap(), "sk-test-value");
    }
}
