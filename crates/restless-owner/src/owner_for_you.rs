//! For you, document and sheet follows, and the owner Inbox's own marks (S65) for the owner and
//! every member, over the verified company principal. A thin transport: OrgIntel assembles the
//! items and decides access; this module only turns their targets into cockpit links.

use super::*;

use restless_orgintel::{
    DocumentError, DocumentReadView, ForYouCursor, ForYouItem, ForYouTarget, InboxMarkChange,
};

const DEFAULT_FOR_YOU_PAGE: i64 = 50;

pub(super) fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    RoomApiState: FromRef<S>,
{
    Router::<S>::new()
        .route("/companies/{company}/for-you", get(list_for_you))
        .route("/companies/{company}/for-you/read", post(mark_for_you_read))
        .route("/companies/{company}/for-you/unread", post(mark_for_you_unread))
        .route("/companies/{company}/for-you/archive", post(archive_for_you))
        .route(
            "/companies/{company}/documents/{document}/follow",
            axum::routing::put(follow_document).delete(unfollow_document),
        )
        .route(
            "/companies/{company}/sheets/{sheet}/follow",
            get(sheet_follow_state)
                .put(follow_sheet)
                .delete(unfollow_sheet),
        )
        .route(
            "/companies/{company}/inbox-marks",
            get(list_inbox_marks).put(set_inbox_marks),
        )
        .layer(DefaultBodyLimit::max(16 * 1024))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForYouQuery {
    before: Option<String>,
    limit: Option<i64>,
    /// Only what is unread.
    #[serde(default)]
    unread: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForYouKeysInput {
    keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForYouArchiveInput {
    keys: Vec<String>,
    #[serde(default = "archived_default")]
    archived: bool,
}

fn archived_default() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InboxMarkInput {
    item_id: String,
    #[serde(default)]
    seen_stamp: Option<String>,
    /// Absent keeps the snooze; null wakes the item; a time snoozes it until then.
    #[serde(default, deserialize_with = "present")]
    snoozed_until: Option<Option<DateTime<Utc>>>,
}

fn present<'de, D>(deserializer: D) -> std::result::Result<Option<Option<DateTime<Utc>>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<DateTime<Utc>>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InboxMarksInput {
    marks: Vec<InboxMarkInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ForYouReadInput {
    #[serde(default)]
    keys: Vec<String>,
    #[serde(default)]
    all: bool,
}

#[derive(Debug, Serialize)]
struct ForYouItemView {
    key: String,
    kind: String,
    title: String,
    excerpt: String,
    href: String,
    actor_id: String,
    actor_name: String,
    at: DateTime<Utc>,
    unread: bool,
}

fn no_store(mut response: Response<Body>) -> Response<Body> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn for_you_error(error: DocumentError) -> Response<Body> {
    no_store(match error {
        DocumentError::Invalid(message) => {
            api_error(StatusCode::UNPROCESSABLE_ENTITY, "for_you", message)
        }
        DocumentError::Unavailable => api_error(
            StatusCode::NOT_FOUND,
            "document",
            "the Document is unavailable",
        ),
        error => {
            tracing::error!(%error, "For you operation failed");
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company collaboration is temporarily unavailable",
            )
        }
    })
}

/// "Bianca", "Bianca and Exec", "Bianca, Carol and 2 others".
fn actor_names(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [one, two] => format!("{one} and {two}"),
        [one, two, three] => format!("{one}, {two} and {three}"),
        [one, two, rest @ ..] => format!("{one}, {two} and {} others", rest.len()),
    }
}

/// The cockpit link for an item, relative to the site root. The hosted email uses it too.
pub(super) fn href(company: &str, item: &ForYouItem) -> String {
    // Company names are already path-safe identifiers.
    match &item.target {
        ForYouTarget::Document { document_id } => {
            // Comments open beside the document and a review beside its request; a change or a
            // share opens the document itself, which says what changed.
            let panel = match item.kind.as_str() {
                "change" | "share" => "",
                "review" => "&inspect=review",
                _ => "&inspect=comments",
            };
            format!("/{company}/library/documents?document={document_id}{panel}")
        }
        ForYouTarget::Sheet { sheet_id } => format!("/{company}/library/sheets?sheet={sheet_id}"),
        ForYouTarget::Room {
            room_id,
            thread_root_message_id,
            message_id,
            mention_id,
            ..
        } => match mention_id {
            Some(mention) => format!(
                "/{company}/people?room={room_id}&thread={thread_root_message_id}&message={message_id}&mention={mention}"
            ),
            // A reply in a thread the person is in opens the thread itself. An exact-message
            // link needs its mention; without one the Room calls the address invalid.
            None => format!("/{company}/people?room={room_id}&thread={thread_root_message_id}"),
        },
    }
}

pub(super) fn actor_phrase(names: &[String]) -> String {
    actor_names(names)
}

fn item_view(company: &str, item: ForYouItem) -> ForYouItemView {
    let href = href(company, &item);
    // A direct conversation has no title worth repeating; the person is the context.
    let title = match &item.target {
        ForYouTarget::Room { room_kind, .. } if room_kind == "direct" => String::new(),
        _ => item.title,
    };
    ForYouItemView {
        key: item.key,
        kind: item.kind,
        title,
        excerpt: item.excerpt,
        href,
        actor_id: item.actor_ids.first().cloned().unwrap_or_default(),
        actor_name: actor_names(&item.actor_names),
        at: item.at,
        unread: item.unread,
    }
}

async fn list_for_you(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<ForYouQuery>,
) -> Response<Body> {
    let before = match query.before.as_deref() {
        None | Some("") => None,
        Some(value) => match ForYouCursor::parse(value) {
            Some(cursor) => Some(cursor),
            None => {
                return no_store(api_error(
                    StatusCode::BAD_REQUEST,
                    "for_you_cursor",
                    "before must be a next_before value from an earlier page",
                ))
            }
        },
    };
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    match org
        .for_you_page_filtered(
            principal.actor_id(),
            before.as_ref(),
            query.limit.unwrap_or(DEFAULT_FOR_YOU_PAGE),
            query.unread,
        )
        .await
    {
        Ok(page) => no_store(
            Json(serde_json::json!({
                "items": page
                    .items
                    .into_iter()
                    .map(|item| item_view(&company, item))
                    .collect::<Vec<_>>(),
                "unread_count": page.unread_count,
                "next_before": page.next_before.map(|cursor| cursor.encode()),
            }))
            .into_response(),
        ),
        Err(error) => for_you_error(error),
    }
}

async fn mark_for_you_read(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<ForYouReadInput>,
) -> Response<Body> {
    if input.all == !input.keys.is_empty() {
        return no_store(api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "for_you",
            "send either keys or all, not both",
        ));
    }
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    let result = if input.all {
        org.mark_all_for_you_read(principal.actor_id()).await
    } else {
        org.mark_for_you_read(principal.actor_id(), &input.keys)
            .await
    };
    match result {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => for_you_error(error),
    }
}

/// Marks items unread again for this person, on every device.
async fn mark_for_you_unread(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<ForYouKeysInput>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    match org
        .mark_for_you_unread(principal.actor_id(), &input.keys)
        .await
    {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => for_you_error(error),
    }
}

/// Archives items for this person, or brings them back.
async fn archive_for_you(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<ForYouArchiveInput>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    match org
        .archive_for_you(principal.actor_id(), &input.keys, input.archived)
        .await
    {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => for_you_error(error),
    }
}

fn sheet_follow_error(error: DocumentError) -> Response<Body> {
    match error {
        DocumentError::Unavailable => no_store(api_error(
            StatusCode::NOT_FOUND,
            "sheet",
            "the sheet is unavailable",
        )),
        error => for_you_error(error),
    }
}

async fn sheet_follow(
    state: RoomApiState,
    principal: RequestPrincipal,
    company: String,
    sheet: Uuid,
    following: Option<bool>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    let result = match following {
        Some(following) => org.set_sheet_follow(sheet, principal.actor_id(), following).await,
        None => org.sheet_following(sheet, principal.actor_id()).await,
    };
    match result {
        Ok(following) => no_store(Json(serde_json::json!({ "following": following })).into_response()),
        Err(error) => sheet_follow_error(error),
    }
}

async fn sheet_follow_state(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath((company, sheet)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    sheet_follow(state, principal, company, sheet, None).await
}

async fn follow_sheet(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath((company, sheet)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    sheet_follow(state, principal, company, sheet, Some(true)).await
}

async fn unfollow_sheet(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath((company, sheet)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    sheet_follow(state, principal, company, sheet, Some(false)).await
}

/// This person's own seen and snooze marks on the owner Inbox.
async fn list_inbox_marks(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    match org.inbox_marks(principal.actor_id()).await {
        Ok(marks) => no_store(Json(serde_json::json!({ "marks": marks })).into_response()),
        Err(error) => for_you_error(error),
    }
}

async fn set_inbox_marks(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<InboxMarksInput>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    let marks = input
        .marks
        .into_iter()
        .map(|mark| InboxMarkChange {
            item_id: mark.item_id,
            seen_stamp: mark.seen_stamp,
            snoozed_until: mark.snoozed_until,
        })
        .collect::<Vec<_>>();
    match org.set_inbox_marks(principal.actor_id(), &marks).await {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => for_you_error(error),
    }
}

async fn set_follow(
    state: RoomApiState,
    principal: RequestPrincipal,
    company: String,
    document: Uuid,
    following: bool,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return no_store(response),
    };
    match org
        .set_document_follow(document, principal.actor_id(), following)
        .await
    {
        Ok(state) => no_store(Json(serde_json::json!({ "following": state.following })).into_response()),
        Err(error) => for_you_error(error),
    }
}

async fn follow_document(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    set_follow(state, principal, company, document, true).await
}

async fn unfollow_document(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath((company, document)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    set_follow(state, principal, company, document, false).await
}

/// The document read view with this person's follow state and what changed since they last
/// opened it. If that cannot be read the document still opens, without them.
pub(super) async fn document_read_view(
    org: &restless_orgintel::OrgIntel,
    view: DocumentReadView,
    actor_id: &str,
) -> serde_json::Value {
    let document_id = view.document.id;
    let mut value = serde_json::to_value(view).unwrap_or(serde_json::Value::Null);
    match org.document_follow_state(document_id, actor_id).await {
        Ok(state) => {
            if let Some(object) = value.as_object_mut() {
                object.insert("following".into(), state.following.into());
                object.insert(
                    "changed_since_you_looked".into(),
                    state
                        .changed_since_you_looked
                        .map(|change| {
                            serde_json::json!({
                                "actor_ids": change.actor_ids,
                                "actor_name": actor_names(&change.actor_names),
                                "at": change.at,
                            })
                        })
                        .unwrap_or(serde_json::Value::Null),
                );
            }
        }
        Err(error) => tracing::warn!(%error, %document_id, "document follow state unavailable"),
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt as _;

    #[test]
    fn names_read_as_a_sentence() {
        let names = |list: &[&str]| actor_names(&list.iter().map(|n| n.to_string()).collect::<Vec<_>>());
        assert_eq!(names(&["Bianca"]), "Bianca");
        assert_eq!(names(&["Bianca", "Exec"]), "Bianca and Exec");
        assert_eq!(names(&["Alex", "Bianca", "Exec"]), "Alex, Bianca and Exec");
        assert_eq!(names(&["Alex", "Bianca", "Carol", "Exec"]), "Alex, Bianca and 2 others");
    }

    /// A Room mention opens its exact message; a reply in a thread the person is in opens the
    /// thread, since the Room refuses an exact-message address that names no mention.
    #[test]
    fn room_items_link_to_an_address_the_room_accepts() {
        let room_id = Uuid::from_u128(7);
        let item = |mention_id| ForYouItem {
            key: "k".into(),
            kind: "reply".into(),
            title: "Launch".into(),
            excerpt: String::new(),
            target: ForYouTarget::Room {
                room_id,
                room_kind: "group".into(),
                thread_root_message_id: 4,
                message_id: 9,
                mention_id,
            },
            actor_ids: vec![],
            actor_names: vec![],
            at: Utc::now(),
            unread: true,
        };
        assert_eq!(
            href("company_ab", &item(None)),
            format!("/company_ab/people?room={room_id}&thread=4")
        );
        let mention = Uuid::from_u128(8);
        assert_eq!(
            href("company_ab", &item(Some(mention))),
            format!("/company_ab/people?room={room_id}&thread=4&message=9&mention={mention}")
        );
    }

    async fn request(
        app: &Router,
        method: Method,
        uri: String,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = axum::http::Request::builder().method(method).uri(uri);
        let body = match body {
            Some(body) => {
                builder = builder.header(CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(&body).unwrap())
            }
            None => Body::empty(),
        };
        let response = app.clone().oneshot(builder.body(body).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
    }

    /// Each person reads only their own For you, inside their own company, and may follow only
    /// what they can read.
    #[tokio::test]
    async fn for_you_routes_answer_only_for_the_verified_person() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping For you routes");
            return;
        };
        let suffix = Uuid::new_v4().simple().to_string();
        let company = format!("foryou_api_{suffix}");
        let org = restless_orgintel::OrgIntel::ensure(&url, &company).await.unwrap();
        org.ensure_actor("owner", "owner", "owner", "The Owner").await.unwrap();
        for (id, name) in [("alex", "Alex"), ("bianca", "Bianca")] {
            org.ensure_actor(id, "human", "member", name).await.unwrap();
        }
        let content = |text: &str, mention: Option<&str>| {
            let mut inline = vec![serde_json::json!({"type": "text", "text": text})];
            if let Some(actor) = mention {
                inline.push(serde_json::json!({"type": "mention", "attrs": {"actor_id": actor, "label": actor}}));
            }
            serde_json::json!({"type": "doc", "content": [{"type": "paragraph", "attrs": {"block_id": "b"}, "content": inline}]})
        };
        let private = org
            .create_document(restless_orgintel::NewDocument {
                command_id: Uuid::new_v4(),
                title: "Board notes",
                kind: restless_orgintel::DocumentKind::Plan,
                visibility: restless_orgintel::DocumentVisibility::Participants,
                linked_room_id: None,
                inherit_room_visibility: false,
                owner_actor_id: "owner",
                created_by_actor_id: "owner",
                content_json: &content("Notes", None),
                reason: "First draft",
            })
            .await
            .unwrap()
            .document_id;
        org.set_document_participant(restless_orgintel::SetDocumentParticipant {
            command_id: Uuid::new_v4(),
            document_id: private,
            actor_id: "owner",
            expected_document_version: 1,
            participant_actor_id: "alex",
            access: restless_orgintel::DocumentAccess::Comment,
        })
        .await
        .unwrap();
        org.create_document_comment_thread(restless_orgintel::NewDocumentCommentThread {
            document_id: private,
            actor_id: "owner",
            command_id: Uuid::new_v4(),
            block_id: None,
            content_json: &content("Can you check this? ", Some("alex")),
        })
        .await
        .unwrap();

        let state = RoomApiState::fixed(HashMap::from([(company.clone(), org)]), url);
        let app = |actor: &str, role: &str| {
            let principal = RequestPrincipal::from_verified(&VerifiedIdentity {
                user: format!("user-{actor}"),
                issuer: None,
                owner: "fixture-owner".into(),
                scope: CompanyScope::Company { company: company.clone() },
                role: role.into(),
                actor: Some(actor.into()),
                company_id: None,
                cell_id: None,
                membership_id: None,
                membership_version: None,
                display_name: None,
            })
            .unwrap();
            routes::<RoomApiState>()
                .layer(Extension(principal))
                .with_state(state.clone())
        };
        let alex = app("alex", "member");
        let bianca = app("bianca", "member");
        let path = format!("/companies/{company}/for-you");

        let (status, body) = request(&alex, Method::GET, path.clone(), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["unread_count"], 2, "the mention, and being given the document");
        let shared = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["kind"] == "share")
            .expect("Alex hears he was given Board notes");
        assert_eq!(shared["excerpt"], "Can comment");
        assert_eq!(
            shared["href"],
            format!("/{company}/library/documents?document={private}")
        );
        let item = &body["items"][0];
        assert_eq!(item["kind"], "mention");
        assert_eq!(item["actor_name"], "The Owner");
        assert_eq!(item["title"], "Board notes");
        assert_eq!(
            item["href"],
            format!("/{company}/library/documents?document={private}&inspect=comments")
        );
        let (_, body) = request(&bianca, Method::GET, path.clone(), None).await;
        assert_eq!(body["items"].as_array().unwrap().len(), 0, "nothing of Alex's reaches Bianca");

        // Bianca cannot follow a document she cannot read, nor learn it exists.
        let follow = format!("/companies/{company}/documents/{private}/follow");
        let (status, _) = request(&bianca, Method::PUT, follow.clone(), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, body) = request(&alex, Method::DELETE, follow.clone(), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["following"], false);

        // Read state belongs to the person who read it.
        let read = format!("{path}/read");
        let key = item["key"].as_str().unwrap().to_string();
        let (status, _) =
            request(&bianca, Method::POST, read.clone(), Some(serde_json::json!({"keys": [key]}))).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(request(&alex, Method::GET, path.clone(), None).await.1["unread_count"], 2);
        let (status, _) = request(
            &bianca,
            Method::POST,
            format!("{path}/archive"),
            Some(serde_json::json!({"keys": [key.clone()]})),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(
            request(&alex, Method::GET, path.clone(), None).await.1["items"]
                .as_array()
                .unwrap()
                .len(),
            2,
            "Bianca's archive is her own"
        );
        let (_, body) = request(&alex, Method::GET, format!("{path}?unread=true"), None).await;
        assert_eq!(body["items"].as_array().unwrap().len(), 2);

        // Inbox marks are the person's own.
        let marks = format!("/companies/{company}/inbox-marks");
        let (status, _) = request(
            &alex,
            Method::PUT,
            marks.clone(),
            Some(serde_json::json!({"marks": [{"item_id": "handoff:1", "seen_stamp": "1", "snoozed_until": null}]})),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(request(&alex, Method::GET, marks.clone(), None).await.1["marks"][0]["item_id"], "handoff:1");
        assert_eq!(request(&bianca, Method::GET, marks, None).await.1["marks"], serde_json::json!([]));

        // A sheet nobody can open cannot be followed, and says nothing about whether it exists.
        let (status, _) = request(
            &alex,
            Method::PUT,
            format!("/companies/{company}/sheets/{}/follow", Uuid::new_v4()),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) =
            request(&alex, Method::POST, read.clone(), Some(serde_json::json!({"all": true}))).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(request(&alex, Method::GET, path.clone(), None).await.1["unread_count"], 0);
        // Marked unread again, it waits for Alex past "mark all read".
        let (status, _) = request(
            &alex,
            Method::POST,
            format!("{path}/unread"),
            Some(serde_json::json!({"keys": [key.clone()]})),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(request(&alex, Method::GET, path.clone(), None).await.1["unread_count"], 1);
        let (status, _) =
            request(&alex, Method::POST, read, Some(serde_json::json!({"keys": ["x:y"]}))).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        // Another company's path is refused before anything is read.
        let (status, _) =
            request(&alex, Method::GET, "/companies/elsewhere/for-you".into(), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
}
