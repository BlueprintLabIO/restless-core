//! Authenticated HTTP surface for Room Message lifecycle and search.
//!
//! This module is a thin transport over `restless_orgintel`. The verified
//! company principal supplies the acting Actor; request bodies cannot select
//! another Actor or broaden the company scope.

use super::*;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::routing::patch;

const ROOM_LIFECYCLE_BODY_LIMIT: usize = 66 * 1024;
const DEFAULT_ROOM_SEARCH_LIMIT: i64 = 30;
const DEFAULT_MESSAGE_SEARCH_LIMIT: i64 = 20;
const DEFAULT_REVISION_LIMIT: i64 = 10;
const MAX_PAGE_LIMIT: i64 = 100;
const MAX_MESSAGE_SEARCH_LIMIT: i64 = 25;
const MAX_REVISION_LIMIT: i64 = 10;
const MAX_SEARCH_QUERY_BYTES: usize = 256;
const MAX_SEARCH_SNIPPET_BYTES: usize = 480;
const MAX_MESSAGE_BODY_BYTES: usize = 64 * 1024;
const IDEMPOTENCY_KEY: &str = "idempotency-key";

pub(super) fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    RoomApiState: FromRef<S>,
{
    Router::<S>::new()
        .route("/companies/{company}/rooms/search", get(search_room_titles))
        .route(
            "/companies/{company}/room-messages/search",
            get(search_messages),
        )
        .route(
            "/companies/{company}/rooms/{room}/messages/{message}",
            patch(edit_message).delete(delete_message),
        )
        .route(
            "/companies/{company}/rooms/{room}/messages/{message}/revisions",
            get(list_message_revisions),
        )
        .layer(DefaultBodyLimit::max(ROOM_LIFECYCLE_BODY_LIMIT))
        .layer(middleware::map_response(no_store_response))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EditMessageInput {
    expected_revision_number: i64,
    body: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RevisionPageQuery {
    before_revision_number: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MessageSearchQuery {
    q: String,
    before_message_id: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoomTitleSearchQuery {
    q: String,
    before_created_at: Option<String>,
    before_room_id: Option<Uuid>,
    limit: Option<i64>,
}

/// A revision body is available exactly once in this response. The internal
/// `plain_text` search projection is intentionally not serialized as a second
/// copy of the same (potentially 64 KiB) text.
#[derive(Debug, Serialize)]
struct MessageRevisionResponse {
    id: Uuid,
    revision_number: i64,
    editor_actor_id: String,
    body: String,
    created_at: DateTime<Utc>,
}

impl From<restless_orgintel::RoomMessageRevisionRow> for MessageRevisionResponse {
    fn from(revision: restless_orgintel::RoomMessageRevisionRow) -> Self {
        Self {
            id: revision.id,
            revision_number: revision.revision_number,
            editor_actor_id: revision.editor_actor_id,
            body: revision.body,
            created_at: revision.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
struct MessageEditResponse {
    revision: MessageRevisionResponse,
    created: bool,
}

#[derive(Debug, Serialize)]
struct MessageRevisionPageResponse {
    revisions: Vec<MessageRevisionResponse>,
    next_before_revision_number: Option<i64>,
    has_more: bool,
}

#[derive(Debug, Serialize)]
struct MessageSearchResultResponse {
    id: i64,
    room_id: Uuid,
    from_actor: String,
    thread_root_message_id: Option<i64>,
    snippet: String,
}

#[derive(Debug, Serialize)]
struct MessageSearchPageResponse {
    messages: Vec<MessageSearchResultResponse>,
    next_before_message_id: Option<i64>,
    has_more: bool,
}

#[derive(Debug)]
struct RoomLifecycleFailure {
    status: StatusCode,
    error: &'static str,
    message: String,
}

impl RoomLifecycleFailure {
    fn new(status: StatusCode, error: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            error,
            message: message.into(),
        }
    }

    fn into_response(self) -> Response<Body> {
        api_error(self.status, self.error, self.message)
    }
}

fn lifecycle_json<T: Serialize>(status: StatusCode, value: T) -> Response<Body> {
    (status, Json(value)).into_response()
}

fn lifecycle_error(error: restless_orgintel::OrgIntelError) -> Response<Body> {
    match error {
        restless_orgintel::OrgIntelError::RoomAccessDenied(_) => api_error(
            StatusCode::NOT_FOUND,
            "room_message",
            "the Room Message is unavailable",
        ),
        restless_orgintel::OrgIntelError::InvalidRoom(message)
            if message == "a Message edit must change the visible body" =>
        {
            api_error(StatusCode::UNPROCESSABLE_ENTITY, "message_body", message)
        }
        // At this boundary all caller-controlled Room inputs have already
        // been validated. A remaining InvalidRoom means the addressed Room or
        // Message is unavailable. Keep it indistinguishable from denied access.
        restless_orgintel::OrgIntelError::InvalidRoom(_) => api_error(
            StatusCode::NOT_FOUND,
            "room_message",
            "the Room Message is unavailable",
        ),
        restless_orgintel::OrgIntelError::RoomCommandConflict(message) => {
            api_error(StatusCode::CONFLICT, "room_command", message)
        }
        error => {
            tracing::error!(%error, "Room lifecycle operation failed");
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company collaboration is temporarily unavailable",
            )
        }
    }
}

async fn no_store_response(mut response: Response<Body>) -> Response<Body> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn command_id(headers: &HeaderMap) -> std::result::Result<Uuid, RoomLifecycleFailure> {
    let mut values = headers.get_all(IDEMPOTENCY_KEY).iter();
    let Some(value) = values.next() else {
        return Err(RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Room Message mutations require one UUID Idempotency-Key header",
        ));
    };
    if values.next().is_some() {
        return Err(RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Room Message mutations require exactly one Idempotency-Key header",
        ));
    }
    let value = value.to_str().ok().map(str::trim).unwrap_or_default();
    let command_id = Uuid::parse_str(value).map_err(|_| {
        RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Idempotency-Key must be a non-nil UUID",
        )
    })?;
    if command_id.is_nil() {
        return Err(RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Idempotency-Key must be a non-nil UUID",
        ));
    }
    Ok(command_id)
}

fn parse_json<T>(
    input: std::result::Result<Json<T>, JsonRejection>,
) -> std::result::Result<T, RoomLifecycleFailure> {
    input.map(|Json(input)| input).map_err(|_| {
        RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "request_body",
            "the JSON body does not match the Room Message command",
        )
    })
}

fn parse_query<T>(
    query: std::result::Result<Query<T>, QueryRejection>,
) -> std::result::Result<T, RoomLifecycleFailure> {
    query.map(|Query(query)| query).map_err(|_| {
        RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "query",
            "the query parameters do not match this Room request",
        )
    })
}

fn positive_message_id(message_id: i64) -> std::result::Result<i64, RoomLifecycleFailure> {
    if message_id <= 0 {
        return Err(RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "message_id",
            "a Room Message id must be positive",
        ));
    }
    Ok(message_id)
}

fn page_limit(
    requested: Option<i64>,
    default: i64,
    maximum: i64,
) -> std::result::Result<i64, RoomLifecycleFailure> {
    let limit = requested.unwrap_or(default);
    if !(1..=maximum).contains(&limit) {
        return Err(RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "page_limit",
            format!("page limit must be between 1 and {maximum}"),
        ));
    }
    Ok(limit)
}

fn bounded_search_snippet(body: &str) -> String {
    if body.len() <= MAX_SEARCH_SNIPPET_BYTES {
        return body.to_owned();
    }
    let mut end = MAX_SEARCH_SNIPPET_BYTES - '…'.len_utf8();
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    let mut snippet = body[..end].trim_end().to_owned();
    snippet.push('…');
    snippet
}

fn search_query(value: &str) -> std::result::Result<&str, RoomLifecycleFailure> {
    let value = value.trim();
    if value.is_empty() || value.len() > MAX_SEARCH_QUERY_BYTES {
        return Err(RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "search_query",
            "a Room search query needs 1 to 256 bytes",
        ));
    }
    Ok(value)
}

fn room_search_cursor(
    query: &RoomTitleSearchQuery,
) -> std::result::Result<Option<(DateTime<Utc>, Uuid)>, RoomLifecycleFailure> {
    match (&query.before_created_at, query.before_room_id) {
        (None, None) => Ok(None),
        (Some(created_at), Some(room_id)) => DateTime::parse_from_rfc3339(created_at)
            .map(|created_at| Some((created_at.with_timezone(&Utc), room_id)))
            .map_err(|_| {
                RoomLifecycleFailure::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "room_cursor",
                    "before_created_at must be RFC 3339 and paired with before_room_id",
                )
            }),
        _ => Err(RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "room_cursor",
            "before_created_at and before_room_id must be supplied together",
        )),
    }
}

async fn edit_message(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath((company, room, message)): AxumPath<(String, Uuid, i64)>,
    headers: HeaderMap,
    input: std::result::Result<Json<EditMessageInput>, JsonRejection>,
) -> Response<Body> {
    let message = match positive_message_id(message) {
        Ok(message) => message,
        Err(failure) => return failure.into_response(),
    };
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let input = match parse_json(input) {
        Ok(input) => input,
        Err(failure) => return failure.into_response(),
    };
    if input.expected_revision_number < 0 {
        return RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "message_revision",
            "expected_revision_number cannot be negative",
        )
        .into_response();
    }
    if input.body.trim().is_empty() || input.body.len() > MAX_MESSAGE_BODY_BYTES {
        return RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "message_body",
            "a Room Message revision needs 1 to 65536 bytes",
        )
        .into_response();
    }
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .edit_room_message(
            room,
            principal.actor_id(),
            message,
            input.expected_revision_number,
            &input.body,
            &command_id.to_string(),
        )
        .await
    {
        Ok(result) => lifecycle_json(
            StatusCode::OK,
            MessageEditResponse {
                revision: result.revision.into(),
                created: result.created,
            },
        ),
        Err(error) => lifecycle_error(error),
    }
}

async fn delete_message(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath((company, room, message)): AxumPath<(String, Uuid, i64)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let message = match positive_message_id(message) {
        Ok(message) => message,
        Err(failure) => return failure.into_response(),
    };
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    if !body.is_empty() {
        return RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "request_body",
            "Room Message deletion does not accept a request body",
        )
        .into_response();
    }
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .delete_room_message(room, principal.actor_id(), message, &command_id.to_string())
        .await
    {
        Ok(result) => lifecycle_json(StatusCode::OK, result),
        Err(error) => lifecycle_error(error),
    }
}

async fn list_message_revisions(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath((company, room, message)): AxumPath<(String, Uuid, i64)>,
    query: std::result::Result<Query<RevisionPageQuery>, QueryRejection>,
) -> Response<Body> {
    let message = match positive_message_id(message) {
        Ok(message) => message,
        Err(failure) => return failure.into_response(),
    };
    let query = match parse_query(query) {
        Ok(query) => query,
        Err(failure) => return failure.into_response(),
    };
    if query.before_revision_number.is_some_and(|value| value <= 0) {
        return RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "message_revision_cursor",
            "before_revision_number must be positive",
        )
        .into_response();
    }
    let limit = match page_limit(query.limit, DEFAULT_REVISION_LIMIT, MAX_REVISION_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .room_message_revisions_before(
            principal.actor_id(),
            room,
            message,
            query.before_revision_number,
            limit,
        )
        .await
    {
        Ok(page) => lifecycle_json(
            StatusCode::OK,
            MessageRevisionPageResponse {
                revisions: page.revisions.into_iter().map(Into::into).collect(),
                next_before_revision_number: page.next_before_revision_number,
                has_more: page.has_more,
            },
        ),
        Err(error) => lifecycle_error(error),
    }
}

async fn search_messages(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    query: std::result::Result<Query<MessageSearchQuery>, QueryRejection>,
) -> Response<Body> {
    let query = match parse_query(query) {
        Ok(query) => query,
        Err(failure) => return failure.into_response(),
    };
    let search = match search_query(&query.q) {
        Ok(search) => search,
        Err(failure) => return failure.into_response(),
    };
    if query.before_message_id.is_some_and(|value| value <= 0) {
        return RoomLifecycleFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "message_cursor",
            "before_message_id must be positive",
        )
        .into_response();
    }
    let limit = match page_limit(
        query.limit,
        DEFAULT_MESSAGE_SEARCH_LIMIT,
        MAX_MESSAGE_SEARCH_LIMIT,
    ) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .search_room_messages(principal.actor_id(), search, query.before_message_id, limit)
        .await
    {
        Ok(page) => lifecycle_json(
            StatusCode::OK,
            MessageSearchPageResponse {
                messages: page
                    .messages
                    .into_iter()
                    .map(|message| MessageSearchResultResponse {
                        id: message.id,
                        room_id: message.room_id,
                        from_actor: message.from_actor,
                        thread_root_message_id: message.thread_root_message_id,
                        snippet: bounded_search_snippet(&message.body),
                    })
                    .collect(),
                next_before_message_id: page.next_before_message_id,
                has_more: page.has_more,
            },
        ),
        Err(error) => lifecycle_error(error),
    }
}

async fn search_room_titles(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    query: std::result::Result<Query<RoomTitleSearchQuery>, QueryRejection>,
) -> Response<Body> {
    let query = match parse_query(query) {
        Ok(query) => query,
        Err(failure) => return failure.into_response(),
    };
    let search = match search_query(&query.q) {
        Ok(search) => search,
        Err(failure) => return failure.into_response(),
    };
    let before = match room_search_cursor(&query) {
        Ok(before) => before,
        Err(failure) => return failure.into_response(),
    };
    let limit = match page_limit(query.limit, DEFAULT_ROOM_SEARCH_LIMIT, MAX_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .search_rooms_for_actor(principal.actor_id(), search, before, limit)
        .await
    {
        Ok(page) => lifecycle_json(StatusCode::OK, page),
        Err(error) => lifecycle_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::to_bytes;
    use tower::ServiceExt as _;

    struct Fixture {
        state: RoomApiState,
        company: String,
        org: restless_orgintel::OrgIntel,
        room_id: Uuid,
        message_id: i64,
    }

    impl Fixture {
        async fn new() -> Option<Self> {
            let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
            let company = format!("room_lifecycle_api_{}", Uuid::new_v4().simple());
            let org = restless_orgintel::OrgIntel::ensure(&database_url, &company)
                .await
                .expect("ensure Room lifecycle route fixture");
            for (actor, kind, role, display) in [
                ("owner", "owner", "owner", "The Owner"),
                ("alice", "human", "member", "Alice"),
                ("bob", "human", "member", "Bob"),
                ("mallory", "human", "member", "Mallory"),
                ("stale", "human", "member", "Stale Member"),
            ] {
                org.ensure_actor(actor, kind, role, display).await.unwrap();
            }
            let room = org
                .create_room(
                    "alice",
                    restless_orgintel::RoomKind::Group,
                    "Launch review",
                    &["bob"],
                    "create-launch-review-room",
                )
                .await
                .unwrap();
            org.create_room(
                "mallory",
                restless_orgintel::RoomKind::Group,
                "Private roadmap",
                &[],
                "create-private-roadmap-room",
            )
            .await
            .unwrap();
            let message = org
                .send_room_message_with_mentions(
                    room.id,
                    "alice",
                    "Original launch evidence",
                    None,
                    "send-launch-evidence",
                    None,
                    &[],
                    None,
                )
                .await
                .unwrap();
            let state = RoomApiState::fixed(
                HashMap::from([(company.clone(), org.clone())]),
                database_url,
            );
            Some(Self {
                state,
                company,
                org,
                room_id: room.id,
                message_id: message.message.id,
            })
        }

        fn identity(&self, actor: &str) -> VerifiedIdentity {
            VerifiedIdentity {
                user: format!("user-{actor}"),
                issuer: None,
                owner: "fixture-owner".into(),
                scope: CompanyScope::Company {
                    company: self.company.clone(),
                },
                role: "member".into(),
                actor: Some(actor.into()),
                company_id: None,
                cell_id: None,
                membership_id: None,
                membership_version: None,
            }
        }

        fn app(&self, actor: &str) -> Router {
            let principal = RequestPrincipal::from_verified(&self.identity(actor))
                .expect("verified fixture principal");
            routes::<RoomApiState>()
                .layer(Extension(principal))
                .with_state(self.state.clone())
        }

        fn message_path(&self) -> String {
            format!(
                "/companies/{}/rooms/{}/messages/{}",
                self.company, self.room_id, self.message_id
            )
        }
    }

    async fn request(
        app: &Router,
        method: Method,
        uri: impl AsRef<str>,
        command_id: Option<Uuid>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, HeaderMap, serde_json::Value) {
        let mut builder = axum::http::Request::builder()
            .method(method)
            .uri(uri.as_ref());
        if let Some(command_id) = command_id {
            builder = builder.header(IDEMPOTENCY_KEY, command_id.to_string());
        }
        let body = match body {
            Some(body) => {
                builder = builder.header(CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(&body).unwrap())
            }
            None => Body::empty(),
        };
        let response = app
            .clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .expect("Room lifecycle route response");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value = serde_json::from_slice(&bytes).unwrap_or_else(
            |_| serde_json::json!({ "raw": String::from_utf8_lossy(&bytes).to_string() }),
        );
        (status, headers, value)
    }

    #[tokio::test]
    async fn lifecycle_routes_derive_actor_and_fail_closed_for_outsiders_and_stale_members() {
        let Some(fixture) = Fixture::new().await else {
            return;
        };
        let path = fixture.message_path();
        let forged = request(
            &fixture.app("alice"),
            Method::PATCH,
            &path,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "actor_id": "bob",
                "expected_revision_number": 0,
                "body": "Forged author"
            })),
        )
        .await;
        assert_eq!(forged.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(forged.2["error"], "request_body");
        assert_eq!(forged.1.get(CACHE_CONTROL).unwrap(), "no-store");

        let outsider = request(
            &fixture.app("mallory"),
            Method::GET,
            format!("{path}/revisions"),
            None,
            None,
        )
        .await;
        assert_eq!(outsider.0, StatusCode::NOT_FOUND);
        assert_eq!(outsider.2["error"], "room_message");

        let nonexistent = request(
            &fixture.app("alice"),
            Method::GET,
            format!(
                "/companies/{}/rooms/{}/messages/999999999/revisions",
                fixture.company, fixture.room_id
            ),
            None,
            None,
        )
        .await;
        assert_eq!(nonexistent.0, StatusCode::NOT_FOUND);
        assert_eq!(nonexistent.2, outsider.2);

        let hidden_messages = request(
            &fixture.app("mallory"),
            Method::GET,
            format!(
                "/companies/{}/room-messages/search?q=launch",
                fixture.company
            ),
            None,
            None,
        )
        .await;
        assert_eq!(hidden_messages.0, StatusCode::OK);
        assert_eq!(hidden_messages.2["messages"].as_array().unwrap().len(), 0);
        let hidden_room = request(
            &fixture.app("mallory"),
            Method::GET,
            format!("/companies/{}/rooms/search?q=Launch", fixture.company),
            None,
            None,
        )
        .await;
        assert_eq!(hidden_room.0, StatusCode::OK);
        assert_eq!(hidden_room.2["rooms"].as_array().unwrap().len(), 0);

        fixture
            .org
            .retire_actor("stale", "owner", "membership removed")
            .await
            .unwrap();
        let stale = request(
            &fixture.app("stale"),
            Method::GET,
            format!("/companies/{}/rooms/search?q=Launch", fixture.company),
            None,
            None,
        )
        .await;
        assert_eq!(stale.0, StatusCode::FORBIDDEN);
        assert_eq!(stale.2["error"], "request_principal");
    }

    #[tokio::test]
    async fn lifecycle_routes_preserve_conflicts_replay_search_and_tombstones() {
        let Some(fixture) = Fixture::new().await else {
            return;
        };
        let app = fixture.app("alice");
        let path = fixture.message_path();

        let missing_key = request(
            &app,
            Method::PATCH,
            &path,
            None,
            Some(serde_json::json!({
                "expected_revision_number": 0,
                "body": "Reviewed launch evidence"
            })),
        )
        .await;
        assert_eq!(missing_key.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(missing_key.2["error"], "idempotency_key");

        let command_id = Uuid::new_v4();
        let edit_body = serde_json::json!({
            "expected_revision_number": 0,
            "body": "Reviewed launch evidence"
        });
        let edited = request(
            &app,
            Method::PATCH,
            &path,
            Some(command_id),
            Some(edit_body.clone()),
        )
        .await;
        assert_eq!(edited.0, StatusCode::OK);
        assert_eq!(edited.2["created"], true);
        assert_eq!(edited.2["revision"]["revision_number"], 1);
        assert!(edited.2["revision"].get("plain_text").is_none());

        let replay = request(
            &app,
            Method::PATCH,
            &path,
            Some(command_id),
            Some(edit_body),
        )
        .await;
        assert_eq!(replay.0, StatusCode::OK);
        assert_eq!(replay.2["created"], false);
        assert_eq!(replay.2["revision"]["id"], edited.2["revision"]["id"]);

        let reused_key = request(
            &app,
            Method::PATCH,
            &path,
            Some(command_id),
            Some(serde_json::json!({
                "expected_revision_number": 0,
                "body": "A different edit"
            })),
        )
        .await;
        assert_eq!(reused_key.0, StatusCode::CONFLICT);
        assert_eq!(reused_key.2["error"], "room_command");

        let stale_revision = request(
            &app,
            Method::PATCH,
            &path,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_revision_number": 0,
                "body": "Stale revision"
            })),
        )
        .await;
        assert_eq!(stale_revision.0, StatusCode::CONFLICT);

        let second_edit = request(
            &app,
            Method::PATCH,
            &path,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_revision_number": 1,
                "body": "Final reviewed launch evidence"
            })),
        )
        .await;
        assert_eq!(second_edit.0, StatusCode::OK);
        assert_eq!(second_edit.2["revision"]["revision_number"], 2);

        let revisions = request(
            &app,
            Method::GET,
            format!("{path}/revisions?limit=1"),
            None,
            None,
        )
        .await;
        assert_eq!(revisions.0, StatusCode::OK);
        assert_eq!(revisions.2["revisions"].as_array().unwrap().len(), 1);
        assert!(revisions.2["revisions"][0].get("plain_text").is_none());

        let oversized_revision_page = request(
            &app,
            Method::GET,
            format!("{path}/revisions?limit={}", MAX_REVISION_LIMIT + 1),
            None,
            None,
        )
        .await;
        assert_eq!(oversized_revision_page.0, StatusCode::UNPROCESSABLE_ENTITY);

        let search = request(
            &app,
            Method::GET,
            format!(
                "/companies/{}/room-messages/search?q=reviewed%20launch",
                fixture.company
            ),
            None,
            None,
        )
        .await;
        assert_eq!(search.0, StatusCode::OK);
        assert_eq!(
            search.2["messages"][0]["snippet"],
            "Final reviewed launch evidence"
        );
        assert!(search.2["messages"][0].get("body").is_none());

        let oversized_search_page = request(
            &app,
            Method::GET,
            format!(
                "/companies/{}/room-messages/search?q=reviewed&limit={}",
                fixture.company,
                MAX_MESSAGE_SEARCH_LIMIT + 1
            ),
            None,
            None,
        )
        .await;
        assert_eq!(oversized_search_page.0, StatusCode::UNPROCESSABLE_ENTITY);
        let room_search = request(
            &app,
            Method::GET,
            format!("/companies/{}/rooms/search?q=launch", fixture.company),
            None,
            None,
        )
        .await;
        assert_eq!(room_search.0, StatusCode::OK);
        assert_eq!(room_search.2["rooms"][0]["id"], fixture.room_id.to_string());

        let delete_id = Uuid::new_v4();
        let deleted = request(&app, Method::DELETE, &path, Some(delete_id), None).await;
        assert_eq!(deleted.0, StatusCode::OK);
        assert_eq!(deleted.2["created"], true);
        let delete_replay = request(&app, Method::DELETE, &path, Some(delete_id), None).await;
        assert_eq!(delete_replay.0, StatusCode::OK);
        assert_eq!(delete_replay.2["created"], false);
        assert_eq!(
            delete_replay.2["tombstone"]["id"],
            deleted.2["tombstone"]["id"]
        );

        let deleted_search = request(
            &app,
            Method::GET,
            format!(
                "/companies/{}/room-messages/search?q=reviewed",
                fixture.company
            ),
            None,
            None,
        )
        .await;
        assert_eq!(deleted_search.0, StatusCode::OK);
        assert_eq!(deleted_search.2["messages"].as_array().unwrap().len(), 0);
        let deleted_revisions =
            request(&app, Method::GET, format!("{path}/revisions"), None, None).await;
        assert_eq!(deleted_revisions.0, StatusCode::NOT_FOUND);
    }

    #[test]
    fn search_snippets_are_utf8_safe_and_fit_the_wire_budget() {
        let body = "🜁".repeat(MAX_SEARCH_SNIPPET_BYTES);
        let snippet = bounded_search_snippet(&body);
        assert!(snippet.len() <= MAX_SEARCH_SNIPPET_BYTES);
        assert!(snippet.ends_with('…'));
        assert!(snippet.is_char_boundary(snippet.len()));
    }
}
