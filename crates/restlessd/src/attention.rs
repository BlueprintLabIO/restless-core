//! Owner attention is a projection, not another workflow database.
//!
//! Authority already owns first-contact grants and OrgIntel already owns
//! blocked commitments.  This module composes those source rows into the
//! common envelope the CLI and owner SPA render.  Rebuilding this value after
//! a daemon restart is therefore sufficient; nothing in here can resolve an
//! item by itself.

use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use chrono::{DateTime, Utc};
use restless_orgintel::{CommitmentState, OrgIntel};
use serde::Serialize;

use crate::approval;
use crate::runtime::{self, CompanyConfig, ContainerStatus};

#[derive(Debug, Clone, Serialize)]
pub struct AttentionView {
    pub company: CompanySummary,
    pub source_health: SourceHealth,
    pub items: Vec<AttentionItem>,
    pub refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompanySummary {
    pub id: String,
    pub name: String,
    pub mission: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceHealth {
    pub orgintel: String,
    pub authority: String,
    pub runtime: String,
    pub browser: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttentionItem {
    pub id: String,
    pub source: AttentionSource,
    pub category: String,
    pub title: String,
    pub what_happened: String,
    pub why_it_matters: String,
    pub recommendation: String,
    pub requested_action: String,
    pub if_no_action: String,
    pub evidence: Vec<AttentionEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_attach: Option<RuntimeAttachRef>,
    pub actions: Vec<AttentionAction>,
    pub can_continue: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttentionSource {
    pub plane: &'static str,
    pub kind: String,
    pub reference: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttentionEvidence {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub kind: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeAttachRef {
    pub company: String,
    pub generation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requesting_actor: Option<String>,
    pub kind: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttentionAction {
    pub id: &'static str,
    pub label: &'static str,
    pub consequence: &'static str,
}

/// Compose the live queue from its owners. Authority remains independently
/// readable when OrgIntel is degraded; the latter contributes commitments only
/// when its recoverable store answers.
pub async fn project(
    config: &CompanyConfig,
    authority: &crate::authority::AuthorityStore,
    org: Option<&OrgIntel>,
) -> Result<AttentionView> {
    let approved = approval::approved_parties(authority, &config.name).await?;
    let approvals = authority
        .records_of_kind(&config.name, "approval_required")
        .await?;
    let grants = authority
        .records_of_kind(&config.name, "approval_granted")
        .await?;
    let declines = authority
        .records_of_kind(&config.name, "approval_declined")
        .await?;

    let mut resolved_after: HashMap<String, i64> = HashMap::new();
    for event in grants.iter().chain(declines.iter()) {
        if let Some(party) = event.body.get("party").and_then(|value| value.as_str()) {
            resolved_after
                .entry(normalize_party(party))
                .and_modify(|id| *id = (*id).max(event.id))
                .or_insert(event.id);
        }
    }

    // One outstanding request per exact capability+party. Repeated blocked
    // effect attempts refresh the evidence without multiplying owner work.
    let mut latest: BTreeMap<(String, String), &crate::authority::AuthorityRecord> =
        BTreeMap::new();
    for event in &approvals {
        let capability = event.body["capability"]
            .as_str()
            .unwrap_or("external effect");
        let Some(party) = event.body["party"].as_str() else {
            continue;
        };
        let party = normalize_party(party);
        if approved.contains(&party)
            || resolved_after
                .get(&party)
                .is_some_and(|resolved| *resolved > event.id)
        {
            continue;
        }
        latest.insert((capability.to_string(), party), event);
    }

    let generation = runtime::generation(&config.name).await.ok().flatten();
    let attach = generation.as_ref().map(|generation| RuntimeAttachRef {
        company: config.name.clone(),
        generation: generation.clone(),
        requesting_actor: Some("exec".to_string()),
        kind: "persistent-browser",
    });

    let mut items = Vec::new();
    for ((capability, party), event) in latest {
        let provider = event.body["provider"].as_str().unwrap_or("real provider");
        let request = event.body.get("request").cloned().unwrap_or_default();
        let subject = request
            .get("subject")
            .and_then(|value| value.as_str())
            .unwrap_or("Prepared first contact");
        let body = request
            .get("text")
            .or_else(|| request.get("text_body"))
            .or_else(|| request.get("html"))
            .and_then(|value| value.as_str());
        let mut evidence = Vec::new();
        if let Some(body) = body {
            evidence.push(AttentionEvidence {
                label: "Exact prepared draft".into(),
                uri: None,
                content: Some(body.to_string()),
                kind: "draft",
            });
        }
        items.push(AttentionItem {
            id: format!("authority:approval:{}:{}", capability, party),
            source: AttentionSource {
                plane: "authority",
                kind: "approval_required".into(),
                reference: event.id.to_string(),
            },
            category: "approval".into(),
            title: format!("First contact: {party}"),
            what_happened: format!("The company prepared “{subject}” through {provider}."),
            why_it_matters: format!(
                "This is the first real {capability} effect to {party}; it carries the owner's reputation."
            ),
            recommendation: "Review the exact recipient and draft, then grant or decline this party.".into(),
            requested_action: format!("Allow or decline first contact with {party}."),
            if_no_action: "Nothing is sent. The company may continue work that does not depend on this contact.".into(),
            evidence,
            runtime_attach: attach.clone(),
            actions: vec![
                AttentionAction {
                    id: "grant",
                    label: "Grant first contact",
                    consequence: "Allows real first-contact effects to this exact party.",
                },
                AttentionAction {
                    id: "decline",
                    label: "Decline",
                    consequence: "Leaves this party unapproved and closes this request.",
                },
            ],
            can_continue: true,
            created_at: event.created_at,
        });
    }

    let (commitments, orgintel_health) = match org {
        Some(org) => match org.list_commitments().await {
            Ok(commitments) => (commitments, "available".to_string()),
            Err(error) => {
                tracing::warn!(company = %config.name, "OrgIntel unavailable to attention projection: {error}");
                (Vec::new(), "unavailable".to_string())
            }
        },
        None => (Vec::new(), "unavailable".to_string()),
    };
    for commitment in commitments {
        if commitment.state != CommitmentState::Blocked {
            continue;
        }
        let joined = format!("{}\n{}", commitment.body, commitment.resolution);
        let evidence = extract_urls(&joined)
            .into_iter()
            .map(|uri| AttentionEvidence {
                label: evidence_label(&uri),
                uri: Some(uri),
                content: None,
                kind: "url",
            })
            .collect();
        let review = joined.to_lowercase().contains("review")
            || joined.to_lowercase().contains("merge")
            || joined.to_lowercase().contains("compare");
        items.push(AttentionItem {
            id: format!("orgintel:commitment:{}", commitment.id),
            source: AttentionSource {
                plane: "orgintel",
                kind: "blocked_commitment".into(),
                reference: commitment.id.to_string(),
            },
            category: if review { "review" } else { "blocker" }.into(),
            title: commitment.title,
            what_happened: nonempty(&commitment.resolution, &commitment.body),
            why_it_matters: "The company cannot honestly close this commitment until the source condition changes.".into(),
            recommendation: if review {
                "Inspect the independent evidence and make the bounded owner decision.".into()
            } else {
                "Complete the prepared human step, then let the company verify the result.".into()
            },
            requested_action: if review {
                "Review the prepared change and accept or reject it at the source.".into()
            } else {
                "Take the prepared last mile in the live company browser.".into()
            },
            if_no_action: "The commitment remains blocked; closing this surface does not resolve it.".into(),
            evidence,
            runtime_attach: attach.clone(),
            actions: vec![AttentionAction {
                id: "open-browser",
                label: "Open live browser",
                consequence: "Transfers only browser input control; it does not approve an external effect.",
            }],
            can_continue: false,
            created_at: commitment.updated_at,
        });
    }

    items.sort_by_key(|item| item.created_at);
    let doctor = runtime::doctor(&config.name).await.ok();
    let (runtime_health, browser_health) = match doctor.as_ref() {
        Some(report) if report.container == ContainerStatus::Running => (
            "available".to_string(),
            report
                .browser
                .as_ref()
                .map(|health| health.status.clone())
                .unwrap_or_else(|| "unavailable".to_string()),
        ),
        Some(_) => ("unavailable".into(), "unavailable".into()),
        None => ("unknown".into(), "unknown".into()),
    };

    Ok(AttentionView {
        company: CompanySummary {
            id: config.name.clone(),
            name: title_case(&config.name),
            mission: config.mission.clone(),
            model: config.model.clone(),
        },
        source_health: SourceHealth {
            orgintel: orgintel_health,
            authority: "available".into(),
            runtime: runtime_health,
            browser: browser_health,
        },
        items,
        refreshed_at: Utc::now(),
    })
}

fn normalize_party(value: &str) -> String {
    value.trim().to_lowercase()
}

fn nonempty(first: &str, fallback: &str) -> String {
    if first.trim().is_empty() {
        fallback.trim().to_string()
    } else {
        first.trim().to_string()
    }
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn extract_urls(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|word| {
            let trimmed = word.trim_matches(|character: char| {
                matches!(character, ',' | '.' | ')' | ']' | '}' | '"' | '\'')
            });
            (trimmed.starts_with("https://") || trimmed.starts_with("http://"))
                .then(|| trimmed.to_string())
        })
        .collect()
}

fn evidence_label(uri: &str) -> String {
    if uri.contains("compare") {
        "Review code change".into()
    } else {
        "Open evidence".into()
    }
}
