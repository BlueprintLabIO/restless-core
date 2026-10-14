//! What the cockpit shell keeps for each person and finds for them (S65), for the owner and every
//! member over the verified company principal: Library pins, which follow a person to every
//! device, and comment search for ⌘K. A thin transport; OrgIntel validates and decides access.

use super::*;

use axum::extract::rejection::QueryRejection;
use restless_orgintel::{DocumentCommentSearchHit, DocumentError};

const DEFAULT_COMMENT_SEARCH_LIMIT: i64 = 8;

pub(super) fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    RoomApiState: FromRef<S>,
{
    Router::<S>::new()
        .route(
            "/companies/{company}/library-pins",
            get(list_pins).post(set_pin),
        )
        .route(
            "/companies/{company}/library-items",
            get(list_library_items).post(set_library_collection),
        )
        .route("/companies/{company}/comment-search", get(search_comments))
        .layer(DefaultBodyLimit::max(32 * 1024))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PinInput {
    key: String,
    pinned: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectionInput {
    keys: Vec<String>,
    /// The collection to put them in; null takes them out of any.
    collection: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommentSearchQuery {
    q: String,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CommentSearchHitView {
    #[serde(flatten)]
    hit: DocumentCommentSearchHit,
    /// Where the comment opens: its document, with the comments beside it.
    href: String,
}

fn no_store(mut response: Response<Body>) -> Response<Body> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn pins_unavailable(error: restless_orgintel::OrgIntelError) -> Response<Body> {
    match error {
        restless_orgintel::OrgIntelError::InvalidContext(message) => {
            api_error(StatusCode::UNPROCESSABLE_ENTITY, "library_pin", message)
        }
        error => {
            tracing::error!(%error, "Library pins failed");
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company collaboration is temporarily unavailable",
            )
        }
    }
}

async fn list_pins(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    no_store(match org.library_pins(principal.actor_id()).await {
        Ok(pins) => Json(serde_json::json!({ "pins": pins })).into_response(),
        Err(error) => pins_unavailable(error),
    })
}

async fn set_pin(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PinInput>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    no_store(
        match org
            .set_library_pin(principal.actor_id(), &input.key, input.pinned)
            .await
        {
            Ok(pins) => Json(serde_json::json!({ "pins": pins })).into_response(),
            Err(error) => pins_unavailable(error),
        },
    )
}

async fn list_library_items(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    no_store(
        match org
            .library_items(principal.membership_role() == "owner")
            .await
        {
            Ok(items) => Json(serde_json::json!({ "items": items })).into_response(),
            Err(error) => pins_unavailable(error),
        },
    )
}

async fn set_library_collection(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<CollectionInput>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    no_store(
        match org
            .set_library_collection(
                principal.actor_id(),
                principal.membership_role() == "owner",
                &input.keys,
                input.collection.as_deref(),
            )
            .await
        {
            Ok(items) => Json(serde_json::json!({ "items": items })).into_response(),
            Err(error) => pins_unavailable(error),
        },
    )
}

async fn search_comments(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    query: std::result::Result<Query<CommentSearchQuery>, QueryRejection>,
) -> Response<Body> {
    let Ok(Query(query)) = query else {
        return no_store(api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "comment_search",
            "send q, and optionally limit",
        ));
    };
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    no_store(
        match org
            .search_document_comments_for_actor(
                principal.actor_id(),
                &query.q,
                query.limit.unwrap_or(DEFAULT_COMMENT_SEARCH_LIMIT),
            )
            .await
        {
            Ok(hits) => Json(serde_json::json!({
                "comments": hits
                    .into_iter()
                    .map(|hit| CommentSearchHitView {
                        href: format!(
                            "/{company}/library/documents?document={}&inspect=comments&thread={}",
                            hit.document_id, hit.thread_id
                        ),
                        hit,
                    })
                    .collect::<Vec<_>>(),
            }))
            .into_response(),
            Err(DocumentError::Invalid(message)) => {
                api_error(StatusCode::UNPROCESSABLE_ENTITY, "comment_search", message)
            }
            Err(error) => {
                tracing::error!(%error, "comment search failed");
                api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "orgintel",
                    "company collaboration is temporarily unavailable",
                )
            }
        },
    )
}
