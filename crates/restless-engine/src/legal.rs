//! Authority-owned legal identity for one operating company.
//!
//! This is intentionally one current profile, not a company-secretary graph.
//! Only fields explicitly safe for ordinary invoices, contracts and provider
//! preparation exist here; identity documents and beneficial-owner evidence
//! have no input field and stay in the provider's protected channel.

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row as _};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistrationIdentifier {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegalProfileInput {
    pub legal_name: String,
    #[serde(default)]
    pub trading_name: Option<String>,
    pub entity_type: String,
    pub jurisdiction: String,
    pub registration_identifier: RegistrationIdentifier,
    /// An owner-selected address safe to place on ordinary business output.
    /// Residential/private source addresses cannot be supplied separately.
    pub approved_business_address: String,
    #[serde(default)]
    pub invoice_email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryObservationStatus {
    Observed,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryObservation {
    pub source: String,
    pub status: RegistryObservationStatus,
    pub observed_at: DateTime<Utc>,
    /// Sanitised public facts only. Empty when the source was unavailable.
    #[serde(default)]
    pub legal_name: Option<String>,
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub jurisdiction: Option<String>,
    #[serde(default)]
    pub registration_identifier: Option<RegistrationIdentifier>,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegalProfile {
    #[serde(flatten)]
    pub safe: LegalProfileInput,
    pub owner_asserted_by: String,
    pub owner_asserted_at: DateTime<Utc>,
    #[serde(default)]
    pub registry_observation: Option<RegistryObservation>,
}

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.legal_profiles (\
           company TEXT PRIMARY KEY, body JSONB NOT NULL, updated_by TEXT NOT NULL, \
           updated_at TIMESTAMPTZ NOT NULL DEFAULT now()\
         )",
    )
    .execute(pool)
    .await
    .context("create Authority legal profiles")?;
    Ok(())
}

pub async fn set_profile(
    store: &crate::authority::AuthorityStore,
    company: &str,
    input: LegalProfileInput,
    owner: &str,
) -> Result<LegalProfile> {
    validate(&input)?;
    let now = Utc::now();
    let previous = get_profile(store, company).await?;
    let registry_observation = previous
        .and_then(|profile| {
            same_registry_subject(&profile.safe, &input).then_some(profile.registry_observation)
        })
        .flatten();
    let profile = LegalProfile {
        safe: input,
        owner_asserted_by: owner.to_string(),
        owner_asserted_at: now,
        registry_observation,
    };
    sqlx::query(
        "INSERT INTO restless_authority.legal_profiles (company,body,updated_by) \
         VALUES ($1,$2,$3) ON CONFLICT (company) DO UPDATE \
         SET body=EXCLUDED.body, updated_by=EXCLUDED.updated_by, updated_at=now()",
    )
    .bind(company)
    .bind(serde_json::to_value(&profile)?)
    .bind(owner)
    .execute(store.pool())
    .await?;
    store
        .emit(
            company,
            "legal_profile_changed",
            Some(owner),
            serde_json::json!({
                "legal_name": profile.safe.legal_name,
                "trading_name": profile.safe.trading_name,
                "entity_type": profile.safe.entity_type,
                "jurisdiction": profile.safe.jurisdiction,
                "registration_identifier": profile.safe.registration_identifier,
                "approved_business_address": profile.safe.approved_business_address,
                "invoice_email": profile.safe.invoice_email,
            }),
        )
        .await?;
    Ok(profile)
}

pub async fn record_registry_observation(
    store: &crate::authority::AuthorityStore,
    company: &str,
    observation: RegistryObservation,
) -> Result<LegalProfile> {
    if observation.source.trim().is_empty() {
        bail!("registry observation needs a source");
    }
    let mut profile = get_profile(store, company)
        .await?
        .context("record the owner-confirmed legal profile before registry observation")?;
    if observation.status == RegistryObservationStatus::Unavailable
        && (observation.legal_name.is_some()
            || observation.entity_type.is_some()
            || observation.registration_identifier.is_some())
    {
        bail!("an unavailable registry observation cannot carry observed legal facts");
    }
    profile.registry_observation = Some(observation.clone());
    sqlx::query(
        "UPDATE restless_authority.legal_profiles SET body=$2, updated_at=now() WHERE company=$1",
    )
    .bind(company)
    .bind(serde_json::to_value(&profile)?)
    .execute(store.pool())
    .await?;
    store
        .emit(
            company,
            "legal_registry_observed",
            Some("daemon"),
            serde_json::to_value(observation)?,
        )
        .await?;
    Ok(profile)
}

pub async fn get_profile(
    store: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<Option<LegalProfile>> {
    let row = sqlx::query("SELECT body FROM restless_authority.legal_profiles WHERE company=$1")
        .bind(company)
        .fetch_optional(store.pool())
        .await?;
    row.map(|row| serde_json::from_value(row.get("body")))
        .transpose()
        .context("decode Authority legal profile")
}

/// The sole Runtime/model projection. It serialises only the safe profile and
/// source metadata; no restricted identity shape exists to accidentally copy.
pub async fn safe_projection(
    store: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<Option<serde_json::Value>> {
    Ok(get_profile(store, company)
        .await?
        .map(serde_json::to_value)
        .transpose()?)
}

/// Live-probe the Australian Business Register's official ABN Lookup JSON
/// service. The service requires its own registered GUID; only public entity
/// facts are retained and an outage becomes an explicit unavailable
/// observation rather than a negative registration claim.
pub async fn probe_abr(
    store: &crate::authority::AuthorityStore,
    config: &crate::runtime::CompanyConfig,
) -> Result<LegalProfile> {
    let profile = get_profile(store, &config.name)
        .await?
        .context("record the owner-confirmed legal profile before probing ABN Lookup")?;
    if !profile.safe.jurisdiction.eq_ignore_ascii_case("AU")
        || !profile
            .safe
            .registration_identifier
            .kind
            .eq_ignore_ascii_case("ABN")
    {
        bail!("the first registry probe supports an Australian ABN profile only");
    }
    let now = Utc::now();
    let guid = match crate::credential::resolve(config, "registry.abr.guid").await {
        Ok(guid) => guid,
        Err(_) => {
            return record_registry_observation(
                store,
                &config.name,
                RegistryObservation {
                    source: "Australian Business Register / ABN Lookup".into(),
                    status: RegistryObservationStatus::Unavailable,
                    observed_at: now,
                    legal_name: None,
                    entity_type: None,
                    jurisdiction: None,
                    registration_identifier: None,
                    detail: Some("ABN Lookup credential unavailable".into()),
                },
            )
            .await;
        }
    };
    let abn: String = profile
        .safe
        .registration_identifier
        .value
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    if abn.len() != 11 {
        bail!("an Australian ABN probe needs exactly 11 digits");
    }
    let mut url = reqwest::Url::parse("https://abr.business.gov.au/json/AbnDetails.aspx")?;
    url.query_pairs_mut()
        .append_pair("abn", &abn)
        .append_pair("callback", "restless")
        .append_pair("guid", guid.trim());
    let response = match reqwest::Client::new().get(url).send().await {
        Ok(response) if response.status().is_success() => response,
        _ => {
            return record_registry_observation(
                store,
                &config.name,
                RegistryObservation {
                    source: "Australian Business Register / ABN Lookup".into(),
                    status: RegistryObservationStatus::Unavailable,
                    observed_at: now,
                    legal_name: None,
                    entity_type: None,
                    jurisdiction: None,
                    registration_identifier: None,
                    detail: Some("ABN Lookup did not return an available response".into()),
                },
            )
            .await;
        }
    };
    let parsed = match response
        .text()
        .await
        .context("read ABN Lookup response")
        .and_then(|body| parse_abr_jsonp(&body))
    {
        Ok(parsed) => parsed,
        Err(_) => {
            return record_registry_observation(
                store,
                &config.name,
                RegistryObservation {
                    source: "Australian Business Register / ABN Lookup".into(),
                    status: RegistryObservationStatus::Unavailable,
                    observed_at: now,
                    legal_name: None,
                    entity_type: None,
                    jurisdiction: None,
                    registration_identifier: None,
                    detail: Some("ABN Lookup did not return usable registry facts".into()),
                },
            )
            .await;
        }
    };
    record_registry_observation(
        store,
        &config.name,
        RegistryObservation {
            source: "Australian Business Register / ABN Lookup".into(),
            status: RegistryObservationStatus::Observed,
            observed_at: now,
            legal_name: Some(parsed.0),
            entity_type: parsed.1,
            jurisdiction: Some("AU".into()),
            registration_identifier: Some(RegistrationIdentifier {
                kind: "ABN".into(),
                value: abn,
            }),
            detail: None,
        },
    )
    .await
}

fn parse_abr_jsonp(value: &str) -> Result<(String, Option<String>)> {
    let json = value
        .trim()
        .strip_prefix("restless(")
        .and_then(|value| value.strip_suffix(')'))
        .context("ABN Lookup returned an unexpected JSONP envelope")?;
    let body: serde_json::Value = serde_json::from_str(json).context("decode ABN Lookup JSON")?;
    if body
        .get("Message")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|message| !message.trim().is_empty())
    {
        bail!("ABN Lookup did not confirm the supplied identifier");
    }
    let legal_name = body
        .get("EntityName")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("ABN Lookup response omitted EntityName")?
        .to_string();
    let entity_type = body
        .get("EntityTypeName")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    Ok((legal_name, entity_type))
}

fn validate(input: &LegalProfileInput) -> Result<()> {
    for (label, value, max) in [
        ("legal name", input.legal_name.as_str(), 240),
        ("entity type", input.entity_type.as_str(), 80),
        ("jurisdiction", input.jurisdiction.as_str(), 80),
        (
            "registration identifier type",
            input.registration_identifier.kind.as_str(),
            40,
        ),
        (
            "registration identifier value",
            input.registration_identifier.value.as_str(),
            80,
        ),
        (
            "approved business address",
            input.approved_business_address.as_str(),
            500,
        ),
    ] {
        if value.trim().is_empty() || value.chars().count() > max {
            bail!("{label} must contain between 1 and {max} characters");
        }
    }
    for (label, value, max) in [
        ("trading name", input.trading_name.as_deref(), 240),
        ("invoice email", input.invoice_email.as_deref(), 320),
    ] {
        if value.is_some_and(|value| value.trim().is_empty() || value.chars().count() > max) {
            bail!("{label} must be absent or contain between 1 and {max} characters");
        }
    }
    Ok(())
}

fn same_registry_subject(previous: &LegalProfileInput, next: &LegalProfileInput) -> bool {
    previous
        .jurisdiction
        .eq_ignore_ascii_case(&next.jurisdiction)
        && previous
            .registration_identifier
            .kind
            .eq_ignore_ascii_case(&next.registration_identifier.kind)
        && previous
            .registration_identifier
            .value
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .flat_map(char::to_uppercase)
            .eq(next
                .registration_identifier
                .value
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .flat_map(char::to_uppercase))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_input_has_no_raw_identity_document_or_private_owner_field() {
        let input: LegalProfileInput = serde_json::from_value(serde_json::json!({
            "legal_name": "Example Pty Ltd",
            "entity_type": "Australian private company",
            "jurisdiction": "AU",
            "registration_identifier": {"kind": "ABN", "value": "00 000 000 000"},
            "approved_business_address": "Level 1, Example Street, Sydney NSW"
        }))
        .unwrap();
        validate(&input).unwrap();
        let safe = serde_json::to_value(input).unwrap().to_string();
        for forbidden in ["passport", "licence", "beneficial_owner", "residential"] {
            assert!(!safe.contains(forbidden));
        }
    }

    #[test]
    fn abr_jsonp_accepts_only_the_named_callback_and_public_entity_facts() {
        let (name, kind) = parse_abr_jsonp(
            r#"restless({"EntityName":"Example Pty Ltd","EntityTypeName":"Australian Private Company","Message":""})"#,
        )
        .unwrap();
        assert_eq!(name, "Example Pty Ltd");
        assert_eq!(kind.as_deref(), Some("Australian Private Company"));
        assert!(parse_abr_jsonp(r#"other({"EntityName":"Wrong callback"})"#).is_err());
        assert!(parse_abr_jsonp(r#"restless({"Message":"No matching ABN"})"#).is_err());
    }

    #[test]
    fn a_registry_observation_survives_formatting_not_an_identity_change() {
        let first = LegalProfileInput {
            legal_name: "Example Pty Ltd".into(),
            trading_name: None,
            entity_type: "company".into(),
            jurisdiction: "AU".into(),
            registration_identifier: RegistrationIdentifier {
                kind: "ABN".into(),
                value: "12 345 678 901".into(),
            },
            approved_business_address: "Business address".into(),
            invoice_email: None,
        };
        assert!(same_registry_subject(
            &first,
            &LegalProfileInput {
                legal_name: "Example Trading Pty Ltd".into(),
                registration_identifier: RegistrationIdentifier {
                    kind: "abn".into(),
                    value: "12345678901".into(),
                },
                ..first.clone()
            }
        ));
        assert!(!same_registry_subject(
            &first,
            &LegalProfileInput {
                registration_identifier: RegistrationIdentifier {
                    kind: "ABN".into(),
                    value: "98 765 432 109".into(),
                },
                ..first.clone()
            }
        ));
    }
}
