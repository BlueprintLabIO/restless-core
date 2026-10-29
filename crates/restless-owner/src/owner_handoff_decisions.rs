//! Durable Undo for a person's handoff decisions. Transport and the deadline
//! worker call the same source-owned OrgIntel decision transaction.
use super::*;
use restless_orgintel::{
    HandoffDecision, HumanDecisionPrincipal, OwnerReviewVersion, PendingHandoffDecision,
};

pub(super) fn routes() -> Router<OwnerState> {
    Router::new()
        .route(
            "/companies/{company}/handoffs/{handoff}/pending-decision",
            post(queue),
        )
        .route("/companies/{company}/handoff-decisions", get(list))
        .route(
            "/companies/{company}/handoff-decisions/{decision}",
            delete(cancel),
        )
        .route(
            "/companies/{company}/handoff-decisions/{decision}/seen",
            post(acknowledge),
        )
        .layer(DefaultBodyLimit::max(128 * 1024))
}

pub(super) fn is_own_route(method: &Method, path: &str) -> bool {
    let Some(rest) = path.strip_prefix("/api/companies/") else {
        return false;
    };
    let parts: Vec<_> = rest.split('/').collect();
    match parts.as_slice() {
        [company, "handoff-decisions"] => !company.is_empty() && *method == Method::GET,
        [company, "handoff-decisions", id] => {
            !company.is_empty() && !id.is_empty() && *method == Method::DELETE
        }
        [company, "handoff-decisions", id, "seen"] => {
            !company.is_empty() && !id.is_empty() && *method == Method::POST
        }
        [company, "handoffs", id, "pending-decision"] => {
            !company.is_empty() && !id.is_empty() && *method == Method::POST
        }
        _ => false,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueueInput {
    id: Uuid,
    expected_version: OwnerReviewVersion,
    decision: HandoffDecision,
}

fn summary(saved: &PendingHandoffDecision) -> serde_json::Value {
    serde_json::json!({
        "id":saved.id, "handoff_id":saved.handoff_id,
        "item_id":format!("orgintel:handoff:{}",saved.handoff_id),
        "kind":match saved.decision {
            HandoffDecision::CompleteHumanStep => "complete_human_step",
            HandoffDecision::Skip => "skip",
            HandoffDecision::AcceptReview { .. } => "accept_review",
        },
        "undo_until":saved.undo_until,"state":saved.state,"problem":saved.problem,
    })
}
fn response(mut value: serde_json::Value) -> Response<Body> {
    if let Some(object) = value.as_object_mut() {
        object.insert("server_now".into(), serde_json::json!(Utc::now()));
    }
    let mut response = Json(value).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
fn decision_error(error: restless_orgintel::OrgIntelError) -> Response<Body> {
    let status = if matches!(error, restless_orgintel::OrgIntelError::ReviewChanged) {
        StatusCode::CONFLICT
    } else {
        StatusCode::BAD_REQUEST
    };
    api_error(status, "handoff_decision", error.plain_message())
}

pub(super) fn owner_access_current(state: &OwnerState, principal: &HumanDecisionPrincipal) -> bool {
    if principal.membership.is_some() {
        return principal.owner_account.is_none(); // OrgIntel locks and checks the exact membership tuple.
    }
    match (&principal.owner_account, state.entry.network()) {
        (Some(account), Some(network)) => network.matches_account_owner(account),
        (None, None) => principal.actor_id == "owner",
        _ => false,
    }
}

/// Held payments keep their provider-observed boundary; a delayed click must
/// not become another way to finish or skip them.
async fn check_payment_boundary(
    state: &OwnerState,
    company: &str,
    handoff: Uuid,
    decision: &HandoffDecision,
) -> Result<(), String> {
    if matches!(decision, HandoffDecision::AcceptReview { .. }) {
        return Ok(());
    }
    let payments = finance::payments(&state.daemon.authority, company)
        .await
        .map_err(|_| "Payment status is temporarily unavailable. Try again.".to_string())?;
    if payments
        .iter()
        .any(|payment| payment.request.owner_handoff_id == handoff)
    {
        return Err("This payment finishes when its provider confirms it.".into());
    }
    Ok(())
}

async fn queue(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, handoff)): AxumPath<(String, Uuid)>,
    Json(input): Json<QueueInput>,
) -> Response<Body> {
    if input.expected_version.handoff.id != handoff {
        return api_error(
            StatusCode::CONFLICT,
            "handoff_decision",
            "Open the current request before deciding.",
        );
    }
    let identity = principal.human_decision_principal();
    if !owner_access_current(&state, identity) {
        return api_error(
            StatusCode::FORBIDDEN,
            "handoff_decision",
            "Your access changed. Open the company again.",
        );
    }
    if let Err(problem) = check_payment_boundary(&state, &company, handoff, &input.decision).await {
        return api_error(StatusCode::BAD_REQUEST, "handoff_decision", problem);
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
    match org
        .queue_handoff_decision(input.id, identity, &input.expected_version, &input.decision)
        .await
    {
        Ok(saved) => {
            state.decision_wake.notify_one();
            response(summary(&saved))
        }
        Err(error) => decision_error(error),
    }
}

async fn list(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
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
    match org.handoff_decisions_for_actor(principal.actor_id()).await {
        Ok(items) => response(serde_json::json!({"items":items.iter()
            .filter(|saved|saved.principal.same_person(principal.human_decision_principal()) && (saved.state!="pending" || saved.principal==*principal.human_decision_principal()))
            .map(summary).collect::<Vec<_>>()})),
        Err(error) => decision_error(error),
    }
}

async fn cancel(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, id)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    if !owner_access_current(&state, principal.human_decision_principal()) {
        return api_error(
            StatusCode::FORBIDDEN,
            "handoff_decision",
            "Your access changed. Open the company again.",
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
    match org
        .cancel_handoff_decision(id, principal.human_decision_principal())
        .await
    {
        Ok(saved) => response(summary(&saved)),
        Err(error) => decision_error(error),
    }
}

/// Joined to the gateway future, so stopping/restarting the gateway stops the
/// worker. Its next start discovers every original deadline from PostgreSQL.
pub(super) async fn run(state: OwnerState) -> std::io::Result<()> {
    // Subscribe before the first durable read. Local queue saves wake us even
    // before a newly created company's shared PostgreSQL listener is ready.
    let mut wakes = state.daemon.cell_wakes.subscribe();
    loop {
        let mut until = tokio::time::Instant::now() + Duration::from_secs(2);
        let companies = match crate::daemon::configured_companies(&state.daemon.root) {
            Ok(companies) => companies,
            Err(error) => {
                tracing::warn!(%error,"pending handoff decisions could not list companies");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        state.daemon.cell_wakes.retain_companies(&companies);
        for company in companies {
            if let Ok(url) = state.daemon.orgintel.cell_database_url(&company).await {
                state.daemon.cell_wakes.ensure_company(&company, &url);
            }
            let org = match state.daemon.orgintel.get(&company).await {
                Ok(org) => org,
                Err(_) => continue,
            };
            super::document_decisions_api::apply_due(&state, &org).await;
            let due = match org.due_handoff_decisions().await {
                Ok(due) => due,
                Err(error) => {
                    tracing::warn!(%error,"pending handoff decisions could not be read");
                    continue;
                }
            };
            for saved in due {
                let problem = if !owner_access_current(&state, &saved.principal) {
                    Some(
                        "Your account access changed before this decision was recorded."
                            .to_string(),
                    )
                } else if let Err(problem) =
                    check_payment_boundary(&state, &company, saved.handoff_id, &saved.decision)
                        .await
                {
                    Some(problem)
                } else {
                    org.apply_handoff_decision(saved.id)
                        .await
                        .err()
                        .map(|error| error.plain_message())
                };
                if let Some(problem) = problem {
                    if let Err(error) = org.fail_handoff_decision(saved.id, &problem).await {
                        tracing::warn!(%error,"a pending handoff decision could not save its refusal");
                    }
                }
            }
            match org.next_human_decision_delay().await {
                Ok(Some(next)) => {
                    until =
                        until.min(tokio::time::Instant::now() + next.max(Duration::from_secs(1)))
                }
                Ok(None) => {}
                Err(error) => tracing::warn!(%error, "pending decision deadline could not be read"),
            }
        }
        // Hints shorten this sleep; durable rows and their original deadlines
        // remain authoritative. The bounded repair also covers lost hints.
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(until) => break,
                _ = state.decision_wake.notified() => break,
                wake = wakes.recv() => match wake {
                    Ok(wake) if matches!(wake.kind.as_deref(),
                        Some("handoff_decision_changed" | "document_decision_changed")) => break,
                    Ok(_) => {},
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(()),
                },
            }
        }
    }
}

async fn acknowledge(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, id)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    if !owner_access_current(&state, principal.human_decision_principal()) {
        return api_error(
            StatusCode::FORBIDDEN,
            "handoff_decision",
            "Your access changed. Open the company again.",
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
    match org
        .acknowledge_handoff_decision(id, principal.human_decision_principal())
        .await
    {
        Ok(()) => response(serde_json::json!({"seen":true})),
        Err(error) => decision_error(error),
    }
}
