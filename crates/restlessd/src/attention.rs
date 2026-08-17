//! Owner attention is a projection, not another workflow database.
//!
//! Authority already owns first-contact grants and OrgIntel already owns
//! explicit owner handoffs. This module composes those source rows into the
//! common envelope the CLI and owner SPA render.  Rebuilding this value after
//! a daemon restart is therefore sufficient; nothing in here can resolve an
//! item by itself.

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::Result;
use chrono::{DateTime, Utc};
use restless_orgintel::OrgIntel;
use serde::Serialize;

use crate::approval;
use crate::runtime::{self, CompanyConfig, ContainerStatus};

#[derive(Debug, Clone, Serialize)]
pub struct AttentionView {
    pub company: CompanySummary,
    pub source_health: SourceHealth,
    /// Same repeatable-read Work graph returned by `restless work graph`.
    /// The owner surface maps it but never writes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_graph: Option<restless_orgintel::WorkGraphSnapshot>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_id: Option<uuid::Uuid>,
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
    pub responsible_actor: Option<AttentionActorRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_attach: Option<RuntimeAttachRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_target: Option<ReviewTargetRef>,
    pub actions: Vec<AttentionAction>,
    pub can_continue: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttentionActorRef {
    pub id: String,
    pub display: String,
    pub role: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requesting_actor_display: Option<String>,
    pub kind: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewTargetRef {
    pub company: String,
    pub generation: String,
    pub uri: String,
    pub status: &'static str,
    pub kind: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttentionAction {
    pub id: &'static str,
    pub label: &'static str,
    pub consequence: &'static str,
}

/// Compose the live queue from its owners. Authority remains independently
/// readable when OrgIntel is degraded; the latter contributes explicit owner handoffs only
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

    // One outstanding request per exact effect class+party. Repeated blocked
    // effect attempts refresh the evidence without multiplying owner work.
    let mut latest: BTreeMap<(String, String), &crate::authority::AuthorityRecord> =
        BTreeMap::new();
    for event in &approvals {
        // Pre-Sprint-5 approval rows did not preserve an executable prepared
        // command. They remain Authority history, but projecting `null` as an
        // actionable decision would ask the owner to approve an unknowable
        // effect. A fresh real effect attempt will create a current row.
        if !has_reviewable_prepared_command(&event.body) {
            continue;
        }
        let capability = event
            .body
            .get("effect_class")
            .or_else(|| event.body.get("capability"))
            .and_then(serde_json::Value::as_str)
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

    let actors = match org {
        Some(org) => org
            .list_actors()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|actor| {
                (
                    actor.id.clone(),
                    AttentionActorRef {
                        id: actor.id,
                        display: actor.display,
                        role: actor.kind,
                    },
                )
            })
            .collect::<HashMap<_, _>>(),
        None => HashMap::new(),
    };
    let generation = runtime::generation(&config.name).await.ok().flatten();
    let attach_for = |actor: Option<&str>| {
        generation.as_ref().map(|generation| RuntimeAttachRef {
            company: config.name.clone(),
            generation: generation.clone(),
            requesting_actor: actor.map(str::to_string),
            requesting_actor_display: actor
                .and_then(|actor| actors.get(actor))
                .map(|actor| actor.display.clone()),
            kind: "persistent-browser",
        })
    };

    let mut items = Vec::new();
    for ((capability, party), event) in latest {
        let prepared_by = event.actor_id.as_deref().unwrap_or("a company actor");
        let command = event
            .body
            .get("prepared_command")
            .cloned()
            .unwrap_or_default();
        let evidence = vec![AttentionEvidence {
            label: "Prepared command".into(),
            uri: None,
            content: Some(command.to_string()),
            kind: "command",
        }];
        items.push(AttentionItem {
            id: format!("authority:approval:{}:{}", capability, party),
            work_id: None,
            source: AttentionSource {
                plane: "authority",
                kind: "approval_required".into(),
                reference: event.id.to_string(),
            },
            category: "approval".into(),
            title: format!("First contact: {party}"),
            what_happened: format!(
                "{prepared_by} prepared an ordinary runtime command and stopped before first contact."
            ),
            why_it_matters: format!(
                "This is the first real {capability} effect to {party}; it carries the owner's reputation."
            ),
            recommendation: "Review the exact recipient and draft, then grant or decline this party.".into(),
            requested_action: format!("Allow or decline first contact with {party}."),
            if_no_action: "Nothing is sent. The company may continue work that does not depend on this contact.".into(),
            evidence,
            responsible_actor: None,
            runtime_attach: None,
            review_target: None,
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

    let (work_graph, orgintel_health) = match org {
        Some(org) => match org.work_graph_snapshot().await {
            Ok(graph) => (Some(graph), "available".to_string()),
            Err(error) => {
                tracing::warn!(company = %config.name, "OrgIntel unavailable to attention projection: {error}");
                (None, "unavailable".to_string())
            }
        },
        None => (None, "unavailable".to_string()),
    };
    let work = work_graph
        .as_ref()
        .map(|graph| graph.work.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|item| (item.id, item))
        .collect::<HashMap<_, _>>();
    for handoff in work_graph
        .as_ref()
        .map(|graph| graph.handoffs.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|handoff| handoff.state == restless_orgintel::OwnerHandoffState::Pending)
        // The owner queue is judgement nobody below them owes (S06-T5). A
        // handoff assigned to a team lead is that lead's queue and does not
        // consume owner attention; it arrives here only if the lead escalates
        // it, and then `escalated_from` says who tried first and why.
        //
        // This is a filter on *whose queue*, never on whether the owner may
        // look: `restless attention --all` and the lead's own queue both remain
        // readable. Judgement is delegated, not hidden.
        .filter(|handoff| handoff.assigned_to.is_none())
    {
        let Some(item) = work.get(&handoff.work_id) else {
            continue;
        };
        let mut evidence = Vec::new();
        let mut seen = HashSet::new();
        let mut runtime_review_url = None;
        for artifact in work_graph
            .as_ref()
            .into_iter()
            .flat_map(|graph| &graph.artifacts)
            .filter(|artifact| {
                artifact.work_id == Some(handoff.work_id)
                    && artifact.state == restless_orgintel::ArtifactRefState::Available
            })
        {
            if seen.insert(artifact.uri.clone()) {
                let is_url = is_url(&artifact.uri);
                let runtime_local = is_runtime_local_url(&artifact.uri);
                if runtime_local {
                    runtime_review_url = Some(artifact.uri.clone());
                }
                evidence.push(AttentionEvidence {
                    label: artifact.label.clone(),
                    uri: (is_url && !runtime_local).then(|| artifact.uri.clone()),
                    content: if runtime_local {
                        Some(format!(
                            "Live inside the company computer: {}",
                            artifact.uri
                        ))
                    } else {
                        (!is_url).then(|| artifact.uri.clone())
                    },
                    kind: if runtime_local {
                        "runtime-url"
                    } else if is_url {
                        "url"
                    } else {
                        "artifact"
                    },
                });
            }
        }
        for uri in extract_urls(&handoff.prepared_state) {
            let runtime_local = is_runtime_local_url(&uri);
            if runtime_local {
                runtime_review_url = Some(uri.clone());
            }
            if seen.insert(uri.clone()) {
                evidence.push(AttentionEvidence {
                    label: if runtime_local {
                        "Prepared browser route".into()
                    } else {
                        evidence_label(&uri)
                    },
                    uri: (!runtime_local).then(|| uri.clone()),
                    content: runtime_local
                        .then(|| format!("Open inside the persistent company browser: {uri}")),
                    kind: if runtime_local { "runtime-url" } else { "url" },
                });
            }
        }
        let judgement = handoff.category == restless_orgintel::OwnerHandoffCategory::OwnerJudgement;
        let responsible_actor = actors.get(&item.owner_id).cloned().or_else(|| {
            Some(AttentionActorRef {
                id: item.owner_id.clone(),
                display: title_case(&item.owner_id),
                role: "work lead".into(),
            })
        });
        let runtime_attach = attach_for(Some(&item.owner_id));
        let review_target = if judgement {
            match (generation.as_ref(), runtime_review_url) {
                (Some(generation), Some(uri)) if runtime::runtime_http_target(&uri).is_ok() => {
                    let status = if runtime::probe_runtime_http(&config.name, &uri)
                        .await
                        .is_ok()
                    {
                        "available"
                    } else {
                        "unavailable"
                    };
                    Some(ReviewTargetRef {
                        company: config.name.clone(),
                        generation: generation.clone(),
                        uri,
                        status,
                        kind: "runtime-web",
                        label: "Live website",
                    })
                }
                _ => None,
            }
        } else {
            None
        };
        let mut actions = if judgement {
            vec![
                AttentionAction {
                    id: "accept-review",
                    label: "Accept outcome",
                    consequence: "Accepts this exact outcome and completes the Work.",
                },
                AttentionAction {
                    id: "request-revision",
                    label: "Request changes",
                    consequence: "Sends exact feedback to the lead and starts a new Work revision.",
                },
                AttentionAction {
                    id: "chat-lead",
                    label: "Talk with lead",
                    consequence: "Opens a Work-scoped conversation without deciding the review.",
                },
            ]
        } else {
            Vec::new()
        };
        if review_target.is_some() || (!judgement && runtime_attach.is_some()) {
            actions.insert(
                0,
                AttentionAction {
                    id: "open-outcome",
                    label: if judgement {
                        "Review live outcome"
                    } else {
                        "Open prepared browser"
                    },
                    consequence: if judgement {
                        "Opens the real outcome without deciding or approving anything."
                    } else {
                        "Opens the prepared company browser without deciding or approving anything."
                    },
                },
            );
        }
        items.push(AttentionItem {
            id: format!("orgintel:handoff:{}", handoff.id),
            work_id: Some(handoff.work_id),
            source: AttentionSource {
                plane: "orgintel",
                kind: "owner_handoff".into(),
                reference: handoff.id.to_string(),
            },
            category: if judgement { "review" } else { "handoff" }.into(),
            title: item.title.clone(),
            what_happened: handoff.prepared_state.clone(),
            why_it_matters: match (&handoff.escalated_from, judgement) {
                // An escalation reached the owner because someone below them
                // could not settle it. Saying who tried, and why they stopped,
                // is what keeps a lead from being a silent filter.
                (Some(from), _) => format!(
                    "{} could not settle this and passed it up: {}",
                    actors.get(from).map_or(from.as_str(), |actor| actor.display.as_str()),
                    if handoff.resolution.trim().is_empty() {
                        "no reason recorded"
                    } else {
                        handoff.resolution.trim()
                    }
                ),
                (None, true) => "The lead has prepared the final outcome. Your judgement decides whether it ships or returns for revision.".into(),
                (None, false) => format!("This exact {:?} step cannot be performed by the company actor.", handoff.category),
            },
            recommendation: if judgement {
                "Inspect the independent evidence and make the bounded owner decision.".into()
            } else {
                "Take the prepared last mile, then release control; the company observes the resume condition itself.".into()
            },
            requested_action: handoff.requested_action.clone(),
            if_no_action: format!("Work remains blocked until: {}", handoff.resume_condition),
            evidence,
            responsible_actor,
            runtime_attach,
            review_target,
            actions,
            can_continue: false,
            created_at: handoff.created_at,
        });
    }

    items.sort_by_key(|item| {
        let priority = match item.category.as_str() {
            "review" => 0,
            "handoff" => 1,
            _ => 2,
        };
        (priority, Reverse(item.created_at))
    });
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
        work_graph,
        items,
        refreshed_at: Utc::now(),
    })
}

fn normalize_party(value: &str) -> String {
    value.trim().to_lowercase()
}

fn has_reviewable_prepared_command(body: &serde_json::Value) -> bool {
    body.get("prepared_command")
        .is_some_and(serde_json::Value::is_object)
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

fn is_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

fn is_runtime_local_url(value: &str) -> bool {
    value.starts_with("http://127.0.0.1:")
        || value.starts_with("https://127.0.0.1:")
        || value.starts_with("http://localhost:")
        || value.starts_with("https://localhost:")
}

fn evidence_label(uri: &str) -> String {
    if uri.contains("compare") {
        "Review code change".into()
    } else {
        "Open evidence".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_current_approvals_with_exact_commands_are_actionable() {
        assert!(has_reviewable_prepared_command(&serde_json::json!({
            "prepared_command": {"argv": ["resend", "send"], "effect_class": "customer-contact.email"}
        })));
        assert!(!has_reviewable_prepared_command(&serde_json::json!({
            "prepared_command": null
        })));
        assert!(!has_reviewable_prepared_command(&serde_json::json!({
            "party": "old@example.com"
        })));
    }
}
