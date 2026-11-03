//! Durable Undo for an Inbox Approve or Decline. Authority holds the window;
//! the deadline worker makes the ordinary Authority decision after it, with
//! the same owner/deputy check the direct route makes, rechecked at commit.
use super::*;
use restless_engine::approval_decisions::{
    self, ApprovalDecision, ApprovalTarget, PendingApprovalDecision,
};
use restless_orgintel::HumanDecisionPrincipal;

pub(super) fn routes() -> Router<OwnerState> {
    Router::new()
        .route(
            "/companies/{company}/approvals/pending-decision",
            post(queue),
        )
        .route("/companies/{company}/approval-decisions", get(list))
        .route(
            "/companies/{company}/approval-decisions/{decision}",
            delete(cancel),
        )
        .route(
            "/companies/{company}/approval-decisions/{decision}/seen",
            post(acknowledge),
        )
        .layer(DefaultBodyLimit::max(64 * 1024))
}

/// Saving is the deputy's route too (its handler checks); reading, undoing and
/// acknowledging are each person's own receipts.
pub(super) fn is_own_route(method: &Method, path: &str) -> bool {
    let Some(rest) = path.strip_prefix("/api/companies/") else {
        return false;
    };
    let parts: Vec<_> = rest.split('/').collect();
    match parts.as_slice() {
        [company, "approval-decisions"] => !company.is_empty() && *method == Method::GET,
        [company, "approval-decisions", id] => {
            !company.is_empty() && !id.is_empty() && *method == Method::DELETE
        }
        [company, "approval-decisions", id, "seen"] => {
            !company.is_empty() && !id.is_empty() && *method == Method::POST
        }
        [company, "approvals", "pending-decision"] => {
            !company.is_empty() && *method == Method::POST
        }
        _ => false,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueueInput {
    id: Uuid,
    item_id: String,
    target: ApprovalTarget,
    decision: ApprovalDecision,
}

/// Who may give each answer: the same rule its direct route applies.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Decider {
    /// First contact and held calls: the Authority owner, or a deputy while they are away.
    OwnerOrDeputy,
    /// Standing permissions, publications and single emails: the current Authority owner only.
    AuthorityOwner,
    /// Payees: the company's owner.
    CompanyOwner,
    /// The owner's own accounts: the account owner's session only.
    AccountOwner,
}

fn decider(target: &ApprovalTarget) -> Decider {
    match target {
        ApprovalTarget::Party { .. } | ApprovalTarget::ToolCall { .. } => Decider::OwnerOrDeputy,
        ApprovalTarget::Mandate { .. }
        | ApprovalTarget::Publication { .. }
        | ApprovalTarget::EmailSend { .. } => Decider::AuthorityOwner,
        ApprovalTarget::Payee { .. } => Decider::CompanyOwner,
        ApprovalTarget::AccountNotice { .. } | ApprovalTarget::AccountRequest { .. } => {
            Decider::AccountOwner
        }
    }
}

fn summary(saved: &PendingApprovalDecision) -> serde_json::Value {
    serde_json::json!({
        "id": saved.id,
        "item_id": saved.item_id,
        "kind": saved.receipt_kind(),
        "undo_until": saved.undo_until,
        "state": saved.state,
        "problem": saved.problem,
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

fn refused(message: impl Into<String>) -> Response<Body> {
    api_error(StatusCode::BAD_REQUEST, "approval_decision", message.into())
}

fn access_changed() -> Response<Body> {
    api_error(
        StatusCode::FORBIDDEN,
        "approval_decision",
        "Your access changed. Open the company again.",
    )
}

async fn queue(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<QueueInput>,
) -> Response<Body> {
    let standing = input.decision == (ApprovalDecision::Grant { always: true });
    match decider(&input.target) {
        Decider::OwnerOrDeputy => {
            if let Err(refusal) =
                require_authority_owner_or_deputy(&state, &company, &principal, standing).await
            {
                return refusal;
            }
        }
        Decider::AuthorityOwner => {
            if let Err(refusal) = require_authority_owner(&state, &company, &principal).await {
                return refusal;
            }
        }
        Decider::CompanyOwner if principal.membership_role() != "owner" => {
            return api_error(
                StatusCode::FORBIDDEN,
                "approval_decision",
                "Only the owner can answer this.",
            )
        }
        Decider::AccountOwner if !principal.is_account_owner() => {
            return api_error(
                StatusCode::FORBIDDEN,
                "approval_decision",
                "Only the account owner answers for their accounts.",
            )
        }
        Decider::CompanyOwner | Decider::AccountOwner => {}
    }
    let identity = principal.human_decision_principal();
    if !super::handoff_decisions_api::owner_access_current(&state, identity) {
        return access_changed();
    }
    if let Err(error) = runtime::CompanyConfig::load(&state.daemon.root, &company) {
        return api_error(
            StatusCode::NOT_FOUND,
            "company",
            logged(error, "This company could not be found."),
        );
    }
    if let Ok(org) = state.daemon.orgintel.get(&company).await {
        if org.check_decision_principal(identity).await.is_err() {
            return access_changed();
        }
    }
    match approval_decisions::queue(
        &state.daemon.authority,
        input.id,
        &company,
        &input.item_id,
        identity,
        &input.target,
        &input.decision,
    )
    .await
    {
        Ok(saved) => {
            state.decision_wake.notify_one();
            response(summary(&saved))
        }
        Err(error) => refused(refusal_or_logged(
            error,
            "That decision could not be saved. Try again.",
        )),
    }
}

async fn list(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    match approval_decisions::for_actor(&state.daemon.authority, &company, principal.actor_id())
        .await
    {
        Ok(items) => {
            let identity = principal.human_decision_principal();
            response(serde_json::json!({"items": items
                .iter()
                .filter(|saved| saved.principal.same_person(identity)
                    && (saved.state != "pending" || saved.principal == *identity))
                .map(summary)
                .collect::<Vec<_>>()}))
        }
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "approval_decision",
            logged(error, "Decisions could not be read. Try again."),
        ),
    }
}

async fn cancel(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, id)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    if !super::handoff_decisions_api::owner_access_current(
        &state,
        principal.human_decision_principal(),
    ) {
        return access_changed();
    }
    match approval_decisions::cancel(
        &state.daemon.authority,
        &company,
        id,
        principal.human_decision_principal(),
    )
    .await
    {
        Ok(saved) => response(summary(&saved)),
        Err(error) => refused(refusal_or_logged(
            error,
            "That decision could not be undone. Try again.",
        )),
    }
}

async fn acknowledge(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, id)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    if !super::handoff_decisions_api::owner_access_current(
        &state,
        principal.human_decision_principal(),
    ) {
        return access_changed();
    }
    match approval_decisions::acknowledge(
        &state.daemon.authority,
        &company,
        id,
        principal.human_decision_principal(),
    )
    .await
    {
        Ok(()) => response(serde_json::json!({"seen": true})),
        Err(error) => refused(refusal_or_logged(
            error,
            "That result could not be marked as seen. Try again.",
        )),
    }
}

/// Whether this person may still make this decision now. `Err` is a
/// transient read failure: the row stays pending and is tried again.
async fn refusal(
    state: &OwnerState,
    saved: &PendingApprovalDecision,
    org: Option<&restless_orgintel::OrgIntel>,
) -> Result<Option<String>, ()> {
    let principal: &HumanDecisionPrincipal = &saved.principal;
    if !super::handoff_decisions_api::owner_access_current(state, principal) {
        return Ok(Some(
            "Your account access changed before this decision was recorded.".into(),
        ));
    }
    if let Some(org) = org {
        if let Err(error) = org.check_decision_principal(principal).await {
            return Ok(Some(error.plain_message()));
        }
    }
    let role = principal
        .membership
        .as_ref()
        .map_or("owner", |membership| membership.membership_role.as_str());
    match decider(&saved.target) {
        Decider::CompanyOwner => {
            return Ok((role != "owner").then(|| "Only the owner can answer this now.".into()))
        }
        // An account owner's session has no membership; access was rechecked above.
        Decider::AccountOwner => {
            return Ok(principal
                .membership
                .is_some()
                .then(|| "Only the account owner answers for their accounts.".into()))
        }
        Decider::AuthorityOwner => {
            return match effective_authority_owner(state, &saved.company, org).await {
                Ok(owner) if owner.actor_id == principal.actor_id => Ok(None),
                Ok(_) => Ok(Some(
                    "Only the company's owner may make this decision now.".into(),
                )),
                Err(_) => Err(()),
            }
        }
        Decider::OwnerOrDeputy => {}
    }
    let standing = saved.decision == (ApprovalDecision::Grant { always: true });
    if restless_engine::away::deputy_may_decide(
        &state.daemon.authority,
        &saved.company,
        role,
        &principal.actor_id,
        standing,
    )
    .await
    {
        return Ok(None);
    }
    match effective_authority_owner(state, &saved.company, org).await {
        Ok(owner) if owner.actor_id == principal.actor_id => Ok(None),
        Ok(_) => Ok(Some(
            "Only the company's owner may make this decision now.".into(),
        )),
        Err(_) => Err(()),
    }
}

/// Make every decision whose Undo window has ended. Returns the delay to the
/// next deadline, so the decision worker sleeps until it.
pub(super) async fn apply_due(state: &OwnerState) -> Option<Duration> {
    let authority = &state.daemon.authority;
    match approval_decisions::due(authority).await {
        Ok(due) => {
            for id in due {
                let claim = match approval_decisions::claim(authority, id).await {
                    Ok(Some(claim)) => claim,
                    Ok(None) => continue,
                    Err(error) => {
                        tracing::warn!(%error, "a pending approval decision could not be claimed");
                        continue;
                    }
                };
                let company = claim.decision().company.clone();
                let org = state.daemon.orgintel.get(&company).await.ok();
                // A publication starts or stops through the company; without it, try again.
                if org.is_none()
                    && matches!(claim.decision().target, ApprovalTarget::Publication { .. })
                {
                    continue;
                }
                // A transient failure drops the claim; the row stays pending.
                let Ok(refusal) = refusal(state, claim.decision(), org.as_ref()).await else {
                    continue;
                };
                let effects = approval_decisions::Effects {
                    root: &state.daemon.root,
                    authority,
                    org: org.as_ref(),
                    publication: Some(&state.daemon.publication),
                };
                if let Err(error) = claim.settle(&effects, refusal).await {
                    tracing::warn!(%error, company, "a pending approval decision could not be settled");
                }
            }
        }
        Err(error) => tracing::warn!(%error, "pending approval decisions could not be read"),
    }
    match approval_decisions::next_delay(authority).await {
        Ok(next) => next,
        Err(error) => {
            tracing::warn!(%error, "pending approval deadline could not be read");
            Some(Duration::from_secs(2))
        }
    }
}
