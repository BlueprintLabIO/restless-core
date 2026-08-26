//! Minimal approval check for live sends (S03-T5).
//!
//! `authority-plane §6.4` calls approvals **rare exceptions**, and §6.5 warns
//! off a policy language. So this is a typed check with exactly one rule, and
//! the rule is chosen to be the smallest thing that makes a real send to a real
//! stranger acceptable:
//!
//! > **A real provider, a party we have never successfully reached before, and
//! > no standing approval → ask the owner. An explicit revocation re-arms that
//! > gate until the owner grants again.**
//!
//! What that deliberately is *not*: thresholds, spend tiers, per-capability
//! policy, a DSL, or an approvals table with a lifecycle. Those are the
//! Authority Kernel proper, deferred with a live trigger.
//!
//! ## Why "new party" and not "every live send"
//!
//! Approving every send makes the owner a dispatcher, which `owner-cockpit
//! §2.3` explicitly rejects — the owner is a governor. First contact is the
//! moment that carries the irreversible weight: it is where the company's
//! reputation and the owner's legal position as sender of record are actually
//! at stake. A follow-up in a conversation the owner already blessed is not a
//! new decision, and asking again would train them to click through.
//!
//! ## Why it reuses Authority receipts rather than another fact
//!
//! "Have we reached this party before?" is already answered by receipts —
//! `effect::prior_effect_on` powers the party-repeat guard. Approvals and
//! receipts now share the Authority-owned governance store; no coordination
//! projection is asked to certify whether an external act occurred.

use std::path::Path;

use anyhow::{bail, Result};

use crate::runtime::CompanyConfig;

/// The decision. Three states, because "we could not tell" must not collapse
/// into either "allowed" or "denied" — the lesson `TurnEnd` and `Outcome`
/// already paid for twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Proceed. Either this is a `_test` company, this party is not new, or
    /// the owner has standing approval on file.
    Proceed,
    /// A real, materially external first contact. Carries the sentence the
    /// owner sees.
    NeedsOwner(String),
}

/// Config approvals are accepted only as one-time migration input. Authority
/// owns the live decision; continuing to read and write this field would make
/// company config a second writer of governance truth.
pub fn legacy_config_approvals(config: &CompanyConfig) -> Vec<String> {
    config
        .approved_parties
        .iter()
        .map(|party| party.trim().to_lowercase())
        .filter(|party| !party.is_empty())
        .collect()
}

/// Purge the migration field after Authority has committed its import. This
/// is the "purge to one canon" half of the transfer: `company show` must not
/// display a stale grant after a later Authority revocation.
pub fn purge_legacy_config_approvals(root: &Path, config: &mut CompanyConfig) -> Result<()> {
    if config.approved_parties.is_empty() {
        return Ok(());
    }
    config.approved_parties.clear();
    CompanyConfig::save(root, config)
}

pub async fn grant(
    root: &Path,
    company: &str,
    party: &str,
    authority: &crate::authority::AuthorityStore,
    org: Option<&restless_orgintel::OrgIntel>,
    principal: &str,
) -> Result<String> {
    CompanyConfig::load(root, company)?;
    let party = normalize_party(party);
    if party.is_empty() {
        bail!("approval party cannot be empty");
    }
    if is_approved(authority, company, &party).await? {
        return Ok(format!("{party} was already approved for {company}"));
    }
    authority
        .emit(
            company,
            "approval_granted",
            Some("owner"),
            serde_json::json!({ "party": party, "principal": principal }),
        )
        .await?;
    announce_decisions(company, authority, org).await;
    Ok(format!("{party} approved for real effects from {company}"))
}

pub async fn revoke(
    root: &Path,
    company: &str,
    party: &str,
    authority: &crate::authority::AuthorityStore,
    org: Option<&restless_orgintel::OrgIntel>,
    principal: &str,
) -> Result<String> {
    CompanyConfig::load(root, company)?;
    let party = normalize_party(party);
    if party.is_empty() {
        bail!("approval party cannot be empty");
    }
    if !is_approved(authority, company, &party).await? {
        return Ok(format!("{party} was not approved for {company}"));
    }
    authority
        .emit(
            company,
            "approval_revoked",
            Some("owner"),
            serde_json::json!({ "party": party, "principal": principal }),
        )
        .await?;
    announce_decisions(company, authority, org).await;
    Ok(format!("{party} approval revoked for {company}"))
}

/// Declining closes the currently presented request without granting standing
/// authority. A later materially different request may ask again; there is no
/// deny-policy language hiding in this operation.
pub async fn decline(
    root: &Path,
    company: &str,
    party: &str,
    authority: &crate::authority::AuthorityStore,
    org: Option<&restless_orgintel::OrgIntel>,
    principal: &str,
) -> Result<String> {
    CompanyConfig::load(root, company)?;
    let party = normalize_party(party);
    if party.is_empty() {
        bail!("approval party cannot be empty");
    }
    authority
        .emit(
            company,
            "approval_declined",
            Some("owner"),
            serde_json::json!({ "party": party, "principal": principal }),
        )
        .await?;
    announce_decisions(company, authority, org).await;
    Ok(format!("first contact with {party} declined"))
}

/// The exact Authority records that are an answer to the company, oldest first.
const DECISION_KINDS: [&str; 3] = ["approval_granted", "approval_declined", "approval_revoked"];

/// Kind of OrgIntel event that records "this exact Authority decision has been
/// told to the company". Keying on the Authority record id is what makes the
/// reconciler idempotent and keeps Authority the only writer of the decision.
const ANNOUNCED_EVENT: &str = "authority_decision_announced";

/// Project every unannounced Authority approval decision into the company.
///
/// The owner's answer used to be durable in Authority while the company was
/// told on a best-effort path that only logged a warning — and `decline` and
/// `revoke` told the company nothing at all, so Work blocked on a question the
/// owner had already answered stayed blocked with nobody informed (S19-T2).
///
/// This is a projection, not a second writer: Authority still owns the
/// decision, and re-running this adds nothing because each announcement is
/// recorded against the exact Authority record id before the scan can repeat.
/// It is called directly by the owner action so the common case is immediate,
/// and by the scheduler so a crash between the two writes is repaired.
pub async fn announce_decisions(
    company: &str,
    authority: &crate::authority::AuthorityStore,
    org: Option<&restless_orgintel::OrgIntel>,
) -> usize {
    let Some(org) = org else {
        return 0;
    };
    if org
        .ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .is_err()
        || org
            .ensure_actor("exec", "exec", "exec", "The Exec")
            .await
            .is_err()
    {
        return 0;
    }

    let mut pending = Vec::new();
    for kind in DECISION_KINDS {
        let Ok(records) = authority.records_of_kind(company, kind).await else {
            return 0;
        };
        for record in records {
            let Some(party) = record.body.get("party").and_then(|value| value.as_str()) else {
                continue;
            };
            pending.push((record.id, kind, normalize_party(party)));
        }
    }
    pending.sort_by_key(|(id, _, _)| *id);

    let mut announced = 0;
    for (record_id, kind, party) in pending {
        match org
            .find_event_body(
                ANNOUNCED_EVENT,
                "authority_record_id",
                &record_id.to_string(),
            )
            .await
        {
            Ok(Some(_)) => continue,
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(company, "could not read authority announcements: {error:#}");
                return announced;
            }
        }
        let body = match kind {
            "approval_granted" => format!(
                "The owner approved real external effects to {party}. You may proceed with the work that was waiting on it."
            ),
            "approval_declined" => format!(
                "The owner declined first contact with {party}. Do not contact them. Work that was waiting on this needs a different route or an explicit decision to stop."
            ),
            _ => format!(
                "The owner revoked approval for real external effects to {party}. Stop any work that depends on reaching them."
            ),
        };
        // Record the announcement first. A duplicate message to the Exec is a
        // wasted turn; a duplicate *effect* is the thing that must never
        // happen, and the message itself performs none. Recording first means
        // a crash between the two loses one announcement rather than repeating
        // it forever, and the owner surface still shows the decision.
        if let Err(error) = org
            .emit_event(
                ANNOUNCED_EVENT,
                Some("owner"),
                serde_json::json!({
                    "authority_record_id": record_id.to_string(),
                    "decision": kind,
                    "party": party,
                }),
            )
            .await
        {
            tracing::warn!(
                company,
                "could not record authority announcement: {error:#}"
            );
            return announced;
        }
        if let Err(error) = org.send_message("owner", Some("exec"), &body).await {
            tracing::warn!(company, "could not announce authority decision: {error:#}");
            return announced;
        }
        announced += 1;
    }
    announced
}

fn normalize_party(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Decide whether this effect may proceed.
///
/// `party` is already derived from the args by `effect::party_of`, so this
/// never re-parses arguments — one derivation, one meaning.
pub async fn check(
    config: &CompanyConfig,
    authority: &crate::authority::AuthorityStore,
    effect_class: &str,
    party: Option<&str>,
    test_mode: bool,
) -> Result<Decision> {
    // Test companies cannot receive live secrets and exercise fake CLIs, so
    // their receipts never ask for standing real-world authority.
    if test_mode {
        return Ok(Decision::Proceed);
    }
    let Some(party) = party else {
        // A real effect with no identifiable party is not first contact with
        // anyone — e.g. a deploy. The party-repeat guard has the same silence.
        return Ok(Decision::Proceed);
    };
    let party_lower = party.trim().to_lowercase();

    if is_approved(authority, &config.name, &party_lower).await? {
        return Ok(Decision::Proceed);
    }

    // Revocation is an authority act. Without this check, any earlier receipt
    // would immediately bypass
    // the gate below and `restless approve --revoke` would have no effect on a
    // party the company had already contacted. The latest grant/revoke event
    // is enough; no approval state machine or second writer is introduced.
    let latest_grant =
        latest_party_event(authority, &config.name, "approval_granted", &party_lower).await?;
    let latest_revoke =
        latest_party_event(authority, &config.name, "approval_revoked", &party_lower).await?;
    if revocation_is_unresolved(latest_revoke, latest_grant) {
        return Ok(Decision::NeedsOwner(needs_owner_reason(
            config,
            effect_class,
            party,
        )));
    }

    // Already reached successfully? Then this is not first contact. Uses the
    // receipts, the same source the party-repeat guard reads.
    if crate::effect::prior_effect_on(authority, &config.name, effect_class, &party_lower, "")
        .await?
        .is_some()
    {
        return Ok(Decision::Proceed);
    }

    Ok(Decision::NeedsOwner(needs_owner_reason(
        config,
        effect_class,
        party,
    )))
}

fn revocation_is_unresolved(latest_revoke: Option<i64>, latest_grant: Option<i64>) -> bool {
    latest_revoke.is_some_and(|revoked| latest_grant.is_none_or(|granted| revoked > granted))
}

async fn latest_party_event(
    authority: &crate::authority::AuthorityStore,
    company: &str,
    kind: &str,
    party: &str,
) -> Result<Option<i64>> {
    Ok(authority
        .records_of_kind(company, kind)
        .await?
        .into_iter()
        .filter(|event| {
            event
                .body
                .get("party")
                .and_then(|value| value.as_str())
                .is_some_and(|candidate| normalize_party(candidate) == party)
        })
        .map(|event| event.id)
        .max())
}

pub async fn is_approved(
    authority: &crate::authority::AuthorityStore,
    company: &str,
    party: &str,
) -> Result<bool> {
    let party = normalize_party(party);
    let latest_grant = latest_party_event(authority, company, "approval_granted", &party).await?;
    let latest_revoke = latest_party_event(authority, company, "approval_revoked", &party).await?;
    Ok(latest_grant.is_some() && !revocation_is_unresolved(latest_revoke, latest_grant))
}

pub async fn approved_parties(
    authority: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<Vec<String>> {
    let mut parties = std::collections::BTreeSet::new();
    for event in authority
        .records_of_kind(company, "approval_granted")
        .await?
    {
        if let Some(party) = event.body.get("party").and_then(|value| value.as_str()) {
            parties.insert(normalize_party(party));
        }
    }
    let mut approved = Vec::new();
    for party in parties {
        if is_approved(authority, company, &party).await? {
            approved.push(party);
        }
    }
    Ok(approved)
}

fn needs_owner_reason(config: &CompanyConfig, effect_class: &str, party: &str) -> String {
    format!(
        "{} wants to perform {effect_class} for {party}, but this party does not \
         currently have owner approval. This is materially external and irreversible: approve with \
         `restless approve -c {} --party {party}`.",
        config.name, config.name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_revocation_rearms_authority_until_a_later_grant() {
        assert!(revocation_is_unresolved(Some(8), Some(7)));
        assert!(revocation_is_unresolved(Some(8), None));
        assert!(!revocation_is_unresolved(Some(8), Some(9)));
        assert!(!revocation_is_unresolved(None, Some(9)));
    }

    fn config(approved: &[&str]) -> CompanyConfig {
        CompanyConfig {
            name: "aris".to_string(),
            mission: String::new(),
            spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(30_000_000),
            model: "moonshot/kimi-k3".to_string(),
            model_failover: Vec::new(),
            credentials: std::collections::BTreeMap::new(),
            approved_parties: approved.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Legacy config import is case- and whitespace-insensitive. The live gate
    /// reads Authority, but migration must preserve the exact old intent.
    #[test]
    fn standing_approval_matches_regardless_of_case_or_padding() {
        let padded = config(&["  YaiLLives@Gmail.com  "]);
        assert!(legacy_config_approvals(&padded).contains(&"yaillives@gmail.com".to_string()));
        // Normalisation must not silently accept a DIFFERENT address: one
        // dropped character is a different person, and a gate that is loose
        // about identity is not a gate. (This assertion exists because the
        // first version of the test above had exactly that typo and passed
        // nothing useful.)
        let typo = config(&["yailives@gmail.com"]);
        assert!(!legacy_config_approvals(&typo).contains(&"yaillives@gmail.com".to_string()));
    }

    /// Every owner decision reaches the company exactly once, including the two
    /// that previously reached it never (S19-T2). The adversarial cases are the
    /// ones that mattered: repeat the reconciler, and drop an announcement the
    /// way a crash between the Authority write and the OrgIntel write would.
    #[tokio::test]
    async fn every_owner_decision_reaches_the_company_exactly_once() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping approval announcement scenario");
            return;
        };
        let company = format!("approve_{}_test", uuid::Uuid::new_v4().simple());
        let authority = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .unwrap();
        let org = restless_orgintel::OrgIntel::ensure(&database_url, &company)
            .await
            .unwrap();

        let announced_bodies = |org: restless_orgintel::OrgIntel| async move {
            org.events_of_kind(ANNOUNCED_EVENT)
                .await
                .unwrap()
                .into_iter()
                .map(|event| event.body)
                .collect::<Vec<_>>()
        };
        let exec_inbox_len = |org: restless_orgintel::OrgIntel| async move {
            org.inbox(Some("exec")).await.unwrap().len()
        };

        for kind in DECISION_KINDS {
            authority
                .emit(
                    &company,
                    kind,
                    Some("owner"),
                    serde_json::json!({ "party": "hello@example.com", "principal": "owner" }),
                )
                .await
                .unwrap();
        }

        assert_eq!(
            announce_decisions(&company, &authority, Some(&org)).await,
            3,
            "grant, decline and revoke each reach the company"
        );
        assert_eq!(exec_inbox_len(org.clone()).await, 3);

        // Idempotent: the owner action and the scheduler both call this.
        assert_eq!(
            announce_decisions(&company, &authority, Some(&org)).await,
            0
        );
        assert_eq!(
            announce_decisions(&company, &authority, Some(&org)).await,
            0
        );
        assert_eq!(exec_inbox_len(org.clone()).await, 3);

        // A decision whose announcement was lost is repaired, and only that one.
        let bodies = announced_bodies(org.clone()).await;
        let dropped = bodies
            .iter()
            .find(|body| body["decision"] == "approval_declined")
            .and_then(|body| body["authority_record_id"].as_str())
            .expect("the decline was announced")
            .to_string();
        // Simulate a crash between the Authority write and the OrgIntel write.
        // A separate pool keeps this out of the production API surface.
        let scratch = sqlx::PgPool::connect(&database_url).await.unwrap();
        sqlx::query(&format!(
            "DELETE FROM {}.events WHERE kind=$1 AND body->>'authority_record_id'=$2",
            org.schema()
        ))
        .bind(ANNOUNCED_EVENT)
        .bind(&dropped)
        .execute(&scratch)
        .await
        .unwrap();
        scratch.close().await;

        assert_eq!(
            announce_decisions(&company, &authority, Some(&org)).await,
            1,
            "only the lost announcement is repaired"
        );
        assert_eq!(exec_inbox_len(org.clone()).await, 4);
        assert_eq!(
            announce_decisions(&company, &authority, Some(&org)).await,
            0
        );

        authority.delete_test_company(&company).await.unwrap();
    }
}
