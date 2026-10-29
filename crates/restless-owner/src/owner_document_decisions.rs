//! HTTP projection of Native Document decision receipts; their effects stay in the document transaction.
use super::*;
use restless_orgintel::{DocumentDecision, PendingDocumentDecision};
pub(super) fn routes() -> Router<OwnerState> {
    Router::new()
        .route(
            "/companies/{company}/documents/{document}/reviews/{request}/pending-decision",
            post(queue_review),
        )
        .route(
            "/companies/{company}/documents/{document}/collaboration/{request}/pending-decision",
            post(queue_collaboration),
        )
        .route(
            "/companies/{company}/documents/{document}/reviews/{request}/decision-draft",
            get(draft),
        )
        .route("/companies/{company}/document-decisions", get(list))
        .route(
            "/companies/{company}/document-decisions/{id}",
            delete(cancel),
        )
        .route(
            "/companies/{company}/document-decisions/{id}/seen",
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
        [company, "document-decisions"] => !company.is_empty() && *method == Method::GET,
        [company, "document-decisions", id] => {
            !company.is_empty() && !id.is_empty() && *method == Method::DELETE
        }
        [company, "document-decisions", id, "seen"] => {
            !company.is_empty() && !id.is_empty() && *method == Method::POST
        }
        [company, "documents", document, "reviews", request, "decision-draft"] => {
            !company.is_empty()
                && !document.is_empty()
                && !request.is_empty()
                && *method == Method::GET
        }
        [company, "documents", document, kind, request, "pending-decision"] => {
            !company.is_empty()
                && !document.is_empty()
                && !request.is_empty()
                && ["reviews", "collaboration"].contains(kind)
                && *method == Method::POST
        }
        _ => false,
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    id: Uuid,
    decision: DocumentDecision,
}
fn item_id(saved: &PendingDocumentDecision) -> String {
    format!(
        "document:{}:{}",
        match saved.decision {
            DocumentDecision::AcceptReview { .. } => "review",
            DocumentDecision::CompleteCollaboration { .. } => "collaboration",
        },
        saved.request_id
    )
}
fn summary(saved: &PendingDocumentDecision) -> serde_json::Value {
    serde_json::json!({"id":saved.id,"document_id":saved.document_id,"request_id":saved.request_id,"item_id":item_id(saved),"kind":match saved.decision{DocumentDecision::AcceptReview{..}=>"accept_document_review",DocumentDecision::CompleteCollaboration{..}=>"complete_document_collaboration"},"undo_until":saved.undo_until,"state":saved.state,"problem":saved.problem})
}
fn response(mut value: serde_json::Value) -> Response<Body> {
    value["server_now"] = serde_json::json!(Utc::now());
    let mut reply = Json(value).into_response();
    reply
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    reply
}
fn error(error: restless_orgintel::DocumentError) -> Response<Body> {
    super::documents_api::document_error(error)
}

fn problem(error: restless_orgintel::DocumentError) -> String {
    match error {
        restless_orgintel::DocumentError::Invalid(message)
        | restless_orgintel::DocumentError::Conflict(message) => message,
        restless_orgintel::DocumentError::Unavailable => {
            "You can no longer decide on this document.".into()
        }
        other => {
            tracing::error!(%other,"Native Document decision could not commit");
            "The document could not be updated. Open it and try again.".into()
        }
    }
}

async fn queue_review(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, document, request)): AxumPath<(String, Uuid, Uuid)>,
    Json(input): Json<Input>,
) -> Response<Body> {
    if !matches!(input.decision, DocumentDecision::AcceptReview { .. }) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "document_decision",
            "Open the current review before accepting it.",
        );
    }
    queue(state, principal, company, document, request, input).await
}
async fn queue_collaboration(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, document, request)): AxumPath<(String, Uuid, Uuid)>,
    Json(input): Json<Input>,
) -> Response<Body> {
    if !matches!(
        input.decision,
        DocumentDecision::CompleteCollaboration { .. }
    ) {
        return api_error(
            StatusCode::BAD_REQUEST,
            "document_decision",
            "Open the current request before completing it.",
        );
    }
    queue(state, principal, company, document, request, input).await
}
async fn queue(
    state: OwnerState,
    principal: RequestPrincipal,
    company: String,
    document: Uuid,
    request: Uuid,
    input: Input,
) -> Response<Body> {
    if !super::handoff_decisions_api::owner_access_current(
        &state,
        principal.human_decision_principal(),
    ) {
        return api_error(
            StatusCode::FORBIDDEN,
            "document_decision",
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
        .queue_document_decision(
            input.id,
            document,
            request,
            principal.human_decision_principal(),
            &input.decision,
        )
        .await
    {
        Ok(saved) => {
            state.decision_wake.notify_one();
            response(summary(&saved))
        }
        Err(cause) => error(cause),
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
    match org.document_decisions_for_actor(principal.actor_id()).await {
        Ok(items) => response(
            serde_json::json!({"items":items.iter().filter(|saved|saved.principal.same_person(principal.human_decision_principal())&&(saved.state!="pending"||saved.principal==*principal.human_decision_principal())).map(summary).collect::<Vec<_>>()}),
        ),
        Err(cause) => error(cause),
    }
}
async fn draft(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, document, request)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    if !super::handoff_decisions_api::owner_access_current(
        &state,
        principal.human_decision_principal(),
    ) {
        return api_error(
            StatusCode::FORBIDDEN,
            "document_decision",
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
        .latest_document_decision_draft(document, request, principal.human_decision_principal())
        .await
    {
        Ok(saved) => response(
            serde_json::json!({"draft":saved.map(|saved|serde_json::json!({"id":saved.id,"state":saved.state,"decision":saved.decision}))}),
        ),
        Err(cause) => error(cause),
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
        return api_error(
            StatusCode::FORBIDDEN,
            "document_decision",
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
        .cancel_document_decision(id, principal.human_decision_principal())
        .await
    {
        Ok(saved) => response(summary(&saved)),
        Err(cause) => error(cause),
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
        return api_error(
            StatusCode::FORBIDDEN,
            "document_decision",
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
        .acknowledge_document_decision(id, principal.human_decision_principal())
        .await
    {
        Ok(()) => response(serde_json::json!({"seen":true})),
        Err(cause) => error(cause),
    }
}
pub(super) async fn apply_due(state: &OwnerState, org: &restless_orgintel::OrgIntel) {
    let due = match org.due_document_decisions().await {
        Ok(due) => due,
        Err(cause) => {
            tracing::warn!(%cause,"pending document decisions could not be read");
            return;
        }
    };
    for saved in due {
        let problem =
            if !super::handoff_decisions_api::owner_access_current(state, &saved.principal) {
                Some("Your account access changed before this decision was recorded.".to_string())
            } else {
                org.apply_document_decision(saved.id)
                    .await
                    .err()
                    .map(problem)
            };
        if let Some(problem) = problem {
            if let Err(cause) = org.fail_document_decision(saved.id, &problem).await {
                tracing::warn!(%cause,"document decision outcome could not be recorded");
            }
        }
    }
}
