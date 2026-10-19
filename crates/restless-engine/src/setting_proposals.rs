//! A company actor asks the owner to change a company setting (S101). The proposal is an
//! authority record; the owner answers it in its Inbox card, which applies the change through
//! the same owner API the Settings page saves with, so permissions, validation and the audit
//! trail are that page's own. Deciding records the answer; nothing changes before it.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub const SETTING_PROPOSED_KIND: &str = "setting_proposed";
pub const SETTING_DECIDED_KIND: &str = "setting_decided";

/// The settings an actor may propose, each with the exact value it asks for. A value only the
/// owner knows (an ABN, a registered address) is left out and typed in the card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "setting", rename_all = "snake_case")]
pub enum SettingChange {
    /// The company's spend limit, in US dollars, for its period.
    SpendLimit { ceiling_usd: f64 },
    /// How hard every agent thinks by default: low, medium, high, xhigh or max.
    ReasoningEffort { effort: String },
    /// The company's legal details, for invoices, contracts and registrations. Every field is
    /// optional: what the actor knows is proposed, the rest the owner fills in.
    LegalProfile {
        #[serde(default)]
        legal_name: Option<String>,
        #[serde(default)]
        trading_name: Option<String>,
        #[serde(default)]
        entity_type: Option<String>,
        #[serde(default)]
        jurisdiction: Option<String>,
        /// The registration number's kind, e.g. `abn`, `acn`, `company_number`.
        #[serde(default)]
        registration_kind: Option<String>,
        #[serde(default)]
        registration_value: Option<String>,
        #[serde(default)]
        approved_business_address: Option<String>,
        #[serde(default)]
        invoice_email: Option<String>,
    },
}

const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

impl SettingChange {
    /// One line for the card's title: what changes, to what.
    pub fn summary(&self) -> String {
        match self {
            Self::SpendLimit { ceiling_usd } => format!("Set the spend limit to US${ceiling_usd:.0}"),
            Self::ReasoningEffort { effort } => format!("Set the company's thinking effort to {effort}"),
            Self::LegalProfile { .. } => "Fill in the company's legal details".into(),
        }
    }

    fn validate(&self) -> Result<()> {
        match self {
            Self::SpendLimit { ceiling_usd } => {
                if !ceiling_usd.is_finite() || *ceiling_usd <= 0.0 || *ceiling_usd > 1_000_000.0 {
                    bail!("propose a spend limit between US$1 and US$1,000,000");
                }
            }
            Self::ReasoningEffort { effort } => {
                if !EFFORTS.contains(&effort.as_str()) {
                    bail!("propose an effort of low, medium, high, xhigh or max");
                }
            }
            Self::LegalProfile {
                legal_name,
                trading_name,
                entity_type,
                jurisdiction,
                registration_kind,
                registration_value,
                approved_business_address,
                invoice_email,
            } => {
                for value in [
                    legal_name,
                    trading_name,
                    entity_type,
                    jurisdiction,
                    registration_kind,
                    registration_value,
                    approved_business_address,
                    invoice_email,
                ]
                .into_iter()
                .flatten()
                {
                    if value.len() > 500 || value.chars().any(char::is_control) {
                        bail!("keep each legal detail to one line of at most 500 characters");
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingProposalInput {
    #[serde(flatten)]
    pub change: SettingChange,
    /// Why the company needs it, in a sentence or two.
    pub reason: String,
    #[serde(default)]
    pub work_id: Option<uuid::Uuid>,
}

/// An actor proposes a setting change; the owner answers it in the Inbox.
pub async fn propose(
    store: &crate::authority::AuthorityStore,
    company: &str,
    actor: &str,
    mut input: SettingProposalInput,
) -> Result<i64> {
    input.reason = input.reason.trim().to_string();
    if input.reason.is_empty() || input.reason.len() > 2_000 {
        bail!("say why the company needs this change (at most 2,000 characters)");
    }
    input.change.validate()?;
    let same = std::mem::discriminant(&input.change);
    if pending(store, company)
        .await?
        .iter()
        .any(|(_, waiting)| std::mem::discriminant(&waiting.change) == same)
    {
        bail!("a change to this setting is already waiting for the owner's answer");
    }
    store
        .emit(company, SETTING_PROPOSED_KIND, Some(actor), serde_json::to_value(&input)?)
        .await
}

/// Proposals the owner has not answered, by record.
pub async fn pending(
    store: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<Vec<(crate::authority::AuthorityRecord, SettingProposalInput)>> {
    let decided = store
        .records_of_kind(company, SETTING_DECIDED_KIND)
        .await?
        .into_iter()
        .filter_map(|record| record.body.get("proposal").and_then(serde_json::Value::as_i64))
        .collect::<std::collections::HashSet<_>>();
    Ok(store
        .records_of_kind(company, SETTING_PROPOSED_KIND)
        .await?
        .into_iter()
        .filter(|record| !decided.contains(&record.id))
        .filter_map(|record| {
            let input = serde_json::from_value(record.body.clone()).ok()?;
            Some((record, input))
        })
        .collect())
}

/// The owner's answer. `applied` means the card saved the change through the settings API
/// first; declining changes nothing.
pub async fn decide(
    store: &crate::authority::AuthorityStore,
    company: &str,
    owner: &str,
    proposal: i64,
    applied: bool,
    note: Option<&str>,
) -> Result<()> {
    let (_, waiting) = pending(store, company)
        .await?
        .into_iter()
        .find(|(record, _)| record.id == proposal)
        .context("this setting change was already answered or no longer exists")?;
    store
        .emit(
            company,
            SETTING_DECIDED_KIND,
            Some(owner),
            serde_json::json!({
                "proposal": proposal,
                "applied": applied,
                "setting": serde_json::to_value(&waiting.change)?.get("setting").cloned(),
                "note": note.map(str::trim).filter(|note| !note.is_empty()),
            }),
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_proposal_reads_as_its_setting_and_value() {
        let input: SettingProposalInput = serde_json::from_str(
            r#"{"setting":"spend_limit","ceiling_usd":200,"reason":"The launch doubles ad spend."}"#,
        )
        .unwrap();
        assert_eq!(input.change, SettingChange::SpendLimit { ceiling_usd: 200.0 });
        assert_eq!(input.change.summary(), "Set the spend limit to US$200");
        let legal: SettingProposalInput = serde_json::from_str(
            r#"{"setting":"legal_profile","registration_kind":"abn","reason":"The registrar needs it."}"#,
        )
        .unwrap();
        assert!(legal.change.validate().is_ok());
    }

    #[test]
    fn an_unknown_setting_or_value_is_refused() {
        assert!(
            serde_json::from_str::<SettingProposalInput>(r#"{"setting":"owner_password","value":"x","reason":"r"}"#)
                .is_err()
        );
        assert!(SettingChange::ReasoningEffort { effort: "extreme".into() }.validate().is_err());
        assert!(SettingChange::SpendLimit { ceiling_usd: -5.0 }.validate().is_err());
    }
}
