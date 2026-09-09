//! Authenticated company-scoped HTTP surface for Core-native Documents.
//!
//! This module is intentionally a thin transport over `restless_orgintel`.
//! The route never accepts an acting Actor or company id in a request body:
//! both come from the verified owner-entry principal and the exact route.

use super::*;

const DOCUMENT_BODY_LIMIT: usize = 1_250_000;
const DEFAULT_PAGE_LIMIT: i64 = 50;
const MAX_DOCUMENT_PAGE_LIMIT: i64 = 100;
const MAX_DOCUMENT_CHILD_PAGE_LIMIT: i64 = 50;
const MAX_DOCUMENT_VERSION_PAGE_LIMIT: i64 = 25;
const MAX_DOCUMENT_PARTICIPANT_PAGE_LIMIT: i64 = 50;
const IDEMPOTENCY_KEY: &str = "idempotency-key";

pub(super) fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    RoomApiState: FromRef<S>,
{
    Router::<S>::new()
        .route(
            "/companies/{company}/documents",
            get(list_documents).post(create_document),
        )
        .route(
            "/companies/{company}/documents/{document}",
            get(get_document).patch(update_document),
        )
        .route(
            "/companies/{company}/documents/{document}/versions",
            get(list_document_versions).post(create_document_version),
        )
        .route(
            "/companies/{company}/documents/{document}/versions/{version}",
            get(get_document_version),
        )
        .route(
            "/companies/{company}/documents/{document}/versions/{version}/restore",
            post(restore_document_version),
        )
        .route(
            "/companies/{company}/documents/{document}/participants",
            get(list_document_participants),
        )
        .route(
            "/companies/{company}/documents/{document}/participants/{participant}",
            axum::routing::put(set_document_participant).delete(remove_document_participant),
        )
        .route(
            "/companies/{company}/documents/{document}/comments",
            get(list_document_comments).post(create_document_comment),
        )
        .route(
            "/companies/{company}/documents/{document}/comments/{thread}/replies",
            get(list_document_comment_replies).post(reply_to_document_comment),
        )
        .route(
            "/companies/{company}/documents/{document}/comments/{thread}/resolve",
            post(resolve_document_comment),
        )
        .route(
            "/companies/{company}/documents/{document}/reviews",
            get(list_document_reviews).post(request_document_review),
        )
        .route(
            "/companies/{company}/documents/{document}/reviews/{review}",
            get(get_document_review),
        )
        .route(
            "/companies/{company}/documents/{document}/reviews/{review}/accept",
            post(accept_document_review),
        )
        .route(
            "/companies/{company}/documents/{document}/proposals",
            get(list_document_proposals),
        )
        .route(
            "/companies/{company}/documents/{document}/proposals/{proposal}",
            get(get_document_proposal),
        )
        .route(
            "/companies/{company}/documents/{document}/proposals/{proposal}/accept",
            post(accept_document_proposal),
        )
        .route(
            "/companies/{company}/documents/{document}/proposals/{proposal}/reject",
            post(reject_document_proposal),
        )
        .layer(DefaultBodyLimit::max(DOCUMENT_BODY_LIMIT))
        .layer(middleware::map_response(no_store_response))
}

struct DocumentPrincipal(RequestPrincipal);

impl<S> FromRequestParts<S> for DocumentPrincipal
where
    S: Send + Sync,
{
    type Rejection = Response<Body>;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<RequestPrincipal>()
            .cloned()
            .map(Self)
            .ok_or_else(|| {
                document_api_error(
                    StatusCode::UNAUTHORIZED,
                    "no_session",
                    "Document access requires a verified company session",
                )
            })
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct DocumentListQuery {
    #[serde(default)]
    include_archived: bool,
    before_updated_at: Option<String>,
    before_document_id: Option<Uuid>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct VersionListQuery {
    before_version_number: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct CreatedPageQuery {
    after_created_at: Option<String>,
    after_id: Option<Uuid>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ParticipantListQuery {
    after_added_at: Option<String>,
    after_actor_id: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateDocumentInput {
    title: String,
    kind: restless_orgintel::DocumentKind,
    visibility: restless_orgintel::DocumentVisibility,
    #[serde(default)]
    linked_room_id: Option<Uuid>,
    #[serde(default)]
    inherit_room_visibility: bool,
    content_json: serde_json::Value,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateDocumentInput {
    expected_version: i64,
    title: String,
    kind: restless_orgintel::DocumentKind,
    visibility: restless_orgintel::DocumentVisibility,
    #[serde(default)]
    linked_room_id: Option<Uuid>,
    #[serde(default)]
    inherit_room_visibility: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetDocumentParticipantInput {
    expected_document_version: i64,
    access: restless_orgintel::DocumentAccess,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoveDocumentParticipantInput {
    expected_document_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateDocumentVersionInput {
    expected_current_version_id: Uuid,
    content_json: serde_json::Value,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreDocumentVersionInput {
    expected_current_version_id: Uuid,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateDocumentCommentInput {
    #[serde(default)]
    block_id: Option<String>,
    content_json: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplyToDocumentCommentInput {
    #[serde(default)]
    reply_to_comment_id: Option<Uuid>,
    content_json: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolveDocumentCommentInput {
    expected_thread_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestDocumentReviewInput {
    expected_document_version: i64,
    expected_current_version_id: Uuid,
    summary: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptDocumentReviewInput {
    expected_document_version: i64,
    expected_review_version: i64,
    accepted_version_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolveDocumentProposalInput {
    expected_proposal_version: i64,
    resolution_summary: String,
    #[serde(default)]
    accepted_version_name: Option<String>,
}

async fn document_orgintel(
    state: &RoomApiState,
    principal: &RequestPrincipal,
    company: &str,
) -> std::result::Result<restless_orgintel::OrgIntel, Response<Body>> {
    if !principal.permits_company(company) {
        return Err(document_api_error(
            StatusCode::FORBIDDEN,
            "company_out_of_scope",
            "this session is not scoped to that company",
        ));
    }
    let org = state.orgintel(company).await.map_err(|error| {
        tracing::error!(%error, company, "could not open company Document store");
        document_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            "company Documents are temporarily unavailable",
        )
    })?;
    match org.active_actor(principal.actor_id()).await {
        Ok(Some(actor)) if actor.actor_class == "human" => Ok(org),
        Ok(_) => Err(document_api_error(
            StatusCode::FORBIDDEN,
            "request_principal",
            "the verified request principal is not an active human Actor",
        )),
        Err(error) => {
            tracing::error!(%error, company, actor = principal.actor_id(), "could not resolve Document principal");
            Err(document_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company Documents are temporarily unavailable",
            ))
        }
    }
}

fn document_api_error(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> Response<Body> {
    no_store(api_error(status, code, message))
}

#[derive(Debug)]
struct DocumentApiFailure {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl DocumentApiFailure {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for DocumentApiFailure {
    fn into_response(self) -> Response<Body> {
        document_api_error(self.status, self.code, self.message)
    }
}

fn document_error(error: restless_orgintel::DocumentError) -> Response<Body> {
    match error {
        restless_orgintel::DocumentError::Invalid(message) => {
            document_api_error(StatusCode::UNPROCESSABLE_ENTITY, "document", message)
        }
        restless_orgintel::DocumentError::Unavailable => document_api_error(
            StatusCode::NOT_FOUND,
            "document",
            "the Document is unavailable",
        ),
        restless_orgintel::DocumentError::Conflict(message) => {
            document_api_error(StatusCode::CONFLICT, "document_conflict", message)
        }
        restless_orgintel::DocumentError::Corrupt(message) => {
            tracing::error!(message, "native Document state is corrupt");
            document_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "document_unavailable",
                "company Documents are temporarily unavailable",
            )
        }
        restless_orgintel::DocumentError::Database(error) => {
            tracing::error!(%error, "native Document operation failed");
            document_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company Documents are temporarily unavailable",
            )
        }
    }
}

fn no_store(mut response: Response<Body>) -> Response<Body> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

async fn no_store_response(response: Response<Body>) -> Response<Body> {
    no_store(response)
}

fn document_json<T: Serialize>(status: StatusCode, value: T) -> Response<Body> {
    no_store((status, Json(value)).into_response())
}

fn command_id(headers: &HeaderMap) -> std::result::Result<Uuid, DocumentApiFailure> {
    let mut values = headers.get_all(IDEMPOTENCY_KEY).iter();
    let Some(value) = values.next() else {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Document mutations require one UUID Idempotency-Key header",
        ));
    };
    if values.next().is_some() {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Document mutations require exactly one Idempotency-Key header",
        ));
    }
    let value = value.to_str().ok().map(str::trim).unwrap_or_default();
    let command_id = Uuid::parse_str(value).map_err(|_| {
        DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Idempotency-Key must be a non-nil UUID",
        )
    })?;
    if command_id.is_nil() {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "idempotency_key",
            "Idempotency-Key must be a non-nil UUID",
        ));
    }
    Ok(command_id)
}

fn validate_participant_actor_id(value: &str) -> std::result::Result<(), DocumentApiFailure> {
    if value.trim().is_empty() || value.len() > 255 || value.chars().any(char::is_control) {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "participant_actor_id",
            "participant Actor id must contain 1 to 255 non-control bytes",
        ));
    }
    Ok(())
}

fn page_limit(
    requested: Option<i64>,
    maximum: i64,
) -> std::result::Result<i64, DocumentApiFailure> {
    let limit = requested.unwrap_or(DEFAULT_PAGE_LIMIT.min(maximum));
    if !(1..=maximum).contains(&limit) {
        return Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "page_limit",
            format!("page limit must be between 1 and {maximum}"),
        ));
    }
    Ok(limit)
}

fn participant_cursor(
    query: &ParticipantListQuery,
) -> std::result::Result<Option<restless_orgintel::DocumentParticipantCursor>, DocumentApiFailure> {
    match (&query.after_added_at, &query.after_actor_id) {
        (None, None) => Ok(None),
        (Some(added_at), Some(actor_id)) => {
            validate_participant_actor_id(actor_id)?;
            DateTime::parse_from_rfc3339(added_at)
                .map(|added_at| {
                    Some(restless_orgintel::DocumentParticipantCursor {
                        added_at: added_at.with_timezone(&Utc),
                        actor_id: actor_id.clone(),
                    })
                })
                .map_err(|_| {
                    DocumentApiFailure::new(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "document_cursor",
                        "after_added_at must be RFC 3339 and paired with after_actor_id",
                    )
                })
        }
        _ => Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_cursor",
            "after_added_at and after_actor_id must be supplied together",
        )),
    }
}

fn document_list_cursor(
    query: &DocumentListQuery,
) -> std::result::Result<Option<restless_orgintel::DocumentListCursor>, DocumentApiFailure> {
    match (&query.before_updated_at, query.before_document_id) {
        (None, None) => Ok(None),
        (Some(updated_at), Some(id)) => DateTime::parse_from_rfc3339(updated_at)
            .map(|updated_at| {
                Some(restless_orgintel::DocumentListCursor {
                    updated_at: updated_at.with_timezone(&Utc),
                    id,
                })
            })
            .map_err(|_| {
                DocumentApiFailure::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "document_cursor",
                    "before_updated_at must be RFC 3339 and paired with before_document_id",
                )
            }),
        _ => Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_cursor",
            "before_updated_at and before_document_id must be supplied together",
        )),
    }
}

fn created_cursor(
    query: &CreatedPageQuery,
) -> std::result::Result<Option<restless_orgintel::DocumentPageCursor>, DocumentApiFailure> {
    match (&query.after_created_at, query.after_id) {
        (None, None) => Ok(None),
        (Some(created_at), Some(id)) => DateTime::parse_from_rfc3339(created_at)
            .map(|created_at| {
                Some(restless_orgintel::DocumentPageCursor {
                    created_at: created_at.with_timezone(&Utc),
                    id,
                })
            })
            .map_err(|_| {
                DocumentApiFailure::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "document_cursor",
                    "after_created_at must be RFC 3339 and paired with after_id",
                )
            }),
        _ => Err(DocumentApiFailure::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_cursor",
            "after_created_at and after_id must be supplied together",
        )),
    }
}

async fn list_documents(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<DocumentListQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match document_list_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_documents_for_actor(
            principal.actor_id(),
            query.include_archived,
            cursor.as_ref(),
            limit,
        )
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn create_document(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<CreateDocumentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .create_document(restless_orgintel::NewDocument {
            command_id,
            title: &input.title,
            kind: input.kind,
            visibility: input.visibility,
            linked_room_id: input.linked_room_id,
            inherit_room_visibility: input.inherit_room_visibility,
            owner_actor_id: principal.actor_id(),
            created_by_actor_id: principal.actor_id(),
            content_json: &input.content_json,
            reason: &input.reason,
        })
        .await
    {
        Ok(document) => document_json(StatusCode::CREATED, document),
        Err(error) => document_error(error),
    }
}

async fn get_document(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .get_document_for_actor(document, principal.actor_id())
        .await
    {
        Ok(document) => document_json(StatusCode::OK, document),
        Err(error) => document_error(error),
    }
}

async fn update_document(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<UpdateDocumentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .update_document_metadata(restless_orgintel::UpdateDocumentMetadata {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_version: input.expected_version,
            title: &input.title,
            kind: input.kind,
            visibility: input.visibility,
            linked_room_id: input.linked_room_id,
            inherit_room_visibility: input.inherit_room_visibility,
        })
        .await
    {
        Ok(document) => document_json(StatusCode::OK, document),
        Err(error) => document_error(error),
    }
}

async fn list_document_versions(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<VersionListQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_VERSION_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match query.before_version_number {
        Some(version_number) if version_number <= 0 => {
            return document_api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "document_cursor",
                "before_version_number must be positive",
            );
        }
        Some(version_number) => Some(restless_orgintel::DocumentVersionCursor { version_number }),
        None => None,
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_versions_for_actor(document, principal.actor_id(), cursor.as_ref(), limit)
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn create_document_version(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<CreateDocumentVersionInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .create_named_document_version(restless_orgintel::NewNamedDocumentVersion {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_current_version_id: input.expected_current_version_id,
            content_json: &input.content_json,
            reason: &input.reason,
        })
        .await
    {
        Ok(document) => document_json(StatusCode::CREATED, document),
        Err(error) => document_error(error),
    }
}

async fn get_document_version(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, version)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .get_document_version_for_actor(document, version, principal.actor_id())
        .await
    {
        Ok(version) => document_json(StatusCode::OK, version),
        Err(error) => document_error(error),
    }
}

async fn restore_document_version(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, version)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<RestoreDocumentVersionInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .restore_document_version(restless_orgintel::RestoreDocumentVersion {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_current_version_id: input.expected_current_version_id,
            source_version_id: version,
            reason: &input.reason,
        })
        .await
    {
        Ok(document) => document_json(StatusCode::CREATED, document),
        Err(error) => document_error(error),
    }
}

async fn list_document_participants(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<ParticipantListQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_PARTICIPANT_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match participant_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_participants_for_actor(
            document,
            principal.actor_id(),
            cursor.as_ref(),
            limit,
        )
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn set_document_participant(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, participant)): AxumPath<(String, Uuid, String)>,
    headers: HeaderMap,
    Json(input): Json<SetDocumentParticipantInput>,
) -> Response<Body> {
    if let Err(failure) = validate_participant_actor_id(&participant) {
        return failure.into_response();
    }
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .set_document_participant(restless_orgintel::SetDocumentParticipant {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_document_version: input.expected_document_version,
            participant_actor_id: &participant,
            access: input.access,
        })
        .await
    {
        Ok(result) => document_json(StatusCode::OK, result),
        Err(error) => document_error(error),
    }
}

async fn remove_document_participant(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, participant)): AxumPath<(String, Uuid, String)>,
    headers: HeaderMap,
    Json(input): Json<RemoveDocumentParticipantInput>,
) -> Response<Body> {
    if let Err(failure) = validate_participant_actor_id(&participant) {
        return failure.into_response();
    }
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .remove_document_participant(restless_orgintel::RemoveDocumentParticipant {
            command_id,
            document_id: document,
            actor_id: principal.actor_id(),
            expected_document_version: input.expected_document_version,
            participant_actor_id: &participant,
        })
        .await
    {
        Ok(result) => document_json(StatusCode::OK, result),
        Err(error) => document_error(error),
    }
}

async fn create_document_comment(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<CreateDocumentCommentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .create_document_comment_thread(restless_orgintel::NewDocumentCommentThread {
            document_id: document,
            actor_id: principal.actor_id(),
            command_id,
            block_id: input.block_id.as_deref(),
            content_json: &input.content_json,
        })
        .await
    {
        Ok(comment) => document_json(StatusCode::CREATED, comment),
        Err(error) => document_error(error),
    }
}

async fn list_document_comments(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<CreatedPageQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_CHILD_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match created_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_comment_threads(document, principal.actor_id(), cursor.as_ref(), limit)
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn reply_to_document_comment(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, thread)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ReplyToDocumentCommentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .reply_to_document_comment(restless_orgintel::ReplyToDocumentComment {
            document_id: document,
            thread_id: thread,
            reply_to_comment_id: input.reply_to_comment_id,
            actor_id: principal.actor_id(),
            command_id,
            content_json: &input.content_json,
        })
        .await
    {
        Ok(comment) => document_json(StatusCode::CREATED, comment),
        Err(error) => document_error(error),
    }
}

async fn list_document_comment_replies(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, thread)): AxumPath<(String, Uuid, Uuid)>,
    Query(query): Query<CreatedPageQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match created_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_comments(
            document,
            thread,
            principal.actor_id(),
            cursor.as_ref(),
            limit,
        )
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn resolve_document_comment(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, thread)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ResolveDocumentCommentInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .resolve_document_comment_thread(restless_orgintel::ResolveDocumentCommentThread {
            document_id: document,
            thread_id: thread,
            actor_id: principal.actor_id(),
            command_id,
            expected_thread_version: input.expected_thread_version,
        })
        .await
    {
        Ok(thread) => document_json(StatusCode::OK, thread),
        Err(error) => document_error(error),
    }
}

async fn list_document_reviews(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<CreatedPageQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_CHILD_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match created_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_reviews(document, principal.actor_id(), cursor.as_ref(), limit)
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn request_document_review(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<RequestDocumentReviewInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .request_document_review(restless_orgintel::RequestDocumentReview {
            document_id: document,
            actor_id: principal.actor_id(),
            command_id,
            expected_document_version: input.expected_document_version,
            expected_current_version_id: input.expected_current_version_id,
            summary: &input.summary,
        })
        .await
    {
        Ok(review) => document_json(StatusCode::CREATED, review),
        Err(error) => document_error(error),
    }
}

async fn get_document_review(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, review)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .get_document_review_for_actor(document, review, principal.actor_id())
        .await
    {
        Ok(review) => document_json(StatusCode::OK, review),
        Err(error) => document_error(error),
    }
}

async fn accept_document_review(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, review)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<AcceptDocumentReviewInput>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .accept_document_review(restless_orgintel::AcceptDocumentReview {
            document_id: document,
            review_id: review,
            actor_id: principal.actor_id(),
            command_id,
            expected_document_version: input.expected_document_version,
            expected_review_version: input.expected_review_version,
            accepted_version_name: &input.accepted_version_name,
        })
        .await
    {
        Ok(resolution) => document_json(StatusCode::OK, resolution),
        Err(error) => document_error(error),
    }
}

async fn list_document_proposals(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
    Query(query): Query<CreatedPageQuery>,
) -> Response<Body> {
    let limit = match page_limit(query.limit, MAX_DOCUMENT_CHILD_PAGE_LIMIT) {
        Ok(limit) => limit,
        Err(failure) => return failure.into_response(),
    };
    let cursor = match created_cursor(&query) {
        Ok(cursor) => cursor,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .list_document_revision_proposals(document, principal.actor_id(), cursor.as_ref(), limit)
        .await
    {
        Ok(page) => document_json(StatusCode::OK, page),
        Err(error) => document_error(error),
    }
}

async fn get_document_proposal(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, proposal)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .get_document_revision_proposal_for_actor(document, proposal, principal.actor_id())
        .await
    {
        Ok(proposal) => document_json(StatusCode::OK, proposal),
        Err(error) => document_error(error),
    }
}

async fn accept_document_proposal(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, proposal)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ResolveDocumentProposalInput>,
) -> Response<Body> {
    let Some(accepted_version_name) = input.accepted_version_name.as_deref() else {
        return document_api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document",
            "accepting a proposal requires accepted_version_name",
        );
    };
    resolve_document_proposal(
        state,
        principal,
        company,
        document,
        proposal,
        headers,
        input.expected_proposal_version,
        restless_orgintel::DocumentRevisionDecision::Accept,
        &input.resolution_summary,
        Some(accepted_version_name),
    )
    .await
}

async fn reject_document_proposal(
    State(state): State<RoomApiState>,
    DocumentPrincipal(principal): DocumentPrincipal,
    AxumPath((company, document, proposal)): AxumPath<(String, Uuid, Uuid)>,
    headers: HeaderMap,
    Json(input): Json<ResolveDocumentProposalInput>,
) -> Response<Body> {
    if input.accepted_version_name.is_some() {
        return document_api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "document",
            "rejecting a proposal cannot include accepted_version_name",
        );
    }
    resolve_document_proposal(
        state,
        principal,
        company,
        document,
        proposal,
        headers,
        input.expected_proposal_version,
        restless_orgintel::DocumentRevisionDecision::Reject,
        &input.resolution_summary,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn resolve_document_proposal(
    state: RoomApiState,
    principal: RequestPrincipal,
    company: String,
    document: Uuid,
    proposal: Uuid,
    headers: HeaderMap,
    expected_proposal_version: i64,
    decision: restless_orgintel::DocumentRevisionDecision,
    resolution_summary: &str,
    accepted_version_name: Option<&str>,
) -> Response<Body> {
    let command_id = match command_id(&headers) {
        Ok(command_id) => command_id,
        Err(failure) => return failure.into_response(),
    };
    let org = match document_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .resolve_document_revision_proposal(restless_orgintel::ResolveDocumentRevisionProposal {
            document_id: document,
            proposal_id: proposal,
            actor_id: principal.actor_id(),
            command_id,
            expected_proposal_version,
            decision,
            resolution_summary,
            accepted_version_name,
        })
        .await
    {
        Ok(resolution) => document_json(StatusCode::OK, resolution),
        Err(error) => document_error(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt as _;

    struct DocumentRouteFixture {
        state: RoomApiState,
        company: String,
        other_company: String,
        org: restless_orgintel::OrgIntel,
    }

    impl DocumentRouteFixture {
        async fn new() -> Option<Self> {
            let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
            let suffix = Uuid::new_v4().simple().to_string();
            let company = format!("document_api_a_{suffix}");
            let other_company = format!("document_api_b_{suffix}");
            let mut companies = HashMap::new();
            for name in [&company, &other_company] {
                let org = restless_orgintel::OrgIntel::ensure(&database_url, name)
                    .await
                    .expect("ensure Document route fixture company");
                org.ensure_actor("owner", "owner", "owner", "The Owner")
                    .await
                    .unwrap();
                org.ensure_actor("alice", "human", "member", "Alice")
                    .await
                    .unwrap();
                org.ensure_actor("research-analyst", "staff", "analyst", "Research Analyst")
                    .await
                    .unwrap();
                companies.insert(name.to_string(), org);
            }
            let org = companies.get(&company).expect("primary company").clone();
            Some(Self {
                state: RoomApiState::fixed(companies, database_url),
                company,
                other_company,
                org,
            })
        }

        fn identity(actor: &str, role: &str, company: &str) -> VerifiedIdentity {
            VerifiedIdentity {
                user: format!("user-{actor}"),
                issuer: None,
                owner: "fixture-owner".into(),
                scope: CompanyScope::Company {
                    company: company.to_string(),
                },
                role: role.to_string(),
                actor: Some(actor.to_string()),
                company_id: None,
                cell_id: None,
                membership_id: None,
                membership_version: None,
            }
        }

        fn app(&self, actor: &str, role: &str, company: &str) -> Router {
            let principal = RequestPrincipal::from_verified(&Self::identity(actor, role, company))
                .expect("verified fixture principal");
            routes::<RoomApiState>()
                .layer(Extension(principal))
                .with_state(self.state.clone())
        }

        fn unauthenticated_app(&self) -> Router {
            routes::<RoomApiState>().with_state(self.state.clone())
        }
    }

    fn content(block_id: &str, text: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "attrs": {"block_id": block_id},
                "content": [{"type": "text", "text": text}]
            }]
        })
    }

    fn create_body(title: &str) -> serde_json::Value {
        serde_json::json!({
            "title": title,
            "kind": "brief",
            "visibility": "participants",
            "linked_room_id": null,
            "inherit_room_visibility": false,
            "content_json": content("claim", "Initial evidence"),
            "reason": "Initial version"
        })
    }

    fn uuid_field(value: &serde_json::Value, field: &str) -> Uuid {
        Uuid::parse_str(value[field].as_str().unwrap()).unwrap()
    }

    async fn request(
        app: &Router,
        method: Method,
        uri: impl AsRef<str>,
        idempotency_key: Option<Uuid>,
        body: Option<serde_json::Value>,
    ) -> Response<Body> {
        let mut builder = Request::builder().method(method).uri(uri.as_ref());
        if let Some(key) = idempotency_key {
            builder = builder.header(IDEMPOTENCY_KEY, key.to_string());
        }
        let body = match body {
            Some(body) => {
                builder = builder.header(CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(&body).unwrap())
            }
            None => Body::empty(),
        };
        app.clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .expect("Document route response")
    }

    async fn response_json(response: Response<Body>) -> (StatusCode, serde_json::Value) {
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read Document route response");
        let body = serde_json::from_slice(&bytes).unwrap_or_else(
            |_| serde_json::json!({"raw": String::from_utf8_lossy(&bytes).to_string()}),
        );
        (status, body)
    }

    async fn request_json(
        app: &Router,
        method: Method,
        uri: impl AsRef<str>,
        idempotency_key: Option<Uuid>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        response_json(request(app, method, uri, idempotency_key, body).await).await
    }

    #[test]
    fn document_cursor_requires_an_exact_pair() {
        let query = DocumentListQuery {
            before_document_id: Some(Uuid::new_v4()),
            ..Default::default()
        };
        assert!(document_list_cursor(&query).is_err());

        let query = CreatedPageQuery {
            after_created_at: Some(Utc::now().to_rfc3339()),
            ..Default::default()
        };
        assert!(created_cursor(&query).is_err());

        let query = ParticipantListQuery {
            after_actor_id: Some("alice".into()),
            ..Default::default()
        };
        assert!(participant_cursor(&query).is_err());

        let query = ParticipantListQuery {
            after_added_at: Some(Utc::now().to_rfc3339()),
            after_actor_id: Some("x".repeat(256)),
            ..Default::default()
        };
        assert!(participant_cursor(&query).is_err());
    }

    #[test]
    fn document_page_limits_are_bounded() {
        assert!(page_limit(Some(0), 50).is_err());
        assert!(page_limit(Some(51), 50).is_err());
        assert_eq!(page_limit(None, 50).unwrap(), DEFAULT_PAGE_LIMIT);
        assert_eq!(page_limit(None, 25).unwrap(), 25);
    }

    #[tokio::test]
    async fn document_routes_require_exact_authenticated_company_and_actor_scope() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping Document route security scenario"
            );
            return;
        };
        let documents = format!("/companies/{}/documents", fixture.company);
        let (status, body) = request_json(
            &fixture.unauthenticated_app(),
            Method::GET,
            &documents,
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "no_session");

        let owner = fixture.app("owner", "owner", &fixture.company);
        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(create_body("Private evidence")),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = Uuid::parse_str(created["document_id"].as_str().unwrap()).unwrap();

        let (status, body) = request_json(
            &owner,
            Method::GET,
            format!("/companies/{}/documents/{document}", fixture.other_company),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "company_out_of_scope");

        let alice = fixture.app("alice", "member", &fixture.company);
        let (status, body) = request_json(
            &alice,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "document");

        let agent = fixture.app("research-analyst", "member", &fixture.company);
        let (status, body) = request_json(
            &agent,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "request_principal");
    }

    #[tokio::test]
    async fn document_routes_bound_and_validate_transport_before_mutation() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Document route input scenario");
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);

        let (status, body) = request_json(
            &owner,
            Method::POST,
            &documents,
            None,
            Some(create_body("No command")),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "idempotency_key");

        let response = owner
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(&documents)
                    .header(CONTENT_TYPE, "application/json")
                    .header(IDEMPOTENCY_KEY, Uuid::new_v4().to_string())
                    .header(IDEMPOTENCY_KEY, Uuid::new_v4().to_string())
                    .body(Body::from(
                        serde_json::to_vec(&create_body("Ambiguous command")).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = response_json(response).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "idempotency_key");

        let response = owner
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(&documents)
                    .header(CONTENT_TYPE, "application/json")
                    .header(IDEMPOTENCY_KEY, Uuid::new_v4().to_string())
                    .body(Body::from("{not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, _) = response_json(response).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let mut unknown = create_body("Unknown field");
        unknown["actor_id"] = serde_json::json!("owner");
        let (status, _) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(unknown),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let (status, body) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "title": "Invalid content",
                "kind": "brief",
                "visibility": "participants",
                "content_json": {"type":"doc","content":[{"type":"raw_html"}]},
                "reason": "Invalid"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "document");

        let (status, _) = request_json(
            &owner,
            Method::GET,
            format!("{documents}?limit=101"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(create_body("Lifecycle boundary")),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = created["document_id"].as_str().unwrap();
        let (status, _) = request_json(
            &owner,
            Method::PATCH,
            format!("{documents}/{document}"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_version": 1,
                "title": "Forged acceptance",
                "kind": "brief",
                "status": "accepted",
                "visibility": "participants",
                "linked_room_id": null,
                "inherit_room_visibility": false
            })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, unchanged) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(unchanged["document"]["status"], "draft");
        assert_eq!(unchanged["document"]["version"], 1);

        let oversized = serde_json::json!({
            "title": "Oversized",
            "kind": "brief",
            "visibility": "participants",
            "content_json": content("claim", &"x".repeat(DOCUMENT_BODY_LIMIT + 1)),
            "reason": "Too large"
        });
        let response = request(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(oversized),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&HeaderValue::from_static("no-store"))
        );
    }

    #[tokio::test]
    async fn document_command_receipts_survive_concurrency_delay_and_participant_revocation() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Document receipt scenario");
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let alice = fixture.app("alice", "member", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);
        let create_key = Uuid::new_v4();
        let create = create_body("Concurrent command");
        let (first_create, second_create) = tokio::join!(
            request_json(
                &owner,
                Method::POST,
                &documents,
                Some(create_key),
                Some(create.clone())
            ),
            request_json(
                &owner,
                Method::POST,
                &documents,
                Some(create_key),
                Some(create.clone())
            )
        );
        assert_eq!(first_create.0, StatusCode::CREATED);
        assert_eq!(second_create, first_create);
        let created = first_create.1;
        let document = uuid_field(&created, "document_id");
        assert_eq!(created["operation"], "document_create");

        let (status, initial) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let initial_version = Uuid::parse_str(
            initial["current_version"]["version"]["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();

        let metadata_key = Uuid::new_v4();
        let metadata = serde_json::json!({
            "expected_version": 1,
            "title": "Concurrent command refined",
            "kind": "plan",
            "visibility": "participants",
            "linked_room_id": null,
            "inherit_room_visibility": false
        });
        let document_path = format!("{documents}/{document}");
        let (first_metadata, second_metadata) = tokio::join!(
            request_json(
                &owner,
                Method::PATCH,
                &document_path,
                Some(metadata_key),
                Some(metadata.clone())
            ),
            request_json(
                &owner,
                Method::PATCH,
                &document_path,
                Some(metadata_key),
                Some(metadata.clone())
            )
        );
        assert_eq!(first_metadata.0, StatusCode::OK);
        assert_eq!(second_metadata, first_metadata);
        let metadata_result = first_metadata.1;

        let participants = format!("{document_path}/participants");
        let (status, initial_participants) =
            request_json(&owner, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(initial_participants["items"].as_array().unwrap().len(), 1);
        let (status, _) = request_json(
            &owner,
            Method::GET,
            format!("{participants}?limit=51"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let alice_participant = format!("{participants}/alice");
        let read_key = Uuid::new_v4();
        let read_body = serde_json::json!({
            "expected_document_version": 2,
            "access": "read"
        });
        let (status, read_result) = request_json(
            &owner,
            Method::PUT,
            &alice_participant,
            Some(read_key),
            Some(read_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(read_result["operation"], "participant_set");
        let (status, owner_participants) =
            request_json(&owner, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(owner_participants["items"].as_array().unwrap().len(), 2);
        let (status, _) = request_json(&alice, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = request_json(&alice, Method::GET, &document_path, None, None).await;
        assert_eq!(status, StatusCode::OK);

        let comments = format!("{document_path}/comments");
        let comment_body = serde_json::json!({
            "content_json": content("comment", "Read access cannot comment")
        });
        let (status, _) = request_json(
            &alice,
            Method::POST,
            &comments,
            Some(Uuid::new_v4()),
            Some(comment_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, _) = request_json(
            &owner,
            Method::PUT,
            &alice_participant,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 3,
                "access": "comment"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = request_json(&alice, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = request_json(
            &alice,
            Method::POST,
            &comments,
            Some(Uuid::new_v4()),
            Some(comment_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let (status, _) = request_json(
            &owner,
            Method::PUT,
            &alice_participant,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 4,
                "access": "edit"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = request_json(&alice, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, _) = request_json(
            &alice,
            Method::PUT,
            format!("{participants}/owner"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 5,
                "access": "read"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let versions = format!("{document_path}/versions");
        let version_key = Uuid::new_v4();
        let version_body = serde_json::json!({
            "expected_current_version_id": initial_version,
            "content_json": content("claim", "Edited by Alice"),
            "reason": "Alice edit"
        });
        let (first_version, second_version) = tokio::join!(
            request_json(
                &alice,
                Method::POST,
                &versions,
                Some(version_key),
                Some(version_body.clone())
            ),
            request_json(
                &alice,
                Method::POST,
                &versions,
                Some(version_key),
                Some(version_body.clone())
            )
        );
        assert_eq!(first_version.0, StatusCode::CREATED);
        assert_eq!(second_version, first_version);
        let version_result = first_version.1;
        let revised_version = uuid_field(&version_result, "result_id");

        let restore_key = Uuid::new_v4();
        let restore_body = serde_json::json!({
            "expected_current_version_id": revised_version,
            "reason": "Restore the starting point"
        });
        let restore_path = format!("{versions}/{initial_version}/restore");
        let (first_restore, second_restore) = tokio::join!(
            request_json(
                &alice,
                Method::POST,
                &restore_path,
                Some(restore_key),
                Some(restore_body.clone())
            ),
            request_json(
                &alice,
                Method::POST,
                &restore_path,
                Some(restore_key),
                Some(restore_body.clone())
            )
        );
        assert_eq!(first_restore.0, StatusCode::CREATED);
        assert_eq!(second_restore, first_restore);
        let restore_result = first_restore.1;

        let remove_key = Uuid::new_v4();
        let remove_body = serde_json::json!({"expected_document_version": 7});
        let (status, remove_result) = request_json(
            &owner,
            Method::DELETE,
            &alice_participant,
            Some(remove_key),
            Some(remove_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(remove_result["operation"], "participant_remove");
        let (status, _) = request_json(&alice, Method::GET, &document_path, None, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let replay_version = request_json(
            &alice,
            Method::POST,
            &versions,
            Some(version_key),
            Some(version_body),
        )
        .await;
        assert_eq!(replay_version, (StatusCode::CREATED, version_result));
        let replay_restore = request_json(
            &alice,
            Method::POST,
            &restore_path,
            Some(restore_key),
            Some(restore_body),
        )
        .await;
        assert_eq!(replay_restore, (StatusCode::CREATED, restore_result));
        let replay_create = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(create_key),
            Some(create.clone()),
        )
        .await;
        assert_eq!(replay_create, (StatusCode::CREATED, created));
        let replay_metadata = request_json(
            &owner,
            Method::PATCH,
            &document_path,
            Some(metadata_key),
            Some(metadata),
        )
        .await;
        assert_eq!(replay_metadata, (StatusCode::OK, metadata_result));
        let replay_read = request_json(
            &owner,
            Method::PUT,
            &alice_participant,
            Some(read_key),
            Some(read_body),
        )
        .await;
        assert_eq!(replay_read, (StatusCode::OK, read_result));
        let replay_remove = request_json(
            &owner,
            Method::DELETE,
            &alice_participant,
            Some(remove_key),
            Some(remove_body),
        )
        .await;
        assert_eq!(replay_remove, (StatusCode::OK, remove_result));
        let (status, final_participants) =
            request_json(&owner, Method::GET, &participants, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(final_participants["items"].as_array().unwrap().len(), 1);

        let cross_actor = request_json(
            &alice,
            Method::POST,
            &documents,
            Some(create_key),
            Some(create),
        )
        .await;
        assert_eq!(cross_actor.0, StatusCode::CONFLICT);
        let other_owner = fixture.app("owner", "owner", &fixture.other_company);
        let (status, _) = request_json(
            &other_owner,
            Method::GET,
            format!(
                "/companies/{}/documents/{document}/participants",
                fixture.other_company
            ),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let guessed_document = Uuid::new_v4();
        let (status, _) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{guessed_document}/participants"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let oversized_actor = "x".repeat(256);
        let (status, _) = request_json(
            &owner,
            Method::PUT,
            format!("{participants}/{oversized_actor}"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_document_version": 8,
                "access": "read"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn version_history_is_metadata_only_and_full_content_is_single_version_scoped() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Document version bound scenario");
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);
        let large_text = "x".repeat(300_000);
        let body = serde_json::json!({
            "title": "Large evidence",
            "kind": "brief",
            "visibility": "participants",
            "linked_room_id": null,
            "inherit_room_visibility": false,
            "content_json": content("large", &large_text),
            "reason": "Bound history responses"
        });
        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = uuid_field(&created, "document_id");
        let document_path = format!("{documents}/{document}");
        let (status, document_view) =
            request_json(&owner, Method::GET, &document_path, None, None).await;
        assert_eq!(status, StatusCode::OK);
        let version = document_view["current_version"]["version"]["id"]
            .as_str()
            .unwrap();
        let versions = format!("{document_path}/versions");
        let (status, history) = request_json(&owner, Method::GET, &versions, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(history.to_string().len() < 8_192);
        let summary = &history["items"][0];
        for body_field in ["content_json", "plain_text", "rendered_html", "markdown"] {
            assert!(
                summary.get(body_field).is_none(),
                "history leaked {body_field}"
            );
        }
        let (status, _) = request_json(
            &owner,
            Method::GET,
            format!("{versions}?limit=26"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, full_version) = request_json(
            &owner,
            Method::GET,
            format!("{versions}/{version}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            full_version["version"]["content_json"]["content"][0]["content"][0]["text"]
                .as_str()
                .unwrap()
                .len(),
            large_text.len()
        );
    }

    #[tokio::test]
    async fn document_routes_replay_commands_and_complete_comment_review_and_proposal_flows() {
        let Some(fixture) = DocumentRouteFixture::new().await else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping Document route workflow scenario"
            );
            return;
        };
        let owner = fixture.app("owner", "owner", &fixture.company);
        let documents = format!("/companies/{}/documents", fixture.company);
        let create_key = Uuid::new_v4();
        let request_body = create_body("Evidence plan");
        let (status, created) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(create_key),
            Some(request_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let document = Uuid::parse_str(created["document_id"].as_str().unwrap()).unwrap();
        let (_, initial_view) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        let initial_version = Uuid::parse_str(
            initial_view["current_version"]["version"]["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();

        let (status, replay) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(create_key),
            Some(request_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(replay, created);
        let (status, conflict) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(create_key),
            Some(create_body("Different meaning")),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(conflict["error"], "document_conflict");

        let (status, second_document) = request_json(
            &owner,
            Method::POST,
            &documents,
            Some(Uuid::new_v4()),
            Some(create_body("Second evidence plan")),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let second_document_id = second_document["document_id"].as_str().unwrap();
        let (status, first_document_page) = request_json(
            &owner,
            Method::GET,
            format!("{documents}?limit=1"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first_document_page["items"].as_array().unwrap().len(), 1);
        assert_eq!(
            first_document_page["items"][0]["document"]["id"],
            second_document_id
        );
        let list_cursor = &first_document_page["next_cursor"];
        let (status, second_document_page) = request_json(
            &owner,
            Method::GET,
            format!(
                "{documents}?limit=1&before_updated_at={}&before_document_id={}",
                list_cursor["updated_at"].as_str().unwrap(),
                list_cursor["id"].as_str().unwrap()
            ),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second_document_page["items"].as_array().unwrap().len(), 1);
        assert_eq!(
            second_document_page["items"][0]["document"]["id"],
            document.to_string()
        );

        let metadata_key = Uuid::new_v4();
        let metadata_body = serde_json::json!({
            "expected_version": 1,
            "title": "Evidence plan refined",
            "kind": "plan",
            "visibility": "participants",
            "linked_room_id": null,
            "inherit_room_visibility": false
        });
        let (status, updated) = request_json(
            &owner,
            Method::PATCH,
            format!("{documents}/{document}"),
            Some(metadata_key),
            Some(metadata_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated["operation"], "document_metadata_update");
        let (status, replay) = request_json(
            &owner,
            Method::PATCH,
            format!("{documents}/{document}"),
            Some(metadata_key),
            Some(metadata_body),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(replay, updated);
        let (status, updated_view) = request_json(
            &owner,
            Method::GET,
            format!("{documents}/{document}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated_view["document"]["title"], "Evidence plan refined");
        assert_eq!(updated_view["document"]["version"], 2);

        let versions = format!("{documents}/{document}/versions");
        let version_key = Uuid::new_v4();
        let version_body = serde_json::json!({
            "expected_current_version_id": initial_version,
            "content_json": content("claim", "Revised evidence"),
            "reason": "Ready for review"
        });
        let (status, revised) = request_json(
            &owner,
            Method::POST,
            &versions,
            Some(version_key),
            Some(version_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let revised_version = Uuid::parse_str(revised["result_id"].as_str().unwrap()).unwrap();
        let (status, replay) = request_json(
            &owner,
            Method::POST,
            &versions,
            Some(version_key),
            Some(version_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(replay, revised);

        let (status, first_page) = request_json(
            &owner,
            Method::GET,
            format!("{versions}?limit=1"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first_page["items"].as_array().unwrap().len(), 1);
        assert!(first_page["items"][0].get("content_json").is_none());
        assert!(first_page["items"][0].get("plain_text").is_none());
        assert_eq!(first_page["next_cursor"]["version_number"], 2);
        let (status, second_page) = request_json(
            &owner,
            Method::GET,
            format!("{versions}?limit=1&before_version_number=2"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second_page["items"][0]["id"], initial_version.to_string());

        let comments = format!("{documents}/{document}/comments");
        let comment_key = Uuid::new_v4();
        let comment_body = serde_json::json!({
            "block_id": "claim",
            "content_json": content("comment", "Please verify the evidence")
        });
        let (status, comment) = request_json(
            &owner,
            Method::POST,
            &comments,
            Some(comment_key),
            Some(comment_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let thread = Uuid::parse_str(comment["thread"]["thread"]["id"].as_str().unwrap()).unwrap();
        let (status, comment_replay) = request_json(
            &owner,
            Method::POST,
            &comments,
            Some(comment_key),
            Some(comment_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(comment_replay["thread"]["thread"]["id"], thread.to_string());

        let replies = format!("{comments}/{thread}/replies");
        let (status, reply) = request_json(
            &owner,
            Method::POST,
            &replies,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({"content_json": content("reply", "Verified")})),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(reply["thread_id"], thread.to_string());
        let (status, resolved) = request_json(
            &owner,
            Method::POST,
            format!("{comments}/{thread}/resolve"),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({"expected_thread_version": 1})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(resolved["status"], "resolved");

        let reviews = format!("{documents}/{document}/reviews");
        let review_one_key = Uuid::new_v4();
        let review_one_body = serde_json::json!({
            "expected_document_version": 3,
            "expected_current_version_id": revised_version,
            "summary": "Review the evidence"
        });
        let (status, review_one) = request_json(
            &owner,
            Method::POST,
            &reviews,
            Some(review_one_key),
            Some(review_one_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let review_one_id = Uuid::parse_str(review_one["id"].as_str().unwrap()).unwrap();

        let (status, advanced) = request_json(
            &owner,
            Method::POST,
            &versions,
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_current_version_id": revised_version,
                "content_json": content("claim", "Final evidence"),
                "reason": "Review rollover"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let final_version = Uuid::parse_str(advanced["result_id"].as_str().unwrap()).unwrap();
        let review_two_key = Uuid::new_v4();
        let review_two_body = serde_json::json!({
            "expected_document_version": 5,
            "expected_current_version_id": final_version,
            "summary": "Review the final evidence"
        });
        let (status, review_two) = request_json(
            &owner,
            Method::POST,
            &reviews,
            Some(review_two_key),
            Some(review_two_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let review_two_id = Uuid::parse_str(review_two["id"].as_str().unwrap()).unwrap();
        let (status, review_two_replay) = request_json(
            &owner,
            Method::POST,
            &reviews,
            Some(review_two_key),
            Some(review_two_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(review_two_replay["id"], review_two_id.to_string());
        assert_eq!(review_two_replay["status"], "requested");

        let (status, review_one_after) = request_json(
            &owner,
            Method::GET,
            format!("{reviews}/{review_one_id}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(review_one_after["status"], "stale");

        let review_accept_key = Uuid::new_v4();
        let review_accept_body = serde_json::json!({
            "expected_document_version": 6,
            "expected_review_version": 1,
            "accepted_version_name": "Accepted evidence"
        });
        let (status, accepted_review) = request_json(
            &owner,
            Method::POST,
            format!("{reviews}/{review_two_id}/accept"),
            Some(review_accept_key),
            Some(review_accept_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(accepted_review["review"]["status"], "accepted");
        let accepted_version = Uuid::parse_str(
            accepted_review["accepted_version"]["version"]["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let (status, accepted_review_replay) = request_json(
            &owner,
            Method::POST,
            format!("{reviews}/{review_two_id}/accept"),
            Some(review_accept_key),
            Some(review_accept_body),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            accepted_review_replay["accepted_version"]["version"]["id"],
            accepted_version.to_string()
        );

        let current = fixture
            .org
            .get_document_for_actor(document, "owner")
            .await
            .unwrap();
        fixture
            .org
            .set_document_participant(restless_orgintel::SetDocumentParticipant {
                command_id: Uuid::new_v4(),
                document_id: document,
                actor_id: "owner",
                expected_document_version: current.document.version,
                participant_actor_id: "research-analyst",
                access: restless_orgintel::DocumentAccess::Edit,
            })
            .await
            .unwrap();
        let proposed_content = content("claim", "Agent-proposed evidence");
        let proposal = fixture
            .org
            .propose_document_revision(restless_orgintel::ProposeDocumentRevision {
                document_id: document,
                actor_id: "research-analyst",
                command_id: Uuid::new_v4(),
                base_version_id: accepted_version,
                scope: restless_orgintel::DocumentRevisionScope::WholeDocument,
                block_id: None,
                proposed_content_json: &proposed_content,
                summary: "Tighten the evidence",
            })
            .await
            .unwrap();
        let proposals = format!("{documents}/{document}/proposals");
        let agent = fixture.app("research-analyst", "member", &fixture.company);
        let (status, body) = request_json(
            &agent,
            Method::POST,
            format!("{proposals}/{}/accept", proposal.id),
            Some(Uuid::new_v4()),
            Some(serde_json::json!({
                "expected_proposal_version": 1,
                "resolution_summary": "Agent cannot self-accept",
                "accepted_version_name": "Invalid"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "request_principal");

        let proposal_accept_key = Uuid::new_v4();
        let proposal_accept_body = serde_json::json!({
            "expected_proposal_version": 1,
            "resolution_summary": "Accepted after review",
            "accepted_version_name": "Agent evidence accepted"
        });
        let (status, accepted) = request_json(
            &owner,
            Method::POST,
            format!("{proposals}/{}/accept", proposal.id),
            Some(proposal_accept_key),
            Some(proposal_accept_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(accepted["proposal"]["status"], "accepted");
        let agent_version = Uuid::parse_str(
            accepted["accepted_version"]["version"]["id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let (status, accepted_replay) = request_json(
            &owner,
            Method::POST,
            format!("{proposals}/{}/accept", proposal.id),
            Some(proposal_accept_key),
            Some(proposal_accept_body),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            accepted_replay["accepted_version"]["version"]["id"],
            agent_version.to_string()
        );

        let rejected_content = content("claim", "A proposal we should decline");
        let rejected_proposal = fixture
            .org
            .propose_document_revision(restless_orgintel::ProposeDocumentRevision {
                document_id: document,
                actor_id: "research-analyst",
                command_id: Uuid::new_v4(),
                base_version_id: agent_version,
                scope: restless_orgintel::DocumentRevisionScope::WholeDocument,
                block_id: None,
                proposed_content_json: &rejected_content,
                summary: "Remove necessary evidence",
            })
            .await
            .unwrap();
        let reject_key = Uuid::new_v4();
        let reject_body = serde_json::json!({
            "expected_proposal_version": 1,
            "resolution_summary": "The evidence is necessary"
        });
        let (status, rejected) = request_json(
            &owner,
            Method::POST,
            format!("{proposals}/{}/reject", rejected_proposal.id),
            Some(reject_key),
            Some(reject_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(rejected["proposal"]["status"], "rejected");
        assert!(rejected["accepted_version"].is_null());
        let (status, rejected_replay) = request_json(
            &owner,
            Method::POST,
            format!("{proposals}/{}/reject", rejected_proposal.id),
            Some(reject_key),
            Some(reject_body),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(rejected_replay["proposal"]["status"], "rejected");

        let restore_key = Uuid::new_v4();
        let restore_body = serde_json::json!({
            "expected_current_version_id": agent_version,
            "reason": "Restore the original evidence"
        });
        let (status, restored) = request_json(
            &owner,
            Method::POST,
            format!("{versions}/{initial_version}/restore"),
            Some(restore_key),
            Some(restore_body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let restored_version = restored["result_id"].as_str().unwrap().to_string();
        let (status, restored_replay) = request_json(
            &owner,
            Method::POST,
            format!("{versions}/{initial_version}/restore"),
            Some(restore_key),
            Some(restore_body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(restored_replay, restored);
        let (status, restored_view) = request_json(
            &owner,
            Method::GET,
            format!("{versions}/{restored_version}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            restored_view["version"]["restored_from_version_id"],
            initial_version.to_string()
        );
    }
}
