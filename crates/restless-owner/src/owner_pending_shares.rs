//! Shares that wait for someone invited to the company (S66), over the verified company
//! principal. A thin transport: OrgIntel decides who may share and gives each share on entry.

use super::*;

use restless_orgintel::{DocumentError, ShareTarget};

pub(super) fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    RoomApiState: FromRef<S>,
{
    Router::<S>::new()
        .route(
            "/companies/{company}/pending-shares",
            get(list_pending_shares).post(share_with_invitee),
        )
        .route(
            "/companies/{company}/pending-shares/cancel",
            post(cancel_pending_share),
        )
        .layer(DefaultBodyLimit::max(4 * 1024))
}

/// Exactly one of a document or a sheet.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetQuery {
    document: Option<Uuid>,
    sheet: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingShareInput {
    document: Option<Uuid>,
    sheet: Option<Uuid>,
    email: String,
    #[serde(default)]
    access: Option<String>,
}

fn target(document: Option<Uuid>, sheet: Option<Uuid>) -> Result<ShareTarget, Response<Body>> {
    match (document, sheet) {
        (Some(document), None) => Ok(ShareTarget::Document(document)),
        (None, Some(sheet)) => Ok(ShareTarget::Sheet(sheet)),
        _ => Err(no_store(api_error(
            StatusCode::BAD_REQUEST,
            "pending_share_target",
            "name exactly one document or sheet",
        ))),
    }
}

fn no_store(mut response: Response<Body>) -> Response<Body> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn pending_share_error(error: DocumentError) -> Response<Body> {
    no_store(match error {
        DocumentError::Invalid(message) => {
            api_error(StatusCode::UNPROCESSABLE_ENTITY, "pending_share", message)
        }
        DocumentError::Unavailable => api_error(
            StatusCode::NOT_FOUND,
            "pending_share",
            "you cannot share this",
        ),
        error => {
            tracing::error!(%error, "pending share operation failed");
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company collaboration is temporarily unavailable",
            )
        }
    })
}

/// Who a document or sheet waits for; empty for anyone who may not share it.
async fn list_pending_shares(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<TargetQuery>,
) -> Response<Body> {
    let target = match target(query.document, query.sheet) {
        Ok(target) => target,
        Err(response) => return response,
    };
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    match org.pending_shares(target, principal.actor_id()).await {
        Ok(items) => no_store(Json(serde_json::json!({ "items": items })).into_response()),
        Err(error) => pending_share_error(error),
    }
}

/// Keeps a share for an invitee, given when they first enter.
async fn share_with_invitee(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PendingShareInput>,
) -> Response<Body> {
    let target = match target(input.document, input.sheet) {
        Ok(target) => target,
        Err(response) => return response,
    };
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    let access = input.access.as_deref().unwrap_or("edit");
    match org
        .share_with_invitee(target, principal.actor_id(), &input.email, access)
        .await
    {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => pending_share_error(error),
    }
}

/// Stops waiting for an invitee.
async fn cancel_pending_share(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PendingShareInput>,
) -> Response<Body> {
    let target = match target(input.document, input.sheet) {
        Ok(target) => target,
        Err(response) => return response,
    };
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    match org
        .cancel_pending_share(target, principal.actor_id(), &input.email)
        .await
    {
        Ok(_) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => pending_share_error(error),
    }
}
