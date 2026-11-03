//! Exec's changes to the owner's Inbox, made from the owner's chat (owner decision, 3 November
//! 2026). The engine saved each one citing the owner's message; this applies it when its Undo
//! window ends, through the same route the owner's own click takes, as the owner, so every check
//! that route makes still holds (A4's "the outcome still opens" among them). Undo is the owner's.
use super::*;
use restless_engine::approval_decisions::{self, ApprovalDecision, ApprovalTarget};
use restless_orgintel::{ExecInboxAction, ExecInboxVerb, OwnerChatAnswer};

pub(super) fn routes() -> Router<OwnerState> {
    Router::new()
        .route("/companies/{company}/exec-inbox-actions", get(recent))
        .route(
            "/companies/{company}/exec-inbox-actions/{action}",
            delete(undo),
        )
}

/// Exec's changes in the last few minutes, each with its item, state and Undo deadline: what the
/// cockpit's notice and an open card's note read. One small read, so the owner hears of a change
/// within its window even while the full Inbox read is busy.
async fn recent(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if principal.membership_role() != "owner" || !principal.permits_company(&company) {
        return api_error(
            StatusCode::FORBIDDEN,
            "exec_inbox",
            "Only the owner sees what Exec did in their Inbox.",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "The company is temporarily unavailable.",
            )
        }
    };
    let actions = match org.recent_exec_inbox_actions(20).await {
        Ok(actions) => actions,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "exec_inbox",
                logged(error, "What Exec did could not be read. Try again."),
            )
        }
    };
    let items = actions
        .into_iter()
        .filter(|action| action.created_at > Utc::now() - ChronoDuration::minutes(15))
        .map(|action| {
            serde_json::json!({
                "id": action.id,
                "item_id": action.item_id,
                "title": action.item_title,
                "verb": action.verb,
                "state": action.state,
                "note": action.note,
                "undo_until": action.undo_until,
                "problem": action.problem,
                "snooze_until": action.snooze_until,
                "position": action.position,
                "source_message_id": action.source_message_id,
                "owner_words": action.owner_words,
                "at": action.created_at,
            })
        })
        .collect::<Vec<_>>();
    let mut response =
        Json(serde_json::json!({ "items": items, "server_now": Utc::now() })).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The company owner, as the route the owner's click takes sees them.
fn owner_principal(state: &OwnerState) -> RequestPrincipal {
    match state.entry.network_coordinates() {
        None => RequestPrincipal::local_owner(),
        Some((owner, _, _)) => RequestPrincipal::from_verified(&crate::entry::VerifiedIdentity {
            user: "owner".into(),
            issuer: None,
            owner: owner.to_string(),
            scope: crate::entry::CompanyScope::Owner,
            role: "owner".into(),
            actor: Some("owner".into()),
            company_id: None,
            cell_id: None,
            membership_id: None,
            membership_version: None,
            display_name: None,
        })
        .unwrap_or_else(RequestPrincipal::local_owner),
    }
}

/// Undo: the owner's, and only while the window is open.
async fn undo(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, action)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    if principal.membership_role() != "owner" || !principal.permits_company(&company) {
        return api_error(
            StatusCode::FORBIDDEN,
            "exec_inbox",
            "Only the owner can undo what Exec did in their Inbox.",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(_) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "The company is temporarily unavailable.",
            )
        }
    };
    match org.cancel_exec_inbox_action(action).await {
        Ok(saved) => {
            let mut response =
                Json(serde_json::json!({ "id": saved.id, "state": saved.state })).into_response();
            response
                .headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(error) => api_error(StatusCode::CONFLICT, "exec_inbox", error.plain_message()),
    }
}

/// The note and place each item carries in the owner's Inbox, added to the owner's Inbox read so
/// a row and its note arrive together.
pub(super) async fn decorate(org: &restless_orgintel::OrgIntel, output: &mut serde_json::Value) {
    let notes = match restless_engine::exec_inbox::notes(org).await {
        Ok(notes) => notes,
        Err(error) => {
            tracing::warn!(%error, "Exec's Inbox notes could not be read");
            return;
        }
    };
    if let Some(items) = output["items"].as_array_mut() {
        for item in items {
            let Some(id) = item["id"].as_str().map(str::to_string) else {
                continue;
            };
            if let Some(note) = notes.notes.get(&id) {
                item["exec_action"] = note.clone();
            }
            if let Some(position) = notes.positions.get(&id) {
                item["exec_position"] = serde_json::json!(position);
            }
        }
    }
}

/// Apply every Exec action in this company whose Undo window has ended.
pub(super) async fn apply_due(
    state: &OwnerState,
    company: &str,
    org: &restless_orgintel::OrgIntel,
) {
    let due = match org.due_exec_inbox_actions().await {
        Ok(due) => due,
        Err(error) => {
            tracing::warn!(%error, company, "Exec's pending Inbox changes could not be read");
            return;
        }
    };
    for action in due {
        let claim = match org.claim_exec_inbox_action(action.id).await {
            Ok(Some(claim)) => claim,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(%error, company, "an Exec Inbox change could not be claimed");
                continue;
            }
        };
        let problem = apply(state, company, org, claim.action()).await.err();
        if let Some(problem) = &problem {
            tracing::info!(company, action = %action.id, problem, "an Exec Inbox change was refused");
        }
        if let Err(error) = claim.settle(problem).await {
            tracing::warn!(%error, company, "an Exec Inbox change could not be settled");
        }
    }
}

/// The plain refusal a route returned, or `Ok` when it succeeded.
async fn outcome(response: Response<Body>) -> std::result::Result<(), String> {
    if response.status().is_success() {
        return Ok(());
    }
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap_or_default();
    Err(serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|body| body["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| "That was refused.".into()))
}

/// The author hears a decline's reason where the item's own path does not carry it.
async fn tell_author(
    org: &restless_orgintel::OrgIntel,
    author: Option<&str>,
    title: &str,
    reason: &str,
) {
    let (Some(author), false) = (author, reason.trim().is_empty()) else {
        return;
    };
    if author == "owner" {
        return;
    }
    let body = format!(
        "The owner declined “{}” in their chat with Exec. Their reason: “{}”",
        title.trim(),
        reason.trim()
    );
    let _ = org
        .ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await;
    if let Err(error) = org.send_message("daemon", Some(author), &body).await {
        tracing::warn!(%error, "could not tell the author about a decline from chat");
    }
}

async fn apply(
    state: &OwnerState,
    company: &str,
    org: &restless_orgintel::OrgIntel,
    action: &ExecInboxAction,
) -> std::result::Result<(), String> {
    let verb = action
        .verb()
        .ok_or_else(|| "This change is no longer understood.".to_string())?;
    let target = &action.target;
    let kind = target["kind"].as_str().unwrap_or_default();
    let reference = target["reference"].as_str().unwrap_or_default();
    let author = target["responsible"].as_str();
    let note = action.note.trim();
    match verb {
        // The applied row is the change: the Inbox reads the place and the new item from it.
        ExecInboxVerb::Move | ExecInboxVerb::Add => Ok(()),
        ExecInboxVerb::Snooze => org
            .set_inbox_marks(
                "owner",
                &[restless_orgintel::InboxMarkChange {
                    item_id: action.item_id.clone(),
                    seen_stamp: None,
                    snoozed_until: Some(action.snooze_until),
                }],
            )
            .await
            .map_err(|error| format!("{error}")),
        ExecInboxVerb::Dismiss => set_aside(org, action, author).await,
        ExecInboxVerb::Accept | ExecInboxVerb::Decline | ExecInboxVerb::Answer => {
            decide(
                state, company, org, action, verb, kind, reference, author, note,
            )
            .await
        }
    }
}

async fn set_aside(
    org: &restless_orgintel::OrgIntel,
    action: &ExecInboxAction,
    author: Option<&str>,
) -> std::result::Result<(), String> {
    org.dismiss_attention(&action.item_id, "owner", &action.item_title, author)
        .await
        .map_err(|error| error.plain_message())
}

#[expect(
    clippy::too_many_arguments,
    reason = "each item kind's decision path takes a different slice of the saved action"
)]
async fn decide(
    state: &OwnerState,
    company: &str,
    org: &restless_orgintel::OrgIntel,
    action: &ExecInboxAction,
    verb: ExecInboxVerb,
    kind: &str,
    reference: &str,
    author: Option<&str>,
    note: &str,
) -> std::result::Result<(), String> {
    let owner = owner_principal(state);
    let yes = matches!(verb, ExecInboxVerb::Accept | ExecInboxVerb::Answer);
    match kind {
        "owner_handoff" => {
            let handoff = Uuid::parse_str(reference)
                .map_err(|_| "This request names no handoff.".to_string())?;
            let review = action.target["outcome_review"].as_bool().unwrap_or(false);
            // A4 on Exec's path: accept only an outcome that still opens, checked again now.
            if verb == ExecInboxVerb::Accept && review {
                super::review_api::outcome_still_opens(state, company, org, handoff, "owner")
                    .await?;
            }
            let answer = match (yes, review) {
                (true, _) => OwnerChatAnswer::Yes,
                (false, true) => OwnerChatAnswer::Changes,
                (false, false) => OwnerChatAnswer::No,
            };
            org.answer_handoff_from_owner_message(handoff, action.source_message_id, answer, note)
                .await
                .map_err(|error| error.plain_message())
        }
        kind if restless_engine::exec_inbox::APPROVAL_KINDS.contains(&kind) => {
            let target = match kind {
                "email_mandate_proposal" => ApprovalTarget::EmailMandate {
                    proposal: Uuid::parse_str(reference)
                        .map_err(|_| "This mandate proposal is unreadable.".to_string())?,
                },
                "payee_proposal" => ApprovalTarget::Payee {
                    proposal: reference
                        .parse()
                        .map_err(|_| "This payee proposal is unreadable.".to_string())?,
                },
                "account_notice" => ApprovalTarget::AccountNotice {
                    account: action.target["account"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    notice: reference.to_string(),
                },
                "account_request" => ApprovalTarget::AccountRequest {
                    request: reference.to_string(),
                },
                "email_send_request" => ApprovalTarget::EmailSend {
                    request: Uuid::parse_str(reference)
                        .map_err(|_| "This email request is unreadable.".to_string())?,
                },
                "publication_authorization" => ApprovalTarget::Publication {
                    publication: reference.to_string(),
                },
                _ => match (
                    action.target["call_key"].as_str(),
                    action.target["party"].as_str(),
                ) {
                    (Some(call_key), _) => ApprovalTarget::ToolCall {
                        call_key: call_key.to_string(),
                    },
                    (None, Some(party)) => ApprovalTarget::Party {
                        party: party.to_string(),
                    },
                    _ => return Err("This approval names nothing to approve.".into()),
                },
            };
            // Only these answers carry a note; the others tell the author directly.
            let carries_note = matches!(
                target,
                ApprovalTarget::Party { .. }
                    | ApprovalTarget::ToolCall { .. }
                    | ApprovalTarget::EmailMandate { .. }
                    | ApprovalTarget::Payee { .. }
                    | ApprovalTarget::EmailSend { .. }
            );
            let decision = if yes {
                ApprovalDecision::Grant { always: false }
            } else {
                ApprovalDecision::Decline {
                    note: (carries_note && !note.is_empty()).then(|| note.to_string()),
                    changes_requested: false,
                }
            };
            approve(state, company, action, &owner, &target, &decision).await?;
            if !yes && !carries_note {
                tell_author(org, author, &action.item_title, note).await;
            }
            Ok(())
        }
        "skill_candidate" => {
            let input = serde_json::from_value(serde_json::json!({
                "disposition": if yes { "accepted" } else { "retired" },
            }))
            .map_err(|_| "This know-how answer could not be formed.".to_string())?;
            outcome(
                super::skills_api::set_disposition(
                    State(state.clone()),
                    Extension(owner),
                    AxumPath((company.to_string(), reference.to_string())),
                    Json(input),
                )
                .await,
            )
            .await?;
            if !yes {
                tell_author(org, author, &action.item_title, note).await;
            }
            Ok(())
        }
        "identity_proposal" => {
            let proposal = Uuid::parse_str(reference)
                .map_err(|_| "This brand proposal is unreadable.".to_string())?;
            let words = if note.is_empty() {
                format!(
                    "The owner agreed in their chat with Exec: “{}”",
                    action.owner_words
                )
            } else {
                note.to_string()
            };
            let response = if yes {
                super::promote_company_identity(
                    State(state.clone()),
                    Extension(owner),
                    AxumPath((company.to_string(), proposal)),
                    Json(super::IdentityPromotionInput {
                        change_account: words,
                    }),
                )
                .await
                .into_response()
            } else {
                super::reject_company_identity(
                    State(state.clone()),
                    Extension(owner),
                    AxumPath((company.to_string(), proposal)),
                    Json(super::IdentityRejectionInput { rationale: words }),
                )
                .await
                .into_response()
            };
            outcome(response).await
        }
        "direction_request" => {
            let request = Uuid::parse_str(reference)
                .map_err(|_| "This request is unreadable.".to_string())?;
            outcome(
                super::direction_api::decide(
                    State(state.clone()),
                    Extension(owner),
                    AxumPath((company.to_string(), request)),
                    Json(restless_engine::direction::DirectionDecisionInput {
                        decision: if yes {
                            restless_engine::direction::DirectionDecisionKind::Approve
                        } else {
                            restless_engine::direction::DirectionDecisionKind::Decline
                        },
                        reason: (!note.is_empty()).then(|| note.to_string()),
                        always_allow: None,
                        make_team_owner: None,
                    }),
                )
                .await,
            )
            .await
        }
        "setting_proposal" if !yes => {
            let proposal: i64 = reference
                .parse()
                .map_err(|_| "This setting proposal is unreadable.".to_string())?;
            let input = serde_json::from_value(serde_json::json!({
                "applied": false,
                "note": (!note.is_empty()).then(|| note.to_string()),
            }))
            .map_err(|_| "This setting answer could not be formed.".to_string())?;
            outcome(
                super::company_settings_api::decide_setting(
                    State(state.clone()),
                    Extension(owner),
                    AxumPath((company.to_string(), proposal)),
                    Json(input),
                )
                .await,
            )
            .await
        }
        "charter_proposal" if !yes => {
            let proposal = Uuid::parse_str(reference)
                .map_err(|_| "This charter draft is unreadable.".to_string())?;
            outcome(
                super::discard_charter_proposal(
                    State(state.clone()),
                    Extension(owner),
                    AxumPath((company.to_string(), proposal)),
                )
                .await,
            )
            .await?;
            tell_author(org, author, &action.item_title, note).await;
            Ok(())
        }
        // A finished result the owner has seen: off the Inbox.
        "conversation_owner_review" if yes => set_aside(org, action, author).await,
        _ => Err("This kind of item is not decided from chat; it is answered in its card.".into()),
    }
}

/// An Authority approval, decided as the owner. Authority's own Undo row is saved already due:
/// the owner's window was this action's, so the worker decides it at once with every recheck
/// it makes for the owner's own click.
async fn approve(
    state: &OwnerState,
    company: &str,
    action: &ExecInboxAction,
    owner: &RequestPrincipal,
    target: &ApprovalTarget,
    decision: &ApprovalDecision,
) -> std::result::Result<(), String> {
    let authority = &state.daemon.authority;
    // The Exec action's id names the approval row too, so a replay after a crash finds it.
    let saved = approval_decisions::queue(
        authority,
        action.id,
        company,
        &action.item_id,
        owner.human_decision_principal(),
        target,
        decision,
    )
    .await
    .map_err(|error| refusal_or_logged(error, "That approval could not be saved."))?;
    approval_decisions::make_due(authority, saved.id)
        .await
        .map_err(|error| logged(error, "That approval could not be saved.").to_string())?;
    for _ in 0..3 {
        super::approval_decisions_api::apply_due(state).await;
        match approval_decisions::outcome(authority, saved.id).await {
            Ok(Some((state, _))) if state == "applied" => return Ok(()),
            Ok(Some((state, problem))) if state == "failed" || state == "canceled" => {
                return Err(problem.unwrap_or_else(|| "Authority refused it.".into()))
            }
            Ok(_) => tokio::time::sleep(Duration::from_millis(200)).await,
            Err(error) => {
                return Err(logged(error, "The approval's outcome could not be read.").to_string())
            }
        }
    }
    // Still held by another worker: it is due, so the decision worker settles it with the same
    // rechecks, and a refusal reaches the owner as that approval's own receipt.
    Ok(())
}
