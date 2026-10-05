//! Owner API tests: routes, sessions, projections and their wire contracts.
//! Kept at the indentation they had inside owner.rs, so string literals are
//! byte-for-byte unchanged.

    use crate::transcript::{split_attention_context, split_context_marker, OwnerIntentKind};
    use super::*;
    use axum::body::to_bytes;
    use sqlx::Connection as _;
    use tower::ServiceExt as _;

    #[test]
    fn create_company_request_accepts_an_omitted_model() {
        let input: CreateCompanyInput = serde_json::from_value(serde_json::json!({
            "name": "company_0123456789abcdef",
            "display_name": "Untitled company",
            "mission": ""
        }))
        .unwrap();
        assert_eq!(input.model, None);
    }

    #[tokio::test]
    async fn company_principal_exposes_only_a_verified_cache_partition() {
        let principal = RequestPrincipal::from_verified(&VerifiedIdentity {
            user: "user-alice".into(),
            issuer: Some("https://cloud.restless.test".into()),
            owner: "owner-1".into(),
            scope: CompanyScope::Company {
                company: "acme".into(),
            },
            role: "member".into(),
            actor: Some("alice".into()),
            company_id: Some(Uuid::new_v4()),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some("membership-1".into()),
            membership_version: Some(4),
        })
        .expect("verified human principal");
        let expected_partition = principal.cache_partition().to_string();
        let app = Router::new()
            .route("/companies/{company}/principal", get(company_principal))
            .layer(Extension(principal));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/companies/acme/principal")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CACHE_CONTROL).unwrap(),
            HeaderValue::from_static("no-store")
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["actor_id"], "alice");
        assert_eq!(body["membership_role"], "member");
        assert_eq!(body["cache_partition"], expected_partition);
        assert_eq!(body.as_object().unwrap().len(), 3);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/companies/other/principal")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    struct RoomRouteFixture {
        state: RoomApiState,
        company: String,
        other_company: String,
        org: restless_orgintel::OrgIntel,
    }

    impl RoomRouteFixture {
        async fn new() -> Option<Self> {
            let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
            let suffix = Uuid::new_v4().simple().to_string();
            let company = format!("room_api_a_{suffix}");
            let other_company = format!("room_api_b_{suffix}");

            let mut handles = HashMap::new();
            for name in [&company, &other_company] {
                let org = restless_orgintel::OrgIntel::ensure(&database_url, name)
                    .await
                    .expect("ensure Room route fixture company");
                org.ensure_actor("owner", "owner", "owner", "The Owner")
                    .await
                    .unwrap();
                org.ensure_actor("exec", "exec", "exec", "The Exec")
                    .await
                    .unwrap();
                for (actor, display) in [
                    ("alice", "Alice"),
                    ("bob", "Bob"),
                    ("carol", "Carol"),
                    ("mallory", "Mallory"),
                ] {
                    org.ensure_actor(actor, "human", "member", display)
                        .await
                        .unwrap();
                }
                handles.insert(name.to_string(), org);
            }
            let org = handles.get(&company).expect("primary company").clone();
            Some(Self {
                state: RoomApiState::fixed(handles, database_url),
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
            self.app_with_state(self.state.clone(), actor, role, company)
        }

        fn app_with_state(
            &self,
            state: RoomApiState,
            actor: &str,
            role: &str,
            company: &str,
        ) -> Router {
            let principal = RequestPrincipal::from_verified(&Self::identity(actor, role, company))
                .expect("verified fixture principal");
            room_api_routes::<RoomApiState>()
                .layer(Extension(principal))
                .with_state(state)
        }

        fn network_app(&self, lease: SessionLease) -> Router {
            self.network_app_with_state(self.state.clone(), lease)
        }

        fn network_app_with_state(&self, mut state: RoomApiState, lease: SessionLease) -> Router {
            let principal = RequestPrincipal::from_verified(&lease.identity)
                .expect("verified fixture principal");
            state.network_mode = true;
            room_api_routes::<RoomApiState>()
                .layer(Extension(principal))
                .layer(Extension(lease))
                .with_state(state)
        }

        fn network_app_without_lease(&self, actor: &str, role: &str, company: &str) -> Router {
            let principal = RequestPrincipal::from_verified(&Self::identity(actor, role, company))
                .expect("verified fixture principal");
            let mut state = self.state.clone();
            state.network_mode = true;
            room_api_routes::<RoomApiState>()
                .layer(Extension(principal))
                .with_state(state)
        }

        fn unauthenticated_app(&self) -> Router {
            room_api_routes::<RoomApiState>().with_state(self.state.clone())
        }
    }

    async fn room_request(
        app: &Router,
        method: Method,
        uri: impl AsRef<str>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = axum::http::Request::builder()
            .method(method)
            .uri(uri.as_ref());
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
            .expect("Room route response");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read Room route response");
        let body = serde_json::from_slice(&bytes).unwrap_or_else(
            |_| serde_json::json!({ "raw": String::from_utf8_lossy(&bytes).to_string() }),
        );
        (status, body)
    }

    async fn room_get_response(
        app: &Router,
        uri: impl AsRef<str>,
        last_event_id: Option<&str>,
    ) -> Response<Body> {
        let mut builder = axum::http::Request::builder()
            .method(Method::GET)
            .uri(uri.as_ref());
        if let Some(last_event_id) = last_event_id {
            builder = builder.header("last-event-id", last_event_id);
        }
        app.clone()
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .expect("Room GET response")
    }

    /// A Room stream's frames without the opening comment, which exists only
    /// to push the response head through buffering proxies.
    fn room_sse_frames(
        response: Response<Body>,
    ) -> std::pin::Pin<
        Box<dyn futures_util::Stream<Item = Result<axum::body::Bytes, axum::Error>> + Send>,
    > {
        Box::pin(response.into_body().into_data_stream().filter(|frame| {
            let opening = matches!(frame, Ok(bytes) if std::str::from_utf8(bytes)
                .is_ok_and(|text| text.trim().trim_start_matches(':').trim() == "connected"));
            std::future::ready(!opening)
        }))
    }

    async fn first_sse_chunk(response: Response<Body>) -> String {
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers()[CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/event-stream"));
        let mut stream = room_sse_frames(response);
        let bytes = tokio::time::timeout(Duration::from_secs(2), stream.next())
            .await
            .expect("SSE emits within the bound")
            .expect("SSE remains open for its first event")
            .expect("SSE body frame");
        String::from_utf8(bytes.to_vec()).expect("SSE is UTF-8")
    }

    fn room_id(response: &serde_json::Value) -> Uuid {
        Uuid::parse_str(response["id"].as_str().expect("Room response id")).unwrap()
    }

    #[tokio::test]
    async fn room_routes_require_authentication_company_scope_and_participation() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room route security scenario");
            return;
        };
        let company_rooms = format!("/companies/{}/rooms", fixture.company);
        let (status, body) = room_request(
            &fixture.unauthenticated_app(),
            Method::GET,
            &company_rooms,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"], "no_session");

        let alice = fixture.app("alice", "member", &fixture.company);
        let (status, body) = room_request(
            &alice,
            Method::GET,
            format!("/companies/{}/rooms", fixture.other_company),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"], "company_out_of_scope");

        let private = fixture
            .org
            .create_room(
                "owner",
                restless_orgintel::RoomKind::Group,
                "Owner and Bob",
                &["bob"],
                "attachment-owner-bob-room",
            )
            .await
            .unwrap();
        let private_message = fixture
            .org
            .send_room_message(private.id, "owner", "Private", None, "private-root")
            .await
            .unwrap();
        for path in [
            format!(
                "/companies/{}/rooms/{}/participants",
                fixture.company, private.id
            ),
            format!(
                "/companies/{}/rooms/{}/messages",
                fixture.company, private.id
            ),
        ] {
            let (status, body) = room_request(&alice, Method::GET, path, None).await;
            assert_eq!(status, StatusCode::FORBIDDEN);
            assert_eq!(body["error"], "room_access");
        }

        let denied_operations = [
            (
                Method::POST,
                format!(
                    "/companies/{}/rooms/{}/messages",
                    fixture.company, private.id
                ),
                Some(serde_json::json!({
                    "body": "Not a participant",
                    "command_id": "nonparticipant-send"
                })),
            ),
            (
                Method::POST,
                format!(
                    "/companies/{}/rooms/{}/messages/{}/replies",
                    fixture.company, private.id, private_message.message.id
                ),
                Some(serde_json::json!({
                    "body": "Not a participant",
                    "command_id": "nonparticipant-reply"
                })),
            ),
            (
                Method::POST,
                format!(
                    "/companies/{}/rooms/{}/read-cursor",
                    fixture.company, private.id
                ),
                Some(serde_json::json!({
                    "through_message_id": private_message.message.id
                })),
            ),
            (
                Method::DELETE,
                format!(
                    "/companies/{}/rooms/{}/participants/bob",
                    fixture.company, private.id
                ),
                None,
            ),
        ];
        for (method, path, request_body) in denied_operations {
            let (status, body) = room_request(&alice, method, path, request_body).await;
            assert_eq!(status, StatusCode::FORBIDDEN);
            assert_eq!(body["error"], "room_access");
        }
    }

    #[tokio::test]
    async fn room_list_route_requires_an_exact_bounded_keyset_cursor() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room list route scenario");
            return;
        };
        for index in 0..3 {
            fixture
                .org
                .create_room(
                    "owner",
                    restless_orgintel::RoomKind::Group,
                    &format!("Alice room {index}"),
                    &["alice"],
                    &format!("alice-room-page-{index}"),
                )
                .await
                .unwrap();
        }
        let alice = fixture.app("alice", "member", &fixture.company);
        let rooms_path = format!("/companies/{}/rooms", fixture.company);

        let (status, first) =
            room_request(&alice, Method::GET, format!("{rooms_path}?limit=1"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first["rooms"].as_array().unwrap().len(), 1);
        assert_eq!(first["has_more"], true);
        let first_id = first["rooms"][0]["id"].as_str().unwrap();
        let before_created_at = first["next_before_created_at"].as_str().unwrap();
        let before_room_id = first["next_before_room_id"].as_str().unwrap();

        let (status, second) = room_request(
            &alice,
            Method::GET,
            format!(
                "{rooms_path}?limit=1&before_created_at={before_created_at}&before_room_id={before_room_id}"
            ),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second["rooms"].as_array().unwrap().len(), 1);
        assert_ne!(second["rooms"][0]["id"], first_id);

        for invalid_path in [
            format!("{rooms_path}?limit=0"),
            format!("{rooms_path}?limit=101"),
            format!("{rooms_path}?before_room_id={before_room_id}"),
            format!("{rooms_path}?before_created_at=not-a-time&before_room_id={before_room_id}"),
        ] {
            let (status, body) = room_request(&alice, Method::GET, invalid_path, None).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert!(matches!(
                body["error"].as_str(),
                Some("room_limit" | "room_cursor")
            ));
        }
    }

    #[tokio::test]
    async fn room_message_routes_derive_sender_and_preserve_retry_thread_and_page_semantics() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room message route scenario");
            return;
        };
        let alice = fixture.app("alice", "member", &fixture.company);
        let bob = fixture.app("bob", "member", &fixture.company);
        let rooms_path = format!("/companies/{}/rooms", fixture.company);
        let create_room_command = serde_json::json!({
            "kind": "group",
            "title": "Launch",
            "command_id": "http-launch-room",
            "participant_actor_ids": ["bob"]
        });
        let (status, created_room) = room_request(
            &alice,
            Method::POST,
            &rooms_path,
            Some(create_room_command.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let room = room_id(&created_room);
        let (status, replayed_room) =
            room_request(&alice, Method::POST, &rooms_path, Some(create_room_command)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(room_id(&replayed_room), room);
        let (status, conflict) = room_request(
            &alice,
            Method::POST,
            &rooms_path,
            Some(serde_json::json!({
                "kind": "group",
                "title": "Different semantics",
                "command_id": "http-launch-room",
                "participant_actor_ids": ["bob"]
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(conflict["error"], "room_command");
        let response = room_get_response(&alice, &rooms_path, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let cursor_path = format!("/companies/{}/rooms/{room}/read-cursor", fixture.company);
        let (status, unread) = room_request(&bob, Method::GET, &cursor_path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(unread["actor_id"], "bob");
        assert!(unread["cursor"].is_null());

        let messages_path = format!("/companies/{}/rooms/{room}/messages", fixture.company);

        let (status, _) = room_request(
            &alice,
            Method::POST,
            &messages_path,
            Some(serde_json::json!({
                "body": "x".repeat(129 * 1024),
                "command_id": "oversize"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

        let first_command = serde_json::json!({
            "body": "First",
            "command_id": "send-first"
        });
        let (status, first) = room_request(
            &alice,
            Method::POST,
            &messages_path,
            Some(first_command.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(first["created"], true);
        assert_eq!(first["message"]["from_actor"], "alice");
        let first_id = first["message"]["id"].as_i64().unwrap();

        let (status, duplicate) =
            room_request(&alice, Method::POST, &messages_path, Some(first_command)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(duplicate["created"], false);
        assert_eq!(duplicate["message"]["id"], first_id);

        let (status, conflict) = room_request(
            &alice,
            Method::POST,
            &messages_path,
            Some(serde_json::json!({
                "body": "Different payload",
                "command_id": "send-first"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(conflict["error"], "room_command");

        let (status, invalid) = room_request(
            &alice,
            Method::POST,
            &messages_path,
            Some(serde_json::json!({
                "body": "No retry identity",
                "command_id": ""
            })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(invalid["error"], "room");

        let (status, _) = room_request(
            &alice,
            Method::POST,
            &messages_path,
            Some(serde_json::json!({
                "body": "Forged",
                "command_id": "forged-sender",
                "author_actor": "owner"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let reply_path = format!(
            "/companies/{}/rooms/{room}/messages/{first_id}/replies",
            fixture.company
        );
        let (status, reply) = room_request(
            &bob,
            Method::POST,
            reply_path,
            Some(serde_json::json!({
                "body": "Reply",
                "command_id": "reply-first"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(reply["message"]["from_actor"], "bob");
        assert_eq!(reply["message"]["parent_message_id"], first_id);
        assert_eq!(reply["message"]["thread_root_message_id"], first_id);

        for (command_id, body) in [("send-second", "Second"), ("send-third", "Third")] {
            let (status, _) = room_request(
                &alice,
                Method::POST,
                &messages_path,
                Some(serde_json::json!({
                    "body": body,
                    "command_id": command_id
                })),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);
        }

        let (status, first_page) = room_request(
            &alice,
            Method::GET,
            format!("{messages_path}?limit=2"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first_page["messages"].as_array().unwrap().len(), 2);
        assert_eq!(first_page["has_more"], true);
        let page_cursor = first_page["next_before_message_id"].as_i64().unwrap();
        let (status, second_page) = room_request(
            &alice,
            Method::GET,
            format!("{messages_path}?before_message_id={page_cursor}&limit=10"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second_page["messages"].as_array().unwrap().len(), 1);
        assert_eq!(second_page["has_more"], false);

        let thread_path = format!(
            "/companies/{}/rooms/{room}/threads/{first_id}",
            fixture.company
        );
        let (status, thread) = room_request(&alice, Method::GET, thread_path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(thread["messages"].as_array().unwrap().len(), 2);
        assert_eq!(thread["messages"][0]["id"], first_id);

        let other_room = fixture
            .org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Other",
                &["bob"],
                "thread-other-room",
            )
            .await
            .unwrap();
        let other_message = fixture
            .org
            .send_room_message(other_room.id, "alice", "Other", None, "other-root")
            .await
            .unwrap();
        let (status, cross_room) = room_request(
            &bob,
            Method::POST,
            format!(
                "/companies/{}/rooms/{room}/messages/{}/replies",
                fixture.company, other_message.message.id
            ),
            Some(serde_json::json!({
                "body": "Wrong Room",
                "command_id": "cross-room-reply"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(cross_room["error"], "room");
    }

    #[tokio::test]
    async fn room_mention_routes_are_scoped_retry_safe_and_require_an_explicit_thread_reply() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room mention route scenario");
            return;
        };
        let room = fixture
            .org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Release decision",
                &["bob"],
                "mention-release-room",
            )
            .await
            .unwrap();
        let sessions = SessionStore::default();
        let alice_token = sessions.establish(
            RoomRouteFixture::identity("alice", "member", &fixture.company),
            Duration::from_secs(60),
        );
        let bob_token = sessions.establish(
            RoomRouteFixture::identity("bob", "member", &fixture.company),
            Duration::from_secs(60),
        );
        let other_company_token = sessions.establish(
            RoomRouteFixture::identity("alice", "member", &fixture.other_company),
            Duration::from_secs(60),
        );
        let alice = fixture.network_app(sessions.resolve_lease(&alice_token).unwrap());
        let bob = fixture.network_app(sessions.resolve_lease(&bob_token).unwrap());
        let other_company_alice =
            fixture.network_app(sessions.resolve_lease(&other_company_token).unwrap());
        let messages_path = format!("/companies/{}/rooms/{}/messages", fixture.company, room.id);
        let command = serde_json::json!({
            "body": "Should we ship?",
            "command_id": "http-mention",
            "mentions": [{
                "actor_id": "bob",
                "why_this_actor": "Bob owns the customer promise",
                "expected_response": "ship or hold",
                "recommendation": "ship",
                "alternatives": ["hold"],
                "evidence": ["probe 42"],
                "affected_scope": "public release"
            }]
        });
        let (status, sent) =
            room_request(&alice, Method::POST, &messages_path, Some(command.clone())).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(sent["message"]["from_actor"], "alice");
        assert_eq!(sent["mentions"].as_array().unwrap().len(), 1);
        assert_eq!(sent["mentions"][0]["mentioned_actor_id"], "bob");
        let message_id = sent["message"]["id"].as_i64().unwrap();
        let mention_id = sent["mentions"][0]["id"].as_str().unwrap();

        let (status, retry) =
            room_request(&alice, Method::POST, &messages_path, Some(command)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(retry["created"], false);
        assert_eq!(retry["mentions"][0]["id"], mention_id);

        let mentions_path = format!("/companies/{}/mentions", fixture.company);
        let (status, _) =
            room_request(&other_company_alice, Method::GET, &mentions_path, None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, bob_attention) = room_request(&bob, Method::GET, &mentions_path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(bob_attention["mentions"].as_array().unwrap().len(), 1);
        assert_eq!(
            bob_attention["mentions"][0]["message"]["body"],
            "Should we ship?"
        );
        assert_eq!(
            bob_attention["mentions"][0]["mention"]["thread_root_message_id"],
            message_id
        );
        let (status, alice_attention) =
            room_request(&alice, Method::GET, &mentions_path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(alice_attention["mentions"].as_array().unwrap().is_empty());

        // An ordinary reply does not implicitly clear a mention.
        let reply_path = format!(
            "/companies/{}/rooms/{}/messages/{message_id}/replies",
            fixture.company, room.id
        );
        let (status, ordinary) = room_request(
            &bob,
            Method::POST,
            &reply_path,
            Some(serde_json::json!({
                "body": "I am checking.",
                "command_id": "ordinary-reply"
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(ordinary["resolved_mention"].is_null());
        let (status, still_pending) = room_request(&bob, Method::GET, &mentions_path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(still_pending["mentions"].as_array().unwrap().len(), 1);

        let (status, forged_resolution) = room_request(
            &alice,
            Method::POST,
            &reply_path,
            Some(serde_json::json!({
                "body": "Forged answer",
                "command_id": "forged-resolution",
                "resolves_mention_id": mention_id
            })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(forged_resolution["error"], "room_access");

        let (status, resolved) = room_request(
            &bob,
            Method::POST,
            &reply_path,
            Some(serde_json::json!({
                "body": "Ship after the final probe.",
                "command_id": "exact-resolution",
                "resolves_mention_id": mention_id
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(resolved["resolved_mention"]["id"], mention_id);
        assert_eq!(
            resolved["resolved_mention"]["resolution_message_id"],
            resolved["message"]["id"]
        );
        let (status, cleared) = room_request(&bob, Method::GET, &mentions_path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(cleared["mentions"].as_array().unwrap().is_empty());

        let (status, page) = room_request(&alice, Method::GET, &messages_path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["mentions"].as_array().unwrap().len(), 1);
        assert_eq!(page["mentions"][0]["id"], mention_id);
        assert!(page["mentions"][0]["resolved_at"].is_string());

        for query in [
            "?after_created_event_id=-1",
            "?limit=0",
            "?limit=101",
            "?unexpected=true",
        ] {
            let response = room_get_response(&bob, format!("{mentions_path}{query}"), None).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
    }

    #[tokio::test]
    async fn member_room_message_may_link_only_work_visible_in_that_exact_room() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping member Work mention route scenario"
            );
            return;
        };
        let shared_room = fixture
            .org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Shared game room",
                &["bob"],
                "member-work-mention-shared-room",
            )
            .await
            .unwrap();
        let other_room = fixture
            .org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Other game room",
                &["bob"],
                "member-work-mention-other-room",
            )
            .await
            .unwrap();
        let add_work = |title: &'static str| restless_orgintel::NewWork {
            owner_id: "exec",
            title,
            outcome: "A playable browser game outcome",
            goal_id: None,
            priority: 1,
            expected_artifact: "A reviewable build",
            workspace: restless_orgintel::WorkspaceSpec::default(),
            attempt_limit: Some(2),
        };
        let company_work = fixture
            .org
            .add_work(add_work("Company game direction"))
            .await
            .unwrap();
        let shared_work = fixture
            .org
            .add_work(add_work("Shared Room playtest"))
            .await
            .unwrap();
        let other_work = fixture
            .org
            .add_work(add_work("Other Room launch plan"))
            .await
            .unwrap();
        for (work_id, room_id) in [(shared_work, shared_room.id), (other_work, other_room.id)] {
            fixture
                .org
                .set_work_collaboration_scope(restless_orgintel::SetWorkCollaborationScope {
                    command_id: Uuid::new_v4(),
                    work_id,
                    actor_id: "alice",
                    expected_revision: 1,
                    visibility: restless_orgintel::WorkCollaborationVisibility::Room,
                    room_id: Some(room_id),
                })
                .await
                .unwrap();
        }

        let sessions = SessionStore::default();
        let alice_token = sessions.establish(
            RoomRouteFixture::identity("alice", "member", &fixture.company),
            Duration::from_secs(60),
        );
        let alice = fixture.network_app(sessions.resolve_lease(&alice_token).unwrap());
        let messages_path = format!(
            "/companies/{}/rooms/{}/messages",
            fixture.company, shared_room.id
        );
        for (work_id, command_id) in [
            (company_work, "member-links-company-work"),
            (shared_work, "member-links-exact-room-work"),
        ] {
            let (status, response) = room_request(
                &alice,
                Method::POST,
                &messages_path,
                Some(serde_json::json!({
                    "body": "Please review this Work.",
                    "command_id": command_id,
                    "mentions": [{
                        "actor_id": "bob",
                        "work_id": work_id,
                        "why_this_actor": "Bob owns the playtest judgement",
                        "expected_response": "Say whether the build is ready"
                    }]
                })),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);
            assert_eq!(response["message"]["from_actor"], "alice");
            assert_eq!(response["mentions"][0]["work_id"], work_id.to_string());
        }

        let (status, denied) = room_request(
            &alice,
            Method::POST,
            &messages_path,
            Some(serde_json::json!({
                "body": "This Work belongs to another Room.",
                "command_id": "member-links-other-room-work",
                "mentions": [{
                    "actor_id": "bob",
                    "work_id": other_work,
                    "why_this_actor": "Bob owns the playtest judgement",
                    "expected_response": "Say whether the build is ready"
                }]
            })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(denied["error"], "room_access");
    }

    #[tokio::test]
    async fn room_event_routes_are_strict_scoped_paged_body_free_and_reconnectable() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room event route scenario");
            return;
        };
        let alice = fixture.app("alice", "member", &fixture.company);
        let other_company_alice = fixture.app("alice", "member", &fixture.other_company);
        let mallory = fixture.app("mallory", "member", &fixture.company);
        let room = fixture
            .org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Replay",
                &["bob"],
                "event-replay-room",
            )
            .await
            .unwrap();
        let baseline = fixture
            .org
            .room_events_after("alice", room.id, 0, 100)
            .await
            .unwrap()
            .snapshot_cursor;
        let first = fixture
            .org
            .send_room_message(room.id, "alice", "First secret body", None, "event-first")
            .await
            .unwrap();
        let second = fixture
            .org
            .send_room_message(room.id, "alice", "Second secret body", None, "event-second")
            .await
            .unwrap();
        let other_room = fixture
            .org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Other replay",
                &["bob"],
                "event-other-room",
            )
            .await
            .unwrap();
        let other = fixture
            .org
            .send_room_message(
                other_room.id,
                "alice",
                "Other secret body",
                None,
                "event-other",
            )
            .await
            .unwrap();
        let third = fixture
            .org
            .send_room_message(room.id, "bob", "Third secret body", None, "event-third")
            .await
            .unwrap();

        let events_path = format!("/companies/{}/rooms/{}/events", fixture.company, room.id);
        for query in [
            "?after_event_id=-1",
            "?limit=0",
            "?limit=101",
            "?limit=1&unexpected=true",
        ] {
            let response = room_get_response(&alice, format!("{events_path}{query}"), None).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        let (status, first_page) = room_request(
            &alice,
            Method::GET,
            format!("{events_path}?after_event_id={baseline}&limit=2"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first_page["events"].as_array().unwrap().len(), 2);
        assert_eq!(first_page["events"][0]["id"], first.event_id);
        assert_eq!(first_page["events"][1]["id"], second.event_id);
        assert_eq!(first_page["has_more"], true);
        assert!(first_page["events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|event| event.get("body").is_none()));
        assert!(!first_page.to_string().contains("secret body"));

        let page_cursor = first_page["next_after_event_id"].as_i64().unwrap();
        let (status, final_page) = room_request(
            &alice,
            Method::GET,
            format!("{events_path}?after_event_id={page_cursor}&limit=100"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let final_ids = final_page["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|event| event["id"].as_i64().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(final_ids, vec![third.event_id]);
        assert!(!final_ids.contains(&other.event_id));
        assert!(final_page["events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|event| event["room_id"] == room.id.to_string()));
        let snapshot_cursor = final_page["snapshot_cursor"].as_i64().unwrap();
        let future_cursor = snapshot_cursor.checked_add(1).unwrap();
        let (status, gap) = room_request(
            &alice,
            Method::GET,
            format!("{events_path}?after_event_id={future_cursor}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(gap["resync_required"], true);
        assert_eq!(gap["snapshot_cursor"], snapshot_cursor);

        let (status, duplicate_page) = room_request(
            &alice,
            Method::GET,
            format!("{events_path}?after_event_id={baseline}&limit=2"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(duplicate_page["events"], first_page["events"]);

        let response = room_get_response(&mallory, &events_path, None).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = room_get_response(
            &alice,
            format!(
                "/companies/{}/rooms/{}/events",
                fixture.other_company, room.id
            ),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = room_get_response(
            &other_company_alice,
            format!(
                "/companies/{}/rooms/{}/events",
                fixture.other_company, room.id
            ),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = room_get_response(
            &alice,
            format!(
                "/companies/{}/rooms/{}/events",
                fixture.company,
                Uuid::new_v4()
            ),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let live_path = format!("{events_path}/live");
        let gap_event = first_sse_chunk(
            room_get_response(
                &alice,
                format!("{live_path}?after_event_id={future_cursor}"),
                None,
            )
            .await,
        )
        .await;
        assert!(gap_event.contains("event: resync"));
        assert!(gap_event.contains(&format!("id: {snapshot_cursor}")));

        let query_event = first_sse_chunk(
            room_get_response(
                &alice,
                format!("{live_path}?after_event_id={baseline}&limit=1"),
                None,
            )
            .await,
        )
        .await;
        assert!(query_event.contains("event: room-event"));
        assert!(query_event.contains(&format!("id: {}", first.event_id)));
        assert!(!query_event.contains("secret body"));

        let header_event = first_sse_chunk(
            room_get_response(
                &alice,
                format!("{live_path}?after_event_id={baseline}&limit=1"),
                Some(&first.event_id.to_string()),
            )
            .await,
        )
        .await;
        assert!(header_event.contains(&format!("id: {}", second.event_id)));

        let response = room_get_response(&alice, &live_path, Some("1x")).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn room_event_stream_resyncs_and_stops_on_participation_session_or_expiry() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room event lifecycle scenario");
            return;
        };
        let alice = fixture.app("alice", "member", &fixture.company);
        let bob = fixture.app("bob", "member", &fixture.company);
        let room = fixture
            .org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Live replay",
                &["bob"],
                "live-replay-room",
            )
            .await
            .unwrap();
        let message = fixture
            .org
            .send_room_message(room.id, "alice", "Compacted body", None, "compact-me")
            .await
            .unwrap();
        fixture
            .org
            .compact_events_through(message.event_id)
            .await
            .unwrap();

        let events_path = format!("/companies/{}/rooms/{}/events", fixture.company, room.id);
        let (status, compacted) = room_request(&alice, Method::GET, &events_path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(compacted["resync_required"], true);
        assert!(compacted["events"].as_array().unwrap().is_empty());
        let snapshot_cursor = compacted["snapshot_cursor"].as_i64().unwrap();

        let response = room_get_response(&alice, format!("{events_path}/live"), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut stream = room_sse_frames(response);
        let resync = tokio::time::timeout(Duration::from_secs(2), stream.next())
            .await
            .expect("resync is immediate")
            .expect("resync event exists")
            .expect("resync body frame");
        let resync = String::from_utf8(resync.to_vec()).unwrap();
        assert!(resync.contains("event: resync"));
        assert!(resync.contains(&format!("id: {snapshot_cursor}")));
        assert!(resync.contains("cursor_unavailable"));
        assert!(!resync.contains("Compacted body"));
        assert!(tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("resync stream closes")
            .is_none());

        let response = room_get_response(
            &fixture.network_app_without_lease("alice", "member", &fixture.company),
            format!("{events_path}/live?after_event_id={snapshot_cursor}"),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let response = room_get_response(
            &bob,
            format!("{events_path}/live?after_event_id={snapshot_cursor}"),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut removed_stream = room_sse_frames(response);
        fixture
            .org
            .remove_room_participant("alice", room.id, "bob")
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(2), removed_stream.next())
                .await
                .expect("participation is rechecked within the poll bound")
                .is_none()
        );
        let live_cursor = fixture
            .org
            .room_events_after("alice", room.id, snapshot_cursor, 100)
            .await
            .unwrap()
            .snapshot_cursor;

        let sessions = SessionStore::default();
        let revoked_token = sessions.establish(
            RoomRouteFixture::identity("alice", "member", &fixture.company),
            Duration::from_secs(60),
        );
        let revoked_lease = sessions.resolve_lease(&revoked_token).unwrap();
        let response = room_get_response(
            &fixture.network_app(revoked_lease),
            format!("{events_path}/live?after_event_id={live_cursor}"),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut revoked_stream = room_sse_frames(response);
        sessions.revoke(&revoked_token);
        assert!(
            tokio::time::timeout(Duration::from_secs(1), revoked_stream.next())
                .await
                .expect("revocation closes the Room stream immediately")
                .is_none()
        );

        let expiry_token = sessions.establish(
            RoomRouteFixture::identity("alice", "member", &fixture.company),
            Duration::from_millis(500),
        );
        let expiry_lease = sessions.resolve_lease(&expiry_token).unwrap();
        let response = room_get_response(
            &fixture.network_app(expiry_lease),
            format!("{events_path}/live?after_event_id={live_cursor}"),
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut expiry_stream = room_sse_frames(response);
        assert!(
            tokio::time::timeout(Duration::from_secs(1), expiry_stream.next())
                .await
                .expect("session expiry closes the Room stream within its TTL")
                .is_none()
        );
    }

    #[tokio::test]
    async fn room_event_stream_uses_body_free_wakes_and_repairs_a_lost_wake() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room wake scenario");
            return;
        };
        let room = fixture
            .org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Wake replay",
                &["bob"],
                "wake-replay-room",
            )
            .await
            .unwrap();
        let baseline = fixture
            .org
            .room_events_after("alice", room.id, 0, 100)
            .await
            .unwrap()
            .snapshot_cursor;
        let live_path = format!(
            "/companies/{}/rooms/{}/events/live?after_event_id={baseline}",
            fixture.company, room.id
        );
        let alice = fixture.app("alice", "member", &fixture.company);
        let response = room_get_response(&alice, &live_path, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut stream = room_sse_frames(response);
        assert!(
            fixture
                .state
                .cell_wakes
                .wait_until_ready(&fixture.company, Duration::from_secs(2))
                .await,
            "the shared cell listener should attach before the fast-path assertion"
        );
        let database_url = fixture
            .state
            .cell_database_url(&fixture.company)
            .await
            .unwrap();
        let mut wake_audit = fixture
            .state
            .cell_wakes
            .subscribe_company(&fixture.company, &database_url);
        let first = fixture
            .org
            .send_room_message(
                room.id,
                "alice",
                "notification must not carry this secret body",
                None,
                "wake-fast-path",
            )
            .await
            .unwrap();
        let wake = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let wake = wake_audit.recv().await.unwrap();
                if wake.wakes_room(&fixture.company, room.id) {
                    break wake;
                }
            }
        })
        .await
        .expect("committed Room event should publish a prompt wake hint");
        assert_eq!(wake.event_id, Some(first.event_id));
        assert!(!wake.raw.contains("secret body"));
        let frame = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("shared wake should avoid waiting for fallback polling")
            .expect("Room event frame")
            .expect("Room event bytes");
        let frame = String::from_utf8(frame.to_vec()).unwrap();
        assert!(frame.contains(&format!("id: {}", first.event_id)));
        assert!(!frame.contains("secret body"));
        drop(stream);

        // A listener that never attaches models a notification lost during an
        // outage. The bounded adaptive fallback must still recover from the
        // same durable cursor without any second event store.
        let mut fallback_state = fixture.state.clone();
        fallback_state.cell_wakes = crate::cell_wake::CellWakeHub::default();
        fallback_state.event_fallback_initial = Duration::from_millis(40);
        fallback_state.event_fallback_max = Duration::from_millis(80);
        if let RoomOrgIntelSource::Fixed { database_url, .. } = &mut fallback_state.source {
            *database_url = "not-a-postgresql-url".into();
        }
        let fallback_app =
            fixture.app_with_state(fallback_state, "alice", "member", &fixture.company);
        let fallback_path = format!(
            "/companies/{}/rooms/{}/events/live?after_event_id={}",
            fixture.company, room.id, first.event_id
        );
        let response = room_get_response(&fallback_app, &fallback_path, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut fallback_stream = room_sse_frames(response);
        let second = fixture
            .org
            .send_room_message(
                room.id,
                "alice",
                "fallback secret body",
                None,
                "wake-lost-fallback",
            )
            .await
            .unwrap();
        let frame = tokio::time::timeout(Duration::from_millis(500), fallback_stream.next())
            .await
            .expect("lost wake should be repaired by the bounded fallback")
            .expect("fallback Room event frame")
            .expect("fallback Room event bytes");
        let frame = String::from_utf8(frame.to_vec()).unwrap();
        assert!(frame.contains(&format!("id: {}", second.event_id)));
        assert!(!frame.contains("fallback secret body"));
    }

    #[tokio::test]
    async fn room_event_stream_caps_idle_fanout_and_refunds_on_every_exit() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room admission scenario");
            return;
        };
        let room = fixture
            .org
            .create_room(
                "alice",
                restless_orgintel::RoomKind::Group,
                "Bounded fanout",
                &["bob", "carol"],
                "bounded-fanout-room",
            )
            .await
            .unwrap();
        let cursor = fixture
            .org
            .room_events_after("alice", room.id, 0, 100)
            .await
            .unwrap()
            .snapshot_cursor;
        let live_path = format!(
            "/companies/{}/rooms/{}/events/live?after_event_id={cursor}",
            fixture.company, room.id
        );
        let mut state = fixture.state.clone();
        state.cell_wakes = crate::cell_wake::CellWakeHub::with_stream_limits(2, 2, 1);
        let hub = state.cell_wakes.clone();
        let alice = fixture.app_with_state(state.clone(), "alice", "member", &fixture.company);
        let bob = fixture.app_with_state(state.clone(), "bob", "member", &fixture.company);
        let carol = fixture.app_with_state(state.clone(), "carol", "member", &fixture.company);

        let alice_idle = room_get_response(&alice, &live_path, None).await;
        assert_eq!(alice_idle.status(), StatusCode::OK);
        assert_eq!(hub.active_streams(), 1);
        let duplicate_alice = room_get_response(&alice, &live_path, None).await;
        assert_eq!(duplicate_alice.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(duplicate_alice.headers()[RETRY_AFTER], "2");

        let bob_idle = room_get_response(&bob, &live_path, None).await;
        assert_eq!(bob_idle.status(), StatusCode::OK);
        assert_eq!(hub.active_streams(), 2);
        for _ in 0..8 {
            let refused = room_get_response(&carol, &live_path, None).await;
            assert_eq!(refused.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(refused.headers()[RETRY_AFTER], "2");
        }
        assert_eq!(hub.active_streams(), 2, "refused clients hold no permit");

        drop(alice_idle);
        assert_eq!(hub.active_streams(), 1);
        let replacement = room_get_response(&alice, &live_path, None).await;
        assert_eq!(replacement.status(), StatusCode::OK);
        assert_eq!(hub.active_streams(), 2);
        drop(replacement);
        drop(bob_idle);
        assert_eq!(hub.active_streams(), 0);

        // Losing Room participation closes the stream on the shared hint and
        // returns admission immediately.
        let response = room_get_response(&bob, &live_path, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut removed_stream = room_sse_frames(response);
        assert!(
            hub.wait_until_ready(&fixture.company, Duration::from_secs(2))
                .await
        );
        fixture
            .org
            .remove_room_participant("alice", room.id, "bob")
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(1), removed_stream.next())
                .await
                .expect("participant-removal wake closes the stream")
                .is_none()
        );
        assert_eq!(hub.active_streams(), 0);

        // A network lease cancellation is selected independently of database
        // wakes and returns the same admission permit.
        let sessions = SessionStore::default();
        let token = sessions.establish(
            RoomRouteFixture::identity("alice", "member", &fixture.company),
            Duration::from_secs(60),
        );
        let lease = sessions.resolve_lease(&token).unwrap();
        let network = fixture.network_app_with_state(state, lease);
        let response = room_get_response(&network, &live_path, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut revoked_stream = room_sse_frames(response);
        assert_eq!(hub.active_streams(), 1);
        sessions.revoke(&token);
        assert!(
            tokio::time::timeout(Duration::from_secs(1), revoked_stream.next())
                .await
                .expect("lease revocation closes the bounded stream")
                .is_none()
        );
        assert_eq!(hub.active_streams(), 0);

        // Deconfiguration is distinct from a transient database disconnect:
        // it closes the company's channel and therefore every remaining
        // stream, rather than leaving a historical tenant polling forever.
        let deconfigured_cursor = fixture
            .org
            .room_events_after("alice", room.id, cursor, 100)
            .await
            .unwrap()
            .snapshot_cursor;
        let deconfigured_path = format!(
            "/companies/{}/rooms/{}/events/live?after_event_id={deconfigured_cursor}",
            fixture.company, room.id
        );
        let response = room_get_response(&alice, &deconfigured_path, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let mut deconfigured_stream = room_sse_frames(response);
        assert_eq!(hub.active_streams(), 1);
        hub.remove_company(&fixture.company);
        assert!(
            tokio::time::timeout(Duration::from_secs(1), deconfigured_stream.next())
                .await
                .expect("deconfiguration closes the company stream")
                .is_none()
        );
        assert_eq!(hub.active_streams(), 0);
    }

    #[tokio::test]
    async fn room_participant_and_read_routes_enforce_room_roles_and_monotonicity() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room role route scenario");
            return;
        };
        let alice = fixture.app("alice", "member", &fixture.company);
        let bob = fixture.app("bob", "member", &fixture.company);
        let admin = fixture.app("carol", "admin", &fixture.company);
        let owner = fixture.app("owner", "owner", &fixture.company);
        let rooms_path = format!("/companies/{}/rooms", fixture.company);

        let (status, denied_company_room) = room_request(
            &alice,
            Method::POST,
            &rooms_path,
            Some(serde_json::json!({
                "kind": "company",
                "title": "Company",
                "command_id": "member-company-room",
                "participant_actor_ids": ["alice", "bob"]
            })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(denied_company_room["error"], "membership_role");
        let (status, denied_admin_room) = room_request(
            &admin,
            Method::POST,
            &rooms_path,
            Some(serde_json::json!({
                "kind": "company",
                "title": "Company",
                "command_id": "admin-company-room",
                "participant_actor_ids": ["alice", "bob", "carol"]
            })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(denied_admin_room["error"], "membership_role");
        let (status, _) = room_request(
            &owner,
            Method::POST,
            &rooms_path,
            Some(serde_json::json!({
                "kind": "company",
                "title": "Company",
                "command_id": "owner-company-room",
                "participant_actor_ids": ["alice", "bob"]
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, group) = room_request(
            &alice,
            Method::POST,
            &rooms_path,
            Some(serde_json::json!({
                "kind": "group",
                "title": "Delivery",
                "command_id": "member-delivery-room",
                "participant_actor_ids": ["bob"]
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let room = room_id(&group);
        let participant_path = format!("/companies/{}/rooms/{room}/participants", fixture.company);
        let add_carol = serde_json::json!({ "actor_id": "carol" });
        let (status, denied) = room_request(
            &bob,
            Method::POST,
            &participant_path,
            Some(add_carol.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(denied["error"], "room_access");
        let (status, added) =
            room_request(&alice, Method::POST, &participant_path, Some(add_carol)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(added["actor_id"], "carol");

        let cursor_path = format!("/companies/{}/rooms/{room}/read-cursor", fixture.company);
        let messages_path = format!("/companies/{}/rooms/{room}/messages", fixture.company);
        let mut message_ids = Vec::new();
        for (command_id, body) in [("read-one", "One"), ("read-two", "Two")] {
            let (status, sent) = room_request(
                &alice,
                Method::POST,
                &messages_path,
                Some(serde_json::json!({
                    "body": body,
                    "command_id": command_id
                })),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);
            message_ids.push(sent["message"]["id"].as_i64().unwrap());
        }
        let (status, newest) = room_request(
            &bob,
            Method::POST,
            &cursor_path,
            Some(serde_json::json!({
                "through_message_id": message_ids[1]
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(newest["last_read_message_id"], message_ids[1]);
        let (status, stale_retry) = room_request(
            &bob,
            Method::POST,
            &cursor_path,
            Some(serde_json::json!({
                "through_message_id": message_ids[0]
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(stale_retry["last_read_message_id"], message_ids[1]);
        let (status, current) = room_request(&bob, Method::GET, &cursor_path, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(current["actor_id"], "bob");
        assert_eq!(current["cursor"]["last_read_message_id"], message_ids[1]);

        let (status, removed) = room_request(
            &alice,
            Method::DELETE,
            format!("{participant_path}/bob"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(removed["actor_id"], "bob");
        assert!(!removed["left_at"].is_null());
        let (status, denied) = room_request(&bob, Method::GET, &messages_path, None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(denied["error"], "room_access");
    }

    #[tokio::test]
    #[ignore = "serves a dedicated *_test company until interrupted for owner-surface visual QA"]
    async fn live_isolated_owner_surface_server() {
        let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL")
            .expect("set RESTLESS_TEST_DATABASE_URL to an isolated test database");
        let company = std::env::var("RESTLESS_OWNER_SURFACE_TEST_COMPANY")
            .expect("set RESTLESS_OWNER_SURFACE_TEST_COMPANY");
        assert!(
            company.ends_with("_test"),
            "visual QA server may expose only a *_test company"
        );
        let root = runtime::state_root();
        let authority = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .unwrap();
        let daemon = Arc::new(crate::Daemon {
            root: root.clone(),
            capabilities: crate::capability::CapabilityIssuer::open(&root).unwrap(),
            spend: crate::spend::SpendLedger::open(&root).unwrap(),
            publication: crate::publication::PublicationManager::new(&root, authority.clone())
                .unwrap(),
            launch: crate::launch::LaunchBroker::new(&root).unwrap(),
            authority,
            orgintel: crate::OrgIntelRegistry {
                database_url,
                root: root.clone(),
                handles: std::sync::Mutex::new(HashMap::new()),
            },
            staff: crate::staff::StaffRegistry::default(),
            activities: crate::activity::AgentActivityStreams::default(),
            cell_wakes: crate::cell_wake::CellWakeHub::default(),
            runtime_bridges: crate::runtime_bridge::RuntimeBridgeRegistry::default(),
            lifecycle: restless_contracts::appliance::LifecycleGate::default(),
            in_flight: Arc::new(std::sync::Mutex::new(crate::schedule::WakeClaims::default())),
            schedule_wake: Arc::new(tokio::sync::Notify::new()),
        });
        // Prove the requested company exists in both the configured Runtime
        // set and the isolated OrgIntel database before publishing a surface.
        runtime::CompanyConfig::load(&root, &company).unwrap();
        assert!(daemon
            .orgintel
            .get(&company)
            .await
            .unwrap()
            .is_live()
            .await
            .unwrap());
        let address = std::env::var("RESTLESS_OWNER_SURFACE_TEST_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7888".into())
            .parse()
            .unwrap();
        let review_address = std::env::var("RESTLESS_OWNER_SURFACE_TEST_REVIEW_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:7894".into())
            .parse()
            .unwrap();
        println!("isolated owner surface for {company}: http://{address}/{company}");
        serve(
            daemon,
            OwnerConfig {
                address,
                review_address,
                review_public_url: format!("http://{{ticket}}.localhost:{}", review_address.port()),
                entry: EntryMode::Local,
                runtime_mode: crate::runtime_mode::RuntimeMode::Local,
            },
        )
        .await
        .unwrap();
    }

    fn network_headers(host: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_str(host).unwrap());
        headers
    }

    fn identity(scope: crate::entry::CompanyScope) -> crate::entry::VerifiedIdentity {
        crate::entry::VerifiedIdentity {
            user: "user-1".into(),
            issuer: Some("https://cloud.restless.test".into()),
            owner: "owner-1".into(),
            scope,
            role: "member".into(),
            actor: None,
            company_id: None,
            cell_id: None,
            membership_id: None,
            membership_version: None,
        }
    }

    fn external_control_context(
        control: &crate::entry::VerifiedMembershipControl,
    ) -> restless_orgintel::ExternalMembershipControlContext<'_> {
        restless_orgintel::ExternalMembershipControlContext {
            issuer: &control.issuer,
            subject: &control.subject,
            assertion_id: control.assertion_id,
            issued_at: control.issued_at,
            expires_at: control.expires_at,
            key_id: &control.key_id,
            assertion_version: control.assertion_version,
            owner_id: control.owner_id,
            plane_id: control.plane_id,
            plane_hostname: &control.plane_hostname,
            company_id: control.company_id,
            cell_id: control.cell_id,
            membership_id: &control.membership_id,
            membership_role: &control.membership_role,
            membership_status: control.membership_status,
            membership_version: control.membership_version,
        }
    }

    const PLANE_HOST: &str = "aris.restless.test";

    #[test]
    fn network_entry_refuses_company_reads_without_a_session() {
        let refusal = network_boundary_violation(
            &Method::GET,
            &network_headers(PLANE_HOST),
            "/api/companies/aris/cockpit",
            PLANE_HOST,
            None,
        )
        .expect("no session is refused");
        assert_eq!(refusal.status, StatusCode::UNAUTHORIZED);
        assert_eq!(refusal.code, "no_session");
    }

    #[test]
    fn network_entry_admits_the_door_without_a_session() {
        assert!(network_boundary_violation(
            &Method::POST,
            &{
                let mut headers = network_headers(PLANE_HOST);
                headers.insert(
                    ORIGIN,
                    HeaderValue::from_str(&format!("https://{PLANE_HOST}")).unwrap(),
                );
                headers
            },
            "/entry",
            PLANE_HOST,
            None,
        )
        .is_none());
    }

    #[test]
    fn public_jwks_routes_are_method_and_path_exact_on_private_service_hosts() {
        const COLLABORATION_SERVICE_HOST: &str = "core-documents-api:7788";

        for path in [
            documents_api::DOCUMENT_COLLABORATION_JWKS_PATH,
            crate::company_projection::JWKS_PATH,
        ] {
            for method in [Method::GET, Method::HEAD] {
                assert!(network_boundary_violation(
                    &method,
                    &network_headers(COLLABORATION_SERVICE_HOST),
                    path,
                    PLANE_HOST,
                    None,
                )
                .is_none());
            }
        }

        for (method, path) in [
            (
                Method::POST,
                documents_api::DOCUMENT_COLLABORATION_JWKS_PATH,
            ),
            (Method::POST, crate::company_projection::JWKS_PATH),
            (Method::DELETE, crate::company_projection::JWKS_PATH),
            (Method::GET, "/.well-known/"),
            (
                Method::GET,
                "/.well-known/restless-native-documents-jwks.json/near-miss",
            ),
            (
                Method::GET,
                "/.well-known/restless-company-projection-jwks.json/near-miss",
            ),
        ] {
            let refusal = network_boundary_violation(
                &method,
                &network_headers(COLLABORATION_SERVICE_HOST),
                path,
                PLANE_HOST,
                None,
            )
            .expect("only an exact GET or HEAD of the public JWKS may bypass the plane Host");
            assert_eq!(refusal.code, "network_owner_boundary");
        }
    }

    #[test]
    fn fleet_cross_site_form_may_reach_the_signed_entry_door() {
        let mut headers = network_headers(PLANE_HOST);
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        headers.insert(
            ORIGIN,
            HeaderValue::from_static("https://cloud.restless.test"),
        );
        assert!(
            network_boundary_violation(&Method::POST, &headers, "/entry", PLANE_HOST, None,)
                .is_none()
        );
    }

    #[test]
    fn cross_site_navigation_opens_only_the_public_shell() {
        let mut headers = network_headers(PLANE_HOST);
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
        headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));
        headers.insert(
            ORIGIN,
            HeaderValue::from_static("https://accounts.example.test"),
        );
        for path in ["/", "/aris", "/aris/work/documents"] {
            assert!(
                network_boundary_violation(&Method::GET, &headers, path, PLANE_HOST, None)
                    .is_none()
            );
            assert!(
                network_boundary_violation(&Method::POST, &headers, path, PLANE_HOST, None)
                    .is_some()
            );
        }
        for path in [
            "/api",
            "/api/companies",
            "/api/companies/aris/principal",
            "/desktop",
            "/desktop/aris",
        ] {
            assert!(
                network_boundary_violation(&Method::GET, &headers, path, PLANE_HOST, None)
                    .is_some()
            );
        }
        headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
        assert!(
            network_boundary_violation(&Method::GET, &headers, "/aris", PLANE_HOST, None).is_none()
        );
        assert!(network_boundary_violation(
            &Method::GET,
            &headers,
            "/api/companies",
            PLANE_HOST,
            None
        )
        .is_some());
        headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
        assert!(
            network_boundary_violation(&Method::GET, &headers, "/aris", PLANE_HOST, None).is_some()
        );
        headers.insert("sec-fetch-mode", HeaderValue::from_static("navigate"));
        headers.insert("sec-fetch-dest", HeaderValue::from_static("iframe"));
        assert!(
            network_boundary_violation(&Method::GET, &headers, "/aris", PLANE_HOST, None).is_some()
        );
        headers.insert("sec-fetch-dest", HeaderValue::from_static("document"));
        headers.insert(HOST, HeaderValue::from_static("another.example.test"));
        assert!(
            network_boundary_violation(&Method::GET, &headers, "/aris", PLANE_HOST, None).is_some()
        );
    }

    #[test]
    fn fleet_control_reaches_only_the_exact_plane_host_without_browser_context() {
        assert!(network_boundary_violation(
            &Method::POST,
            &network_headers(PLANE_HOST),
            MEMBERSHIP_CONTROL_PATH,
            PLANE_HOST,
            None,
        )
        .is_none());

        let mut cross_site = network_headers(PLANE_HOST);
        cross_site.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        cross_site.insert(
            ORIGIN,
            HeaderValue::from_static("https://cloud.restless.test"),
        );
        assert!(network_boundary_violation(
            &Method::POST,
            &cross_site,
            MEMBERSHIP_CONTROL_PATH,
            PLANE_HOST,
            None,
        )
        .is_none());

        for malformed in [
            "aris.restless.test.evil",
            "aris.restless.test:evil",
            "attacker@aris.restless.test",
        ] {
            let refusal = network_boundary_violation(
                &Method::POST,
                &network_headers(malformed),
                MEMBERSHIP_CONTROL_PATH,
                PLANE_HOST,
                None,
            )
            .unwrap_or_else(|| panic!("non-exact Host {malformed:?} must be refused"));
            assert_eq!(refusal.code, "network_owner_boundary");
        }
        assert!(network_host_matches(
            &network_headers("aris.restless.test:443"),
            PLANE_HOST
        ));

        let mut duplicate = network_headers(PLANE_HOST);
        duplicate.append(HOST, HeaderValue::from_static("aris.restless.test"));
        assert!(!network_host_matches(&duplicate, PLANE_HOST));
    }

    #[test]
    fn entry_accepts_form_posts_and_json_without_query_credentials() {
        let mut form_headers = HeaderMap::new();
        form_headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        let (form, redirect) =
            parse_entry_request(&form_headers, b"assertion=header.payload.signature").unwrap();
        assert_eq!(form.assertion, "header.payload.signature");
        assert!(redirect);

        let mut json_headers = HeaderMap::new();
        json_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let (json, redirect) = parse_entry_request(
            &json_headers,
            br#"{"assertion":"header.payload.signature"}"#,
        )
        .unwrap();
        assert_eq!(json.assertion, "header.payload.signature");
        assert!(!redirect);

        assert!(parse_entry_request(&form_headers, b"assertion=one&assertion=two").is_err());
    }

    #[test]
    fn membership_control_request_is_closed_json_and_bounded() {
        let mut json_headers = HeaderMap::new();
        json_headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        let request = parse_membership_control_request(
            &json_headers,
            br#"{"control":"header.payload.signature"}"#,
        )
        .expect("canonical control request");
        assert_eq!(request.control, "header.payload.signature");

        assert!(parse_membership_control_request(
            &json_headers,
            br#"{"control":"x","unexpected":true}"#,
        )
        .is_err());
        assert!(parse_membership_control_request(&json_headers, br#"{"control":""}"#).is_err());
        assert!(
            parse_membership_control_request(&HeaderMap::new(), br#"{"control":"x"}"#).is_err()
        );
        let oversized = vec![b'x'; 32 * 1024 + 1];
        assert!(parse_membership_control_request(&json_headers, &oversized).is_err());
    }

    #[test]
    fn superseded_terminal_control_does_not_evict_a_newer_active_session() {
        let sessions = SessionStore::default();
        let company_id = Uuid::new_v4();
        let cell_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let plane_id = Uuid::new_v4();
        let session = |version: i64| VerifiedIdentity {
            user: "user-1".into(),
            issuer: Some("https://cloud.restless.test".into()),
            owner: owner_id.to_string(),
            scope: CompanyScope::Company {
                company: "aris".into(),
            },
            role: "member".into(),
            actor: Some("human-1".into()),
            company_id: Some(company_id),
            cell_id: Some(cell_id),
            membership_id: Some("membership-1".into()),
            membership_version: Some(version),
        };
        let stale_v4 = sessions.establish(session(4), Duration::from_secs(60));
        let stale_v5 = sessions.establish(session(5), Duration::from_secs(60));
        let current_v6 = sessions.establish(session(6), Duration::from_secs(60));
        let now = Utc::now();
        let control = crate::entry::VerifiedMembershipControl {
            issuer: "https://cloud.restless.test".into(),
            subject: "user-1".into(),
            assertion_id: Uuid::new_v4(),
            issued_at: now,
            expires_at: now + ChronoDuration::seconds(45),
            key_id: "key-1".into(),
            assertion_version: 1,
            owner_id,
            plane_id,
            plane_hostname: PLANE_HOST.into(),
            company_id,
            cell_id,
            membership_id: "membership-1".into(),
            membership_role: "member".into(),
            membership_status: restless_orgintel::ExternalMembershipStatus::Suspended,
            membership_version: 5,
        };
        let receipt = restless_orgintel::ExternalMembershipControlReceipt {
            contract_version: 1,
            jti: control.assertion_id,
            owner_id,
            plane_id,
            plane_hostname: PLANE_HOST.into(),
            company_id,
            cell_id,
            principal_id: "user-1".into(),
            membership_id: "membership-1".into(),
            membership_role: "member".into(),
            requested_status: restless_orgintel::ExternalMembershipStatus::Suspended,
            requested_version: 5,
            outcome: restless_orgintel::MembershipControlOutcome::Superseded,
            observed_status: restless_orgintel::ExternalMembershipStatus::Active,
            observed_version: 6,
            observed_at: now,
        };

        assert_eq!(
            revoke_sessions_for_membership_receipt(&sessions, &control, &receipt),
            2
        );
        assert!(sessions.resolve_lease(&stale_v4).is_none());
        assert!(sessions.resolve_lease(&stale_v5).is_none());
        assert!(sessions.resolve_lease(&current_v6).is_some());
    }

    #[test]
    fn active_handoff_evicts_stale_security_tuples_but_keeps_exact_current_sessions() {
        let sessions = SessionStore::default();
        let company_id = Uuid::new_v4();
        let cell_id = Uuid::new_v4();
        let active = |membership: &str, role: &str, version: i64| VerifiedIdentity {
            user: "user-1".into(),
            issuer: Some("https://cloud.restless.test/".into()),
            owner: "owner-1".into(),
            scope: CompanyScope::Company {
                company: "aris".into(),
            },
            role: role.into(),
            actor: Some("human-1".into()),
            company_id: Some(company_id),
            cell_id: Some(cell_id),
            membership_id: Some(membership.into()),
            membership_version: Some(version),
        };
        let stale_version =
            sessions.establish(active("membership-1", "member", 5), Duration::from_secs(60));
        let replaced_membership = sessions.establish(
            active("membership-old", "member", 6),
            Duration::from_secs(60),
        );
        let stale_role =
            sessions.establish(active("membership-1", "admin", 6), Duration::from_secs(60));
        let exact_current =
            sessions.establish(active("membership-1", "member", 6), Duration::from_secs(60));
        let other_principal = sessions.establish(
            VerifiedIdentity {
                user: "user-2".into(),
                ..active("membership-1", "member", 5)
            },
            Duration::from_secs(60),
        );
        let current = VerifiedIdentity {
            issuer: Some("https://cloud.restless.test".into()),
            ..active("membership-1", "member", 6)
        };

        assert_eq!(sessions.revoke_principal_except_current(&current), 3);
        assert!(sessions.resolve_lease(&stale_version).is_none());
        assert!(sessions.resolve_lease(&replaced_membership).is_none());
        assert!(sessions.resolve_lease(&stale_role).is_none());
        assert!(sessions.resolve_lease(&exact_current).is_some());
        assert!(sessions.resolve_lease(&other_principal).is_some());
    }

    #[tokio::test]
    async fn reconciliation_guard_orders_handoff_and_terminal_session_effects_both_ways() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping session reconciliation race");
            return;
        };
        let company = format!("entry_control_race_{}", Uuid::new_v4().simple());
        let org = restless_orgintel::OrgIntel::ensure(&database_url, &company)
            .await
            .expect("ensure session reconciliation company");
        let sessions = Arc::new(SessionStore::default());
        let issuer = "https://cloud.restless.test";
        let subject = "user-1";
        let owner_id = Uuid::new_v4();
        let plane_id = Uuid::new_v4();
        let company_id = Uuid::new_v4();
        let cell_id = Uuid::new_v4();
        let now = Utc::now();
        org.ensure_company_access_identity(restless_orgintel::CompanyAccessIdentity {
            company_id,
            cell_id,
        })
        .await
        .expect("bind company through hosted bootstrap primitive");

        let initial = org
            .consume_human_access_context(restless_orgintel::HumanAccessContext {
                display_name: None,
                issuer,
                subject,
                company_id,
                cell_id,
                membership_id: "membership-1",
                membership_role: "member",
                membership_version: 1,
                assertion_id: Uuid::new_v4(),
                issued_at: now,
                expires_at: now + ChronoDuration::seconds(60),
            })
            .await
            .expect("initial active handoff");
        let initial_session = sessions.establish(
            VerifiedIdentity {
                user: subject.into(),
                issuer: Some(issuer.into()),
                owner: owner_id.to_string(),
                scope: CompanyScope::Company {
                    company: company.clone(),
                },
                role: initial.membership_role.clone(),
                actor: Some(initial.actor_id.clone()),
                company_id: Some(company_id),
                cell_id: Some(cell_id),
                membership_id: Some(initial.membership_id.clone()),
                membership_version: Some(initial.membership_version),
            },
            Duration::from_secs(60),
        );

        // Terminal first: hold the real per-principal guard, overlap an old
        // handoff behind it, then prove no stale cookie can be established.
        let terminal_v2 = crate::entry::VerifiedMembershipControl {
            issuer: issuer.into(),
            subject: subject.into(),
            assertion_id: Uuid::new_v4(),
            issued_at: now + ChronoDuration::seconds(1),
            expires_at: now + ChronoDuration::seconds(46),
            key_id: "key-1".into(),
            assertion_version: 1,
            owner_id,
            plane_id,
            plane_hostname: PLANE_HOST.into(),
            company_id,
            cell_id,
            membership_id: "membership-1".into(),
            membership_role: "member".into(),
            membership_status: restless_orgintel::ExternalMembershipStatus::Suspended,
            membership_version: 2,
        };
        let terminal_guard = sessions.reconciliation_guard(issuer, subject, company_id);
        let terminal_org = org.clone();
        let terminal_sessions = sessions.clone();
        let (terminal_acquired_tx, terminal_acquired_rx) = tokio::sync::oneshot::channel();
        let (release_terminal_tx, release_terminal_rx) = tokio::sync::oneshot::channel();
        let terminal_task = tokio::spawn(async move {
            let _held = terminal_guard.lock().await;
            terminal_acquired_tx.send(()).unwrap();
            release_terminal_rx.await.unwrap();
            let receipt = terminal_org
                .apply_external_membership_control(external_control_context(&terminal_v2))
                .await
                .expect("terminal control applies");
            revoke_sessions_for_membership_receipt(&terminal_sessions, &terminal_v2, &receipt);
            receipt
        });
        terminal_acquired_rx.await.unwrap();

        let stale_guard = sessions.reconciliation_guard(issuer, subject, company_id);
        let stale_org = org.clone();
        let stale_sessions = sessions.clone();
        let stale_company = company.clone();
        let stale_task = tokio::spawn(async move {
            let _held = stale_guard.lock().await;
            let binding = match stale_org
                .consume_human_access_context(restless_orgintel::HumanAccessContext {
                    display_name: None,
                    issuer,
                    subject,
                    company_id,
                    cell_id,
                    membership_id: "membership-1",
                    membership_role: "member",
                    membership_version: 1,
                    assertion_id: Uuid::new_v4(),
                    issued_at: now + ChronoDuration::seconds(1),
                    expires_at: now + ChronoDuration::seconds(60),
                })
                .await
            {
                Ok(binding) => binding,
                Err(restless_orgintel::OrgIntelError::CompanyAccessMismatch(_)) => return None,
                Err(error) => panic!("unexpected stale-handoff error: {error}"),
            };
            let identity = VerifiedIdentity {
                user: subject.into(),
                issuer: Some(issuer.into()),
                owner: owner_id.to_string(),
                scope: CompanyScope::Company {
                    company: stale_company,
                },
                role: binding.membership_role,
                actor: Some(binding.actor_id),
                company_id: Some(company_id),
                cell_id: Some(cell_id),
                membership_id: Some(binding.membership_id),
                membership_version: Some(binding.membership_version),
            };
            reconcile_active_entry_session(
                &stale_sessions,
                &stale_org,
                &identity,
                Duration::from_secs(60),
            )
            .await
            .expect("reconcile stale handoff")
            .map(|session| session.token)
        });
        tokio::task::yield_now().await;
        release_terminal_tx.send(()).unwrap();
        let terminal_receipt = terminal_task.await.unwrap();
        let stale_token = stale_task.await.unwrap();
        assert_eq!(
            terminal_receipt.observed_status,
            restless_orgintel::ExternalMembershipStatus::Suspended
        );
        assert!(stale_token.is_none());
        assert!(sessions.resolve_lease(&initial_session).is_none());

        // Active first: a newer active handoff establishes its session while
        // the older terminal delivery waits. The superseded control must keep
        // that exact-current session alive.
        let active_guard = sessions.reconciliation_guard(issuer, subject, company_id);
        let active_org = org.clone();
        let active_sessions = sessions.clone();
        let active_company = company.clone();
        let (active_acquired_tx, active_acquired_rx) = tokio::sync::oneshot::channel();
        let (release_active_tx, release_active_rx) = tokio::sync::oneshot::channel();
        let active_task = tokio::spawn(async move {
            let _held = active_guard.lock().await;
            active_acquired_tx.send(()).unwrap();
            release_active_rx.await.unwrap();
            let binding = active_org
                .consume_human_access_context(restless_orgintel::HumanAccessContext {
                    display_name: None,
                    issuer,
                    subject,
                    company_id,
                    cell_id,
                    membership_id: "membership-1",
                    membership_role: "admin",
                    membership_version: 3,
                    assertion_id: Uuid::new_v4(),
                    issued_at: now + ChronoDuration::seconds(2),
                    expires_at: now + ChronoDuration::seconds(60),
                })
                .await
                .expect("newer active handoff applies");
            let identity = VerifiedIdentity {
                user: subject.into(),
                issuer: Some(issuer.into()),
                owner: owner_id.to_string(),
                scope: CompanyScope::Company {
                    company: active_company,
                },
                role: binding.membership_role,
                actor: Some(binding.actor_id),
                company_id: Some(company_id),
                cell_id: Some(cell_id),
                membership_id: Some(binding.membership_id),
                membership_version: Some(binding.membership_version),
            };
            reconcile_active_entry_session(
                &active_sessions,
                &active_org,
                &identity,
                Duration::from_secs(60),
            )
            .await
            .expect("reconcile current handoff")
            .expect("current handoff establishes a session")
            .token
        });
        active_acquired_rx.await.unwrap();

        let late_terminal = crate::entry::VerifiedMembershipControl {
            issuer: issuer.into(),
            subject: subject.into(),
            assertion_id: Uuid::new_v4(),
            issued_at: now + ChronoDuration::seconds(2),
            expires_at: now + ChronoDuration::seconds(47),
            key_id: "key-2".into(),
            assertion_version: 1,
            owner_id,
            plane_id,
            plane_hostname: PLANE_HOST.into(),
            company_id,
            cell_id,
            membership_id: "membership-1".into(),
            membership_role: "member".into(),
            membership_status: restless_orgintel::ExternalMembershipStatus::Suspended,
            membership_version: 2,
        };
        let late_guard = sessions.reconciliation_guard(issuer, subject, company_id);
        let late_org = org.clone();
        let late_sessions = sessions.clone();
        let late_terminal_task = tokio::spawn(async move {
            let _held = late_guard.lock().await;
            let receipt = late_org
                .apply_external_membership_control(external_control_context(&late_terminal))
                .await
                .expect("late terminal delivery gets a receipt");
            revoke_sessions_for_membership_receipt(&late_sessions, &late_terminal, &receipt);
            receipt
        });
        tokio::task::yield_now().await;
        release_active_tx.send(()).unwrap();
        let active_token = active_task.await.unwrap();
        let late_receipt = late_terminal_task.await.unwrap();
        assert_eq!(
            late_receipt.outcome,
            restless_orgintel::MembershipControlOutcome::Superseded
        );
        assert_eq!(
            late_receipt.observed_status,
            restless_orgintel::ExternalMembershipStatus::Active
        );
        assert_eq!(late_receipt.observed_version, 3);
        assert!(sessions.resolve_lease(&active_token).is_some());
    }

    #[tokio::test]
    async fn resolved_membership_lease_closes_on_a_concurrent_revoke() {
        let activities = crate::activity::AgentActivityStreams::default();
        let receiver = activities.subscribe("aris", "exec", Some(1), None);
        let sessions = SessionStore::default();
        let token = sessions.establish(
            identity(CompanyScope::Company {
                company: "aris".into(),
            }),
            Duration::from_secs(60),
        );
        let lease = sessions.resolve_lease(&token).expect("resolved lease");
        let stream = agent_activity_stream(receiver, Some(lease));
        futures_util::pin_mut!(stream);
        assert!(
            stream.next().await.is_some(),
            "initial projection is delivered"
        );

        // Model the control commit landing after boundary middleware cloned
        // the lease but before (or while) the long-lived handler runs.
        sessions.revoke(&token);
        let ended = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("cancelled stream ends promptly");
        assert!(ended.is_none());
    }

    #[tokio::test]
    async fn membership_lease_expiry_closes_an_open_activity_stream() {
        let activities = crate::activity::AgentActivityStreams::default();
        let receiver = activities.subscribe("aris", "exec", Some(1), None);
        let sessions = SessionStore::default();
        let token = sessions.establish(
            identity(CompanyScope::Company {
                company: "aris".into(),
            }),
            Duration::from_millis(100),
        );
        let lease = sessions.resolve_lease(&token).expect("resolved lease");
        let stream = agent_activity_stream(receiver, Some(lease));
        futures_util::pin_mut!(stream);
        assert!(
            stream.next().await.is_some(),
            "initial projection is delivered"
        );

        let ended = tokio::time::timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("expired stream ends promptly");
        assert!(ended.is_none());
    }

    #[tokio::test]
    async fn desktop_session_guard_observes_revocation_and_expiry() {
        let sessions = SessionStore::default();
        let revoked_token = sessions.establish(
            identity(CompanyScope::Company {
                company: "aris".into(),
            }),
            Duration::from_secs(60),
        );
        let revoked_lease = sessions
            .resolve_lease(&revoked_token)
            .expect("resolved desktop lease");
        sessions.revoke(&revoked_token);
        tokio::time::timeout(
            Duration::from_secs(1),
            optional_session_ended(Some(&revoked_lease)),
        )
        .await
        .expect("desktop guard observes revocation");

        let expiring_token = sessions.establish(
            identity(CompanyScope::Company {
                company: "aris".into(),
            }),
            Duration::from_millis(100),
        );
        let expiring_lease = sessions
            .resolve_lease(&expiring_token)
            .expect("resolved expiring desktop lease");
        tokio::time::timeout(
            Duration::from_secs(1),
            optional_session_ended(Some(&expiring_lease)),
        )
        .await
        .expect("desktop guard observes expiry");
    }

    #[test]
    fn non_owner_members_may_collaborate_but_not_call_owner_mutations() {
        let identity = crate::entry::VerifiedIdentity {
            user: "user-1".into(),
            issuer: Some("https://cloud.restless.test".into()),
            owner: "owner-1".into(),
            scope: crate::entry::CompanyScope::Company {
                company: "aris".into(),
            },
            role: "member".into(),
            actor: Some("human-1".into()),
            company_id: Some(Uuid::new_v4()),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some("membership-1".into()),
            membership_version: Some(1),
        };
        let principal = RequestPrincipal::from_verified(&identity).unwrap();
        for role in ["member", "admin"] {
            let mut identity = identity.clone(); identity.role = role.into();
            let participant = RequestPrincipal::from_verified(&identity).unwrap();
            for (method, path) in [
                (Method::GET, "/api/companies/aris/sheets"),
                (Method::POST, "/api/companies/aris/sheets/sheet-id/operations"),
                (Method::GET, "/api/companies/aris/sheets/sheet-id/collaboration"),
                (Method::GET, "/api/companies/aris/sheets/sheet-id/versions"),
            ] { assert!(membership_boundary_violation(&method,path,&participant).is_none()); }
            assert!(membership_boundary_violation(&Method::POST,"/api/companies/aris/up",&participant).is_some());
        }
        assert!(
            membership_boundary_violation(
                &Method::GET,
                "/api/companies/aris/actors/exec/exchanges",
                &principal,
            )
            .is_some(),
            "internal agent exchanges are an owner-only surface"
        );
        assert!(membership_boundary_violation(
            &Method::POST,
            "/api/companies/aris/actors/exec/conversation",
            &principal,
        )
        .is_none());
        assert!(membership_boundary_violation(
            &Method::GET,
            "/api/companies/aris/rooms/room-id/messages",
            &principal,
        )
        .is_none());
        assert!(membership_boundary_violation(
            &Method::GET,
            "/api/companies/aris/principal",
            &principal,
        )
        .is_none());
        let attachment_path = format!("/api/companies/aris/attachments/{}", Uuid::new_v4());
        assert!(
            membership_boundary_violation(&Method::GET, &attachment_path, &principal).is_none()
        );
        assert!(
            membership_boundary_violation(&Method::HEAD, &attachment_path, &principal).is_none()
        );
        assert!(
            membership_boundary_violation(&Method::POST, &attachment_path, &principal).is_some()
        );
        assert!(
            membership_boundary_violation(&Method::DELETE, &attachment_path, &principal).is_some()
        );
        assert!(membership_boundary_violation(
            &Method::GET,
            "/api/companies/aris/attachments/not-a-uuid",
            &principal,
        )
        .is_some());
        for protected_read in [
            "/api/companies/aris/custom-harnesses",
            "/api",
            "/api/companies/aris/cockpit",
            "/desktop",
            "/desktop/aris",
            "/api/companies/aris/actors/exec/activity",
        ] {
            assert_eq!(
                membership_boundary_violation(&Method::GET, protected_read, &principal)
                    .expect("a member must not inherit owner-only reads")
                    .code,
                "membership_role"
            );
            assert_eq!(
                membership_boundary_violation(&Method::HEAD, protected_read, &principal)
                    .expect("HEAD must not bypass the owner-only read boundary")
                    .code,
                "membership_role"
            );
        }
        assert!(
            membership_boundary_violation(&Method::GET, "/assets/app.js", &principal).is_none()
        );
        assert_eq!(
            membership_boundary_violation(
                &Method::POST,
                "/api/companies/aris/actors/exec/conversation/42/interrupt",
                &principal,
            )
            .expect("a member must not cancel another principal's owner directive")
            .code,
            "membership_role"
        );
        for unrelated in [
            "/api/companies/aris/not-rooms/admin",
            "/api/companies/aris/custom-harnesses/example",
            "/api/companies/aris/rooms-admin",
            "/api/companies/aris/principal/admin",
            "/api/companies/aris/reports/conversation",
            "/api/companies/aris/documents-admin",
        ] {
            assert_eq!(
                membership_boundary_violation(&Method::POST, unrelated, &principal)
                    .expect("a substring must not grant collaboration write access")
                    .code,
                "membership_role"
            );
        }
        assert_eq!(
            membership_boundary_violation(
                &Method::POST,
                "/api/companies/aris/approvals/grant",
                &principal,
            )
            .unwrap()
            .code,
            "membership_role"
        );

        let mut work_scoped = OwnerMessageInput {
            work_id: Some(Uuid::new_v4()),
            ..OwnerMessageInput::default()
        };
        assert_eq!(
            owner_message_membership_violation("member", &work_scoped)
                .expect("an unshared Work reference is owner-only")
                .code,
            "work_scope"
        );
        assert!(owner_message_membership_violation("owner", &work_scoped).is_none());
        assert_eq!(
            owner_message_membership_violation("admin", &work_scoped)
                .expect("admin is not the Work-sharing authority")
                .code,
            "work_scope"
        );
        work_scoped.work_id = None;
        work_scoped.interrupt = true;
        assert_eq!(
            owner_message_membership_violation("member", &work_scoped)
                .expect("a member cannot interrupt another cognitive session")
                .code,
            "membership_role"
        );
        assert!(owner_message_membership_violation("admin", &work_scoped).is_none());
    }

    /// S27-T2. The plane genuinely serves both companies, so a pass here proves
    /// scoping rather than the absence of the other company.
    #[test]
    fn a_company_scoped_session_reaches_only_its_own_company() {
        let scoped = identity(crate::entry::CompanyScope::Company {
            company: "aris".into(),
        });

        assert!(
            network_boundary_violation(
                &Method::GET,
                &network_headers(PLANE_HOST),
                "/api/companies/aris/cockpit",
                PLANE_HOST,
                Some(&scoped),
            )
            .is_none(),
            "its own company must remain reachable, or the refusal below proves nothing"
        );

        let refusal = network_boundary_violation(
            &Method::GET,
            &network_headers(PLANE_HOST),
            "/api/companies/other/cockpit",
            PLANE_HOST,
            Some(&scoped),
        )
        .expect("another company on the same plane is refused");
        assert_eq!(refusal.status, StatusCode::FORBIDDEN);
        assert_eq!(refusal.code, "company_out_of_scope");

        // The desktop stream is the same boundary, not a second one.
        let refusal = network_boundary_violation(
            &Method::GET,
            &network_headers(PLANE_HOST),
            "/desktop/other/observe",
            PLANE_HOST,
            Some(&scoped),
        )
        .expect("the desktop path is scoped too");
        assert_eq!(refusal.code, "company_out_of_scope");
    }

    #[test]
    fn an_owner_scoped_session_reaches_every_company_on_its_plane() {
        let owner = identity(crate::entry::CompanyScope::Owner);
        for path in ["/api/companies/aris/cockpit", "/desktop/other/observe"] {
            assert!(network_boundary_violation(
                &Method::GET,
                &network_headers(PLANE_HOST),
                path,
                PLANE_HOST,
                Some(&owner),
            )
            .is_none());
        }
    }

    #[test]
    fn network_entry_refuses_a_host_that_is_not_this_plane() {
        let refusal = network_boundary_violation(
            &Method::GET,
            &network_headers("someone-else.restless.test"),
            "/api/companies/aris/cockpit",
            PLANE_HOST,
            Some(&identity(crate::entry::CompanyScope::Owner)),
        )
        .expect("wrong host refused");
        assert_eq!(refusal.code, "network_owner_boundary");
    }

    /// Scope is re-derived per request, so a session cannot be widened by
    /// arriving at a different hostname or carrying a forwarding claim.
    #[test]
    fn scope_ignores_forwarding_claims_and_the_route_it_arrived_on() {
        let scoped = identity(crate::entry::CompanyScope::Company {
            company: "aris".into(),
        });
        let mut headers = network_headers(PLANE_HOST);
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("other.restless.test"),
        );
        headers.insert("x-real-ip", HeaderValue::from_static("10.0.0.1"));

        let refusal = network_boundary_violation(
            &Method::GET,
            &headers,
            "/api/companies/other/cockpit",
            PLANE_HOST,
            Some(&scoped),
        )
        .expect("a forwarding header does not widen scope");
        assert_eq!(refusal.code, "company_out_of_scope");
    }

    #[test]
    fn cockpit_typescript_bindings_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web/src/lib/model/generated/cockpit.ts");
        let rendered = render_cockpit_bindings();

        if std::env::var_os("RESTLESS_WRITE_COCKPIT_BINDINGS").is_some() {
            if let Some(directory) = path.parent() {
                std::fs::create_dir_all(directory).expect("create cockpit bindings directory");
            }
            std::fs::write(&path, rendered).expect("write cockpit bindings");
            return;
        }

        let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{}: {error}\nRegenerate with: RESTLESS_WRITE_COCKPIT_BINDINGS=1 cargo test -p restless-owner cockpit_typescript_bindings_match",
                path.display()
            )
        });
        assert_eq!(
            committed, rendered,
            "cockpit TypeScript bindings drifted; regenerate with RESTLESS_WRITE_COCKPIT_BINDINGS=1 cargo test -p restless-owner cockpit_typescript_bindings_match"
        );
    }

    #[test]
    fn conversation_typescript_bindings_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web/src/lib/model/generated/conversation.ts");
        let rendered = render_conversation_bindings();

        if std::env::var_os("RESTLESS_WRITE_CONVERSATION_BINDINGS").is_some() {
            if let Some(directory) = path.parent() {
                std::fs::create_dir_all(directory).expect("create conversation bindings directory");
            }
            std::fs::write(&path, rendered).expect("write conversation bindings");
            return;
        }

        let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{}: {error}\nRegenerate with: RESTLESS_WRITE_CONVERSATION_BINDINGS=1 cargo test -p restless-owner conversation_typescript_bindings_match",
                path.display()
            )
        });
        assert_eq!(
            committed, rendered,
            "conversation TypeScript bindings drifted; regenerate with RESTLESS_WRITE_CONVERSATION_BINDINGS=1 cargo test -p restless-owner conversation_typescript_bindings_match"
        );
    }

    #[test]
    fn conversation_contract_preserves_transcript_and_attachment_wire_names() {
        let at = chrono::DateTime::parse_from_rfc3339("2026-08-28T12:00:00Z")
            .expect("fixture timestamp")
            .with_timezone(&Utc);
        let view = ConversationView {
            actor: ConversationActorView {
                id: "exec".into(),
                display: "Exec".into(),
                kind: "exec".into(),
                role: "Executive".into(),
            },
            focus: Some(ConversationFocusView {
                after_message_id: 41,
                started_at: Some(at),
            }),
            messages: vec![ConversationMessageView {
                id: 42,
                from_actor: "owner".into(),
                to_actor: Some("exec".into()),
                body: "Please verify the launch plan.".into(),
                outcome_standard: Some(restless_orgintel::OutcomeStandard::Exceptional),
                attachments: vec![OwnerAttachment {
                    upload_id: Uuid::nil(),
                    name: "plan.md".into(),
                    media_type: "text/markdown".into(),
                    size_bytes: 42,
                    path: "/var/lib/restless-owner-attachments/plan/content".into(),
                }],
                details: None,
                intent: Some(OwnerIntentReceipt {
                    kind: OwnerIntentKind::Conversation,
                    summary: "Launch-plan check".into(),
                    outcome: Some("The launch plan is ready for review.".into()),
                    next_step: Some("Exec checks the prepared plan.".into()),
                    owner_need: None,
                    owner_replies: Vec::new(),
                }),
                context_path: Some("/demo_test/company".into()),
                created_at: at,
                read_at: None,
            }],
        };

        let value = serde_json::to_value(view).expect("encode conversation contract");
        assert_eq!(value["focus"]["after_message_id"], 41);
        assert_eq!(value["messages"][0]["from_actor"], "owner");
        assert_eq!(
            value["messages"][0]["attachments"][0]["uploadId"],
            Uuid::nil().to_string()
        );
        assert_eq!(value["messages"][0]["intent"]["kind"], "conversation");
        assert_eq!(
            value["messages"][0]["intent"]["outcome"],
            "The launch plan is ready for review."
        );
    }

    fn cockpit_contract_fixture(degraded: bool) -> CockpitView {
        let at = || {
            chrono::DateTime::parse_from_rfc3339("2026-08-24T00:00:00Z")
                .expect("fixture timestamp")
                .with_timezone(&Utc)
        };
        let source_health = BTreeMap::from([
            ("orgintel".into(), "available".into()),
            (
                "authority".into(),
                if degraded {
                    "unavailable: fixture authority outage".into()
                } else {
                    "available".into()
                },
            ),
            ("runtime".into(), "running".into()),
        ]);
        let legal = if degraded {
            CockpitLegal {
                status: "unavailable".into(),
                profile: None,
                detail: Some("fixture authority outage".into()),
            }
        } else {
            CockpitLegal {
                status: "available".into(),
                profile: Some(CockpitLegalProfile {
                    legal_name: "Fixture Robotics Pty Ltd".into(),
                    trading_name: Some("Fixture Robotics".into()),
                    entity_type: "company".into(),
                    jurisdiction: "AU".into(),
                    registration_identifier: CockpitRegistrationIdentifier {
                        kind: "ACN".into(),
                        value: "123456789".into(),
                    },
                    approved_business_address: "1 Test Street".into(),
                    invoice_email: Some("ops@example.test".into()),
                    owner_asserted_by: "owner".into(),
                    owner_asserted_at: at(),
                    registry_observation: None,
                }),
                detail: None,
            }
        };
        let provider = if degraded {
            CockpitProvider {
                status: "unavailable".into(),
                connection: None,
                detail: Some("fixture authority outage".into()),
            }
        } else {
            CockpitProvider {
                status: "available".into(),
                connection: Some(CockpitProviderConnection {
                    environment: "sandbox".into(),
                    account_ref: "acct_fixture".into(),
                    api_version: "2026-01-01".into(),
                    read_scopes: vec!["balances:read".into()],
                    submit_scopes: vec!["transfers:submit".into()],
                    approval_workflow_observed: true,
                    observed_at: Some(at()),
                    updated_at: at(),
                }),
                detail: None,
            }
        };
        let finance = if degraded {
            CockpitFinance {
                status: "unavailable".into(),
                envelopes: Vec::new(),
                payments: Vec::new(),
                last_balance_observation: None,
                detail: Some("fixture authority outage".into()),
            }
        } else {
            CockpitFinance {
                status: "available".into(),
                envelopes: vec![CockpitMoneyEnvelope {
                    source_account_ref: "acct_fixture".into(),
                    currency: "AUD".into(),
                    beneficiary_refs: vec!["beneficiary_fixture".into()],
                    per_payment_limit_minor: 50_000,
                    aggregate_limit_minor: 100_000,
                    frozen: false,
                    period_started_at: at(),
                    updated_by: "owner".into(),
                    updated_at: at(),
                }],
                payments: vec![CockpitPaymentIntent {
                    work_id: Uuid::from_u128(1),
                    owner_handoff_id: Uuid::from_u128(2),
                    source_account_ref: "acct_fixture".into(),
                    provider_beneficiary_ref: "beneficiary_fixture".into(),
                    amount_minor: 12_34,
                    currency: "AUD".into(),
                    purpose: "fixture payment".into(),
                    evidence_refs: vec!["work:fixture".into()],
                    idempotency_key: "fixture-payment-1".into(),
                    requesting_actor: "exec".into(),
                    state: "reserved".into(),
                    provider: "airwallex".into(),
                    provider_transfer_id: None,
                    raw_provider_status: None,
                    provider_approval_url: None,
                    settled_at: None,
                    created_at: at(),
                    updated_at: at(),
                }],
                last_balance_observation: Some(CockpitBalanceObservation {
                    observed_at: at(),
                    body: serde_json::json!({ "currency": "AUD", "available": "10.00" }),
                }),
                detail: None,
            }
        };
        CockpitView {
            company: CockpitCompany {
                id: "fixture_test".into(),
                name: "Fixture Test".into(),
                mission: "Verify the owner projection.".into(),
                model: "fixture/model".into(),
                outcome_standard: restless_orgintel::OutcomeStandard::Exceptional,
            },
            source_health,
            people: vec![CockpitPerson {
                actor_id: "exec".into(),
                kind: "exec".into(),
                role: "exec".into(),
                display: "The Exec".into(),
                model: Some("fixture/model".into()),
                team_id: None,
                spent_usd: 1.25,
                session_running: true,
                session_observed_at: Some(at()),
                model_cooldown: None,
            }],
            teams: vec![CockpitTeam {
                id: Uuid::from_u128(3),
                name: "Research".into(),
                brief: "Research the fixture.".into(),
                outcome_standard: restless_orgintel::OutcomeStandard::Exceptional,
                outcome_standard_source: restless_orgintel::OutcomeStandardSource::CompanyDefault,
                standard_source_message_id: None,
                frontier_phase: "building".into(),
                lead_actor_id: "exec".into(),
                created_by: "owner".into(),
                created_at: at(),
                member_count: 1,
                in_motion_count: 1,
                blocked_count: 0,
            }],
            goals: vec![CockpitGoal {
                id: Uuid::from_u128(4),
                title: "Fixture goal".into(),
                body: "Make the contract observable.".into(),
                created_by: "owner".into(),
                created_at: at(),
                closed_at: None,
                outcome_standard: restless_orgintel::OutcomeStandard::Thorough,
            }],
            spend: CockpitSpend {
                accounted_usd: 1.25,
                ceiling_usd: 25.0,
                remaining_usd: Some(23.75),
                status: "available".into(),
            },
            authority: CockpitAuthority {
                approved_parties: vec!["fixture-provider".into()],
                credentials: vec![CockpitCredential {
                    binding: "fixture.api".into(),
                    status: "present".into(),
                    detail: None,
                }],
                legal,
                provider,
                finance,
            },
            receipts: vec![CockpitEffectReceipt {
                id: 7,
                effect_class: Some(serde_json::json!("provider_read")),
                tool: Some(serde_json::json!("fixture")),
                success: Some(serde_json::json!(true)),
                party: Some(serde_json::json!("fixture-provider")),
                actor: Some(serde_json::json!("exec")),
                outcome: Some(serde_json::json!({ "status": "observed" })),
                evidence_quality: CockpitEvidenceQuality::Governed,
                at: at(),
            }],
            refreshed_at: at(),
        }
    }

    #[tokio::test]
    async fn cockpit_router_keeps_populated_and_degraded_response_shapes() {
        let app = Router::new()
            .route(
                "/populated",
                get(|| async { Json(cockpit_contract_fixture(false)) }),
            )
            .route(
                "/degraded",
                get(|| async { Json(cockpit_contract_fixture(true)) }),
            );

        for (path, degraded) in [("/populated", false), ("/degraded", true)] {
            let response = app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .uri(path)
                        .body(Body::empty())
                        .expect("fixture request"),
                )
                .await
                .expect("fixture router response");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok()),
                Some("application/json")
            );
            let body = to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read fixture response body");
            let json: serde_json::Value =
                serde_json::from_slice(&body).expect("fixture response is JSON");
            assert_eq!(json["company"]["id"], "fixture_test");
            assert_eq!(json["people"][0]["actor_id"], "exec");
            assert_eq!(json["receipts"][0]["evidence_quality"], "governed");
            assert_eq!(
                json["authority"]["finance"]["status"],
                if degraded { "unavailable" } else { "available" }
            );
            if degraded {
                assert_eq!(
                    json["authority"]["legal"]["profile"],
                    serde_json::Value::Null
                );
                assert_eq!(
                    json["authority"]["provider"]["detail"],
                    "fixture authority outage"
                );
            } else {
                assert_eq!(
                    json["authority"]["legal"]["profile"]["legal_name"],
                    "Fixture Robotics Pty Ltd"
                );
                assert!(json["authority"]["provider"].get("detail").is_none());
                assert_eq!(
                    json["authority"]["finance"]["payments"][0]["state"],
                    "reserved"
                );
            }
        }
    }

    #[test]
    fn local_owner_boundary_allows_reads_and_same_origin_writes() {
        let mut read = HeaderMap::new();
        read.insert(HOST, HeaderValue::from_static("localhost:7788"));
        assert_eq!(local_owner_boundary_violation(&Method::GET, &read), None);

        let mut write = read.clone();
        write.insert(ORIGIN, HeaderValue::from_static("http://localhost:7788"));
        write.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        assert_eq!(local_owner_boundary_violation(&Method::POST, &write), None);

        let mut proxied = HeaderMap::new();
        proxied.insert(HOST, HeaderValue::from_static("127.0.0.1:5173"));
        proxied.insert(ORIGIN, HeaderValue::from_static("http://127.0.0.1:5173"));
        assert_eq!(
            local_owner_boundary_violation(&Method::POST, &proxied),
            None
        );
    }

    #[test]
    fn local_owner_boundary_refuses_proxy_cross_site_and_origin_bypass() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("localhost:7788"));

        assert!(local_owner_boundary_violation(&Method::POST, &headers).is_some());

        headers.insert(ORIGIN, HeaderValue::from_static("http://127.0.0.1:7788"));
        assert!(local_owner_boundary_violation(&Method::POST, &headers).is_some());

        headers.insert(ORIGIN, HeaderValue::from_static("http://localhost:7788"));
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        assert!(local_owner_boundary_violation(&Method::POST, &headers).is_some());

        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        headers.insert("x-forwarded-for", HeaderValue::from_static("127.0.0.1"));
        assert!(local_owner_boundary_violation(&Method::GET, &headers).is_some());

        headers.remove("x-forwarded-for");
        headers.insert(HOST, HeaderValue::from_static("example.com:7788"));
        assert!(local_owner_boundary_violation(&Method::GET, &headers).is_some());
    }

    #[test]
    fn owner_message_metadata_round_trips_without_leaking_into_visible_copy() {
        let attachment = OwnerAttachment {
            upload_id: Uuid::nil(),
            name: "brief.pdf".into(),
            media_type: "application/pdf".into(),
            size_bytes: 42,
            path:
                "/var/lib/restless-owner-attachments/00000000-0000-0000-0000-000000000000/content"
                    .into(),
        };
        let with_context = message_with_context("Please read this.", Some("/aris/work"));
        let recorded = message_with_attachments(&with_context, std::slice::from_ref(&attachment));
        let attention_item = attention::AttentionItem {
            id: "orgintel:handoff:00000000-0000-0000-0000-000000000001".into(),
            work_id: Some(Uuid::nil()),
            source: attention::AttentionSource {
                plane: "orgintel",
                kind: "owner_handoff".into(),
                reference: "00000000-0000-0000-0000-000000000001".into(),
                party: None,
                call_key: None,
            },
            category: "decision".into(),
            title: "Choose the launch boundary".into(),
            what_happened: "The lead found two viable paths.".into(),
            why_it_matters: "Either path changes the release boundary.".into(),
            recommendation: "Compare the paths with the owner.".into(),
            requested_action: "Choose a path.".into(),
            if_no_action: "The Work remains blocked.".into(),
            uncertainty: Some("Demand is not yet proven.".into()),
            deadline: None,
            brief_status: "current",
            brief_author: None,
            briefed_at: None,
            evidence: vec![attention::AttentionEvidence {
                label: "Experiment".into(),
                uri: Some("/company/reports/experiment.md".into()),
                content: Some("Path A won on speed; path B won on control.".into()),
                kind: "artifact",
            }],
            review_sources: Vec::new(),
            responsible_actor: Some(attention::AttentionActorRef {
                id: "exec".into(),
                display: "Ari".into(),
                role: "exec".into(),
            }),
            runtime_attach: None,
            review_target: None,
            native_document: None,
            actions: vec![attention::AttentionAction {
                id: "chat-lead".into(),
                label: "Work through this with Ari".into(),
                role: "conversation",
                consequence: "Opens a Work-scoped conversation.".into(),
                next_state: "The decision stays open.".into(),
                href: None,
            }],
            preparing: false,
            can_continue: true,
            created_at: Utc::now(),
        };
        let recorded = message_with_attention_context(&recorded, &attention_item);

        let (without_attention, attention_id) = split_attention_context(&recorded);
        let (without_attachments, attachments) = split_attachment_block(without_attention);
        let (visible, context) = split_context_marker(without_attachments);
        assert_eq!(visible, "Please read this.");
        assert_eq!(context.as_deref(), Some("/aris/work"));
        assert_eq!(attention_id.as_deref(), Some(attention_item.id.as_str()));
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].name, "brief.pdf");
        assert!(recorded.contains(&attachment.path));
        assert!(recorded.contains("Path A won on speed"));
        assert!(!visible.contains("Restless Attention context"));
    }

    #[test]
    fn attachment_download_is_integrity_checked_and_never_inline_active_content() {
        let attachment_id = Uuid::new_v4();
        let bytes = b"<script>fetch('/companies/acme')</script>".to_vec();
        let record = restless_orgintel::OwnerAttachmentRecord {
            attachment_id,
            canonical_name: "proof.html".into(),
            canonical_media_type: "text/html".into(),
            size_bytes: bytes.len() as i64,
            content_sha256: format!("{:x}", Sha256::digest(&bytes)),
            message_id: 42,
        };
        let response = verified_attachment_response(&record, bytes.clone()).unwrap();
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "application/octet-stream"
        );
        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(
            response.headers().get(CONTENT_DISPOSITION).unwrap(),
            "attachment; filename=\"proof.html\""
        );
        let mut mutated = bytes;
        mutated[0] ^= 1;
        assert!(verified_attachment_response(&record, mutated).is_err());

        assert!(ATTACHMENT_STAGE_STALE_AFTER >= ChronoDuration::minutes(5));
        assert!(ATTACHMENT_GC_CLAIM_FOR >= ChronoDuration::minutes(1));
    }

    #[tokio::test]
    async fn attachment_inventory_is_owner_only_bounded_and_retention_truthful() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping attachment inventory scenario");
            return;
        };
        let room = fixture
            .org
            .create_room(
                "owner",
                restless_orgintel::RoomKind::Group,
                "Retention inventory",
                &["exec"],
                "attachment-inventory-room",
            )
            .await
            .unwrap();
        let message = fixture
            .org
            .send_room_message(
                room.id,
                "owner",
                "Retained evidence",
                None,
                "attachment-inventory-message",
            )
            .await
            .unwrap();
        let mut attachment_ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
        let mut raw = sqlx::PgConnection::connect(&database_url).await.unwrap();
        sqlx::query(&format!("SET search_path TO {}", fixture.org.schema()))
            .execute(&mut raw)
            .await
            .unwrap();
        for (index, attachment_id) in attachment_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO owner_attachments ( \
                    attachment_id,sender_actor_id,target_actor_id,client_command_id, \
                    client_payload_sha256,canonical_name,canonical_media_type,size_bytes, \
                    content_sha256,message_id,linked_at,staging_finished_at,created_at \
                 ) VALUES ($1,'owner','exec',$2,$3,$4,'application/octet-stream',$5,$6,$7, \
                           now(),now(),'2026-01-01T00:00:00Z')",
            )
            .bind(attachment_id)
            .bind(format!("inventory-{index}"))
            .bind(format!("{:064x}", index + 1))
            .bind(format!("evidence-{index}.bin"))
            .bind(((index + 1) * 10) as i64)
            .bind(format!("{:064x}", index + 11))
            .bind(message.message.id)
            .execute(&mut raw)
            .await
            .unwrap();
        }
        // One retained item is queued for purge and one is already a durable
        // tombstone whose bytes no longer count against storage.
        sqlx::query(
            "UPDATE owner_attachments SET purge_requested_at=now(), \
                    purge_requested_by='owner',purge_reason='retention ended' \
             WHERE attachment_id=$1",
        )
        .bind(attachment_ids[1])
        .execute(&mut raw)
        .await
        .unwrap();
        sqlx::query(
            "UPDATE owner_attachments SET purge_requested_at=now(), \
                    purge_requested_by='owner',purge_reason='retention ended',purged_at=now() \
             WHERE attachment_id=$1",
        )
        .bind(attachment_ids[2])
        .execute(&mut raw)
        .await
        .unwrap();

        let collection = format!("/companies/{}/attachments", fixture.company);
        let (status, denied) = room_request(
            &fixture.app("exec", "member", &fixture.company),
            Method::GET,
            &collection,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(denied["error"], "membership_role");

        attachment_ids[..2].sort_by(|left, right| right.cmp(left));
        let owner = fixture.app("owner", "owner", &fixture.company);
        let (status, first) =
            room_request(&owner, Method::GET, format!("{collection}?limit=1"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(first["attachments"].as_array().unwrap().len(), 1);
        assert_eq!(
            first["attachments"][0]["attachment_id"],
            attachment_ids[0].to_string()
        );
        assert_eq!(first["has_more"], true);
        assert_eq!(first["usage"]["retained_files"], 2);
        assert_eq!(first["usage"]["retained_bytes"], 30);
        assert_eq!(first["usage"]["purge_pending_files"], 1);
        assert_eq!(first["usage"]["purge_pending_bytes"], 20);
        assert_eq!(
            first["limits"]["retained_files"],
            MAX_RETAINED_ATTACHMENT_FILES
        );
        let before_created_at = first["next_before_created_at"].as_str().unwrap();
        let before_attachment_id = first["next_before_attachment_id"].as_str().unwrap();
        let (status, second) = room_request(
            &owner,
            Method::GET,
            format!(
                "{collection}?limit=1&before_created_at={before_created_at}&before_attachment_id={before_attachment_id}"
            ),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            second["attachments"][0]["attachment_id"],
            attachment_ids[1].to_string()
        );
        assert_eq!(second["has_more"], false);

        let (status, audit) = room_request(
            &owner,
            Method::GET,
            format!("{collection}?limit=100&include_purged=true"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(audit["attachments"].as_array().unwrap().len(), 3);
        assert!(audit["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|attachment| attachment["purged_at"].is_string()));

        for invalid in [
            format!("{collection}?limit=0"),
            format!("{collection}?before_attachment_id={before_attachment_id}"),
            format!(
                "{collection}?before_created_at=not-a-time&before_attachment_id={before_attachment_id}"
            ),
        ] {
            let (status, body) = room_request(&owner, Method::GET, invalid, None).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert!(matches!(
                body["error"].as_str(),
                Some("attachment_limit" | "attachment_cursor")
            ));
        }
    }

    #[tokio::test]
    async fn lost_commit_receipt_recovers_the_exact_committed_attachment_identity() {
        let Some(fixture) = RoomRouteFixture::new().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping attachment receipt scenario");
            return;
        };
        let attachment_id = Uuid::new_v4();
        let attachment = OwnerAttachment {
            upload_id: attachment_id,
            name: "proof.txt".into(),
            media_type: "text/plain".into(),
            size_bytes: 5,
            path: canonical_attachment_path(attachment_id),
        };
        let body =
            message_with_attachments("Evidence attached.", std::slice::from_ref(&attachment));
        let command_id = format!("lost-commit-{}", Uuid::new_v4());
        let digest = "b".repeat(64);
        let content = b"proof";
        fixture
            .org
            .register_owner_attachments(
                "owner",
                "exec",
                &command_id,
                &digest,
                &[restless_orgintel::OwnerAttachmentRegistration {
                    attachment_id,
                    canonical_name: attachment.name.clone(),
                    canonical_media_type: attachment.media_type.clone(),
                    size_bytes: attachment.size_bytes as i64,
                    content_sha256: format!("{:x}", Sha256::digest(content)),
                }],
                MAX_STAGED_ATTACHMENT_FILES as i64,
                MAX_STAGED_ATTACHMENT_BYTES as i64,
                MAX_RETAINED_ATTACHMENT_FILES as i64,
                MAX_RETAINED_ATTACHMENT_BYTES as i64,
                MAX_PRINCIPAL_RETAINED_ATTACHMENT_FILES as i64,
                MAX_PRINCIPAL_RETAINED_ATTACHMENT_BYTES as i64,
            )
            .await
            .unwrap();

        let (message_id, _, created) = fixture
            .org
            .send_human_runtime_conversation_message_idempotent_with_standard(
                "owner",
                "exec",
                &body,
                false,
                None,
                &[attachment_id],
                &command_id,
                &digest,
            )
            .await
            .expect("the simulated command commit succeeds");
        assert!(created);

        // Model an acknowledgement disappearing after COMMIT: recovery knows
        // only the stable command semantics and must rediscover both the one
        // Message receipt and the exact UUID paths it committed.
        let (recovered_id, focus, recovered_body) = fixture
            .org
            .conversation_command_receipt("owner", "exec", &command_id, &digest)
            .await
            .unwrap()
            .expect("durable command receipt");
        assert_eq!(recovered_id, message_id);
        assert!(focus.is_some());
        assert!(receipt_uses_staged_attachments(
            &recovered_body,
            &[attachment]
        ));
        let loser = OwnerAttachment {
            upload_id: Uuid::new_v4(),
            name: "proof.txt".into(),
            media_type: "text/plain".into(),
            size_bytes: 5,
            path: String::new(),
        };
        assert!(!receipt_uses_staged_attachments(&recovered_body, &[loser]));
        let durable = fixture
            .org
            .owner_attachment_for_actor(attachment_id, "owner")
            .await
            .unwrap()
            .expect("linked attachment record remains authoritative");
        assert_eq!(durable.canonical_name, "proof.txt");
        assert_eq!(durable.canonical_media_type, "text/plain");
        assert_eq!(durable.size_bytes, 5);
        assert_eq!(
            durable.content_sha256,
            format!("{:x}", Sha256::digest(content))
        );
        assert_eq!(durable.message_id, message_id);
        assert!(
            fixture
                .org
                .owner_attachment_for_actor(attachment_id, "mallory")
                .await
                .unwrap()
                .is_none(),
            "another active company human cannot read a private Direct-Room attachment"
        );
        let (status, body) = room_request(
            &fixture.app("mallory", "member", &fixture.company),
            Method::GET,
            format!("/companies/{}/attachments/{attachment_id}", fixture.company),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "attachment");
        assert!(fixture
            .org
            .owner_attachment_for_actor(Uuid::new_v4(), "owner")
            .await
            .unwrap()
            .is_none());

        let left_org = fixture.org.clone();
        let right_org = fixture.org.clone();
        let (left, right) = tokio::join!(
            left_org.claim_owner_attachment_gc(
                ATTACHMENT_GC_BATCH as i64,
                3_600,
                ATTACHMENT_GC_CLAIM_FOR.num_seconds(),
            ),
            right_org.claim_owner_attachment_gc(
                ATTACHMENT_GC_BATCH as i64,
                3_600,
                ATTACHMENT_GC_CLAIM_FOR.num_seconds(),
            ),
        );
        let mut claimed = left.unwrap();
        claimed.extend(right.unwrap());
        assert_eq!(claimed.len(), 1, "only one collector may own a stage");
        let (claimed_id, linked, purge_requested, claim_token) = claimed[0];
        assert_eq!(claimed_id, attachment_id);
        assert!(linked);
        assert!(!purge_requested);
        assert!(fixture
            .org
            .complete_owner_attachment_gc(attachment_id, claim_token)
            .await
            .unwrap());
        assert!(
            fixture
                .org
                .owner_attachment_for_actor(attachment_id, "owner")
                .await
                .unwrap()
                .is_some(),
            "settling ingress keeps the durable attachment record"
        );
        let retained_overflow = Uuid::new_v4();
        let error = fixture
            .org
            .register_owner_attachments(
                "owner",
                "exec",
                "retained-overflow",
                &"d".repeat(64),
                &[restless_orgintel::OwnerAttachmentRegistration {
                    attachment_id: retained_overflow,
                    canonical_name: "overflow.txt".into(),
                    canonical_media_type: "text/plain".into(),
                    size_bytes: 1,
                    content_sha256: format!("{:x}", Sha256::digest(b"x")),
                }],
                MAX_STAGED_ATTACHMENT_FILES as i64,
                MAX_STAGED_ATTACHMENT_BYTES as i64,
                1,
                5,
                MAX_PRINCIPAL_RETAINED_ATTACHMENT_FILES as i64,
                MAX_PRINCIPAL_RETAINED_ATTACHMENT_BYTES as i64,
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("retained attachment storage"));
        assert!(fixture
            .org
            .owner_attachment_for_actor(retained_overflow, "owner")
            .await
            .unwrap()
            .is_none());
        let principal_overflow = Uuid::new_v4();
        let error = fixture
            .org
            .register_owner_attachments(
                "owner",
                "exec",
                "principal-retained-overflow",
                &"e".repeat(64),
                &[restless_orgintel::OwnerAttachmentRegistration {
                    attachment_id: principal_overflow,
                    canonical_name: "principal-overflow.txt".into(),
                    canonical_media_type: "text/plain".into(),
                    size_bytes: 1,
                    content_sha256: format!("{:x}", Sha256::digest(b"x")),
                }],
                MAX_STAGED_ATTACHMENT_FILES as i64,
                MAX_STAGED_ATTACHMENT_BYTES as i64,
                MAX_RETAINED_ATTACHMENT_FILES as i64,
                MAX_RETAINED_ATTACHMENT_BYTES as i64,
                1,
                5,
            )
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("principal's retained attachment"));

        // Reverse the race: once GC atomically claims an old unlinked UUID,
        // the Message transaction can no longer consume it. This is the
        // durable fence that a filesystem rename plus timing window lacks.
        let reclaimed_id = Uuid::new_v4();
        let reclaimed_attachment = OwnerAttachment {
            upload_id: reclaimed_id,
            name: "orphan.txt".into(),
            media_type: "text/plain".into(),
            size_bytes: 7,
            path: canonical_attachment_path(reclaimed_id),
        };
        let reclaimed_body =
            message_with_attachments("Stale stage.", std::slice::from_ref(&reclaimed_attachment));
        let reclaimed_command = format!("gc-first-{}", Uuid::new_v4());
        let reclaimed_digest = "c".repeat(64);
        fixture
            .org
            .register_owner_attachments(
                "owner",
                "exec",
                &reclaimed_command,
                &reclaimed_digest,
                &[restless_orgintel::OwnerAttachmentRegistration {
                    attachment_id: reclaimed_id,
                    canonical_name: reclaimed_attachment.name.clone(),
                    canonical_media_type: reclaimed_attachment.media_type.clone(),
                    size_bytes: reclaimed_attachment.size_bytes as i64,
                    content_sha256: format!("{:x}", Sha256::digest(b"orphan!")),
                }],
                MAX_STAGED_ATTACHMENT_FILES as i64,
                MAX_STAGED_ATTACHMENT_BYTES as i64,
                MAX_RETAINED_ATTACHMENT_FILES as i64,
                MAX_RETAINED_ATTACHMENT_BYTES as i64,
                MAX_PRINCIPAL_RETAINED_ATTACHMENT_FILES as i64,
                MAX_PRINCIPAL_RETAINED_ATTACHMENT_BYTES as i64,
            )
            .await
            .unwrap();
        let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
        let mut raw = sqlx::PgConnection::connect(&database_url).await.unwrap();
        sqlx::query(&format!("SET search_path TO {}", fixture.org.schema()))
            .execute(&mut raw)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE owner_attachments SET created_at=now()-interval '2 hours' \
             WHERE attachment_id=$1",
        )
        .bind(reclaimed_id)
        .execute(&mut raw)
        .await
        .unwrap();
        let first_claim = fixture
            .org
            .claim_owner_attachment_gc(
                ATTACHMENT_GC_BATCH as i64,
                3_600,
                ATTACHMENT_GC_CLAIM_FOR.num_seconds(),
            )
            .await
            .unwrap();
        assert_eq!(first_claim.len(), 1);
        let (first_id, first_linked, first_purge_requested, first_token) = first_claim[0];
        assert_eq!(first_id, reclaimed_id);
        assert!(!first_linked);
        assert!(!first_purge_requested);
        assert!(fixture
            .org
            .claim_owner_attachment_gc(
                ATTACHMENT_GC_BATCH as i64,
                3_600,
                ATTACHMENT_GC_CLAIM_FOR.num_seconds(),
            )
            .await
            .unwrap()
            .is_empty());
        assert!(fixture
            .org
            .send_human_runtime_conversation_message_idempotent_with_standard(
                "owner",
                "exec",
                &reclaimed_body,
                false,
                None,
                &[reclaimed_id],
                &reclaimed_command,
                &reclaimed_digest,
            )
            .await
            .unwrap_err()
            .to_string()
            .contains("reclaimed"));
        assert!(fixture
            .org
            .conversation_command_receipt("owner", "exec", &reclaimed_command, &reclaimed_digest,)
            .await
            .unwrap()
            .is_none());
        // A collector crash leaves the token durable. It is not stealable
        // until its bounded lease expires, then a new token fences the old
        // collector's completion.
        sqlx::query(
            "UPDATE owner_attachments SET gc_claimed_at=now()-interval '10 minutes' \
             WHERE attachment_id=$1",
        )
        .bind(reclaimed_id)
        .execute(&mut raw)
        .await
        .unwrap();
        let reclaimed = fixture
            .org
            .claim_owner_attachment_gc(
                ATTACHMENT_GC_BATCH as i64,
                3_600,
                ATTACHMENT_GC_CLAIM_FOR.num_seconds(),
            )
            .await
            .unwrap();
        assert_eq!(reclaimed.len(), 1);
        let (reclaimed_again, linked_again, purge_again, new_token) = reclaimed[0];
        assert_eq!(reclaimed_again, reclaimed_id);
        assert!(!linked_again);
        assert!(!purge_again);
        assert_ne!(new_token, first_token);
        assert!(!fixture
            .org
            .complete_owner_attachment_gc(reclaimed_id, first_token)
            .await
            .unwrap());
        assert!(fixture
            .org
            .complete_owner_attachment_gc(reclaimed_id, new_token)
            .await
            .unwrap());

        assert!(fixture
            .org
            .request_owner_attachment_purge(
                attachment_id,
                "owner",
                "the evidence retention period ended",
            )
            .await
            .unwrap());
        assert!(!fixture
            .org
            .request_owner_attachment_purge(
                attachment_id,
                "owner",
                "the repeated request reuses the durable tombstone",
            )
            .await
            .unwrap());
        assert!(fixture
            .org
            .owner_attachment_for_actor(attachment_id, "owner")
            .await
            .unwrap()
            .is_none());
        let purge_claim = fixture
            .org
            .claim_owner_attachment_gc(
                ATTACHMENT_GC_BATCH as i64,
                3_600,
                ATTACHMENT_GC_CLAIM_FOR.num_seconds(),
            )
            .await
            .unwrap();
        assert_eq!(purge_claim.len(), 1);
        let (purge_id, purge_linked, purge_requested, purge_token) = purge_claim[0];
        assert_eq!(purge_id, attachment_id);
        assert!(purge_linked);
        assert!(purge_requested);
        assert!(fixture
            .org
            .complete_owner_attachment_gc(purge_id, purge_token)
            .await
            .unwrap());
        assert!(fixture
            .org
            .events_of_kind("owner.attachment.purge_requested.v1")
            .await
            .unwrap()
            .iter()
            .any(|event| event.body["attachment_id"] == attachment_id.to_string()));

        let replacement_id = Uuid::new_v4();
        fixture
            .org
            .register_owner_attachments(
                "owner",
                "exec",
                "replacement-after-purge",
                &"f".repeat(64),
                &[restless_orgintel::OwnerAttachmentRegistration {
                    attachment_id: replacement_id,
                    canonical_name: "replacement.txt".into(),
                    canonical_media_type: "text/plain".into(),
                    size_bytes: 1,
                    content_sha256: format!("{:x}", Sha256::digest(b"x")),
                }],
                MAX_STAGED_ATTACHMENT_FILES as i64,
                MAX_STAGED_ATTACHMENT_BYTES as i64,
                1,
                1,
                1,
                1,
            )
            .await
            .expect("a completed governed purge releases retained quota");
    }

    #[test]
    fn cockpit_context_is_scoped_by_url_path_not_raw_query_text() {
        assert_eq!(
            canonical_cockpit_context("aris", "/aris?item=release-integrity"),
            Some("/aris?item=release-integrity".into())
        );
        assert_eq!(
            canonical_cockpit_context("aris", "/aris?next=https://example.com/review"),
            Some("/aris?next=https://example.com/review".into())
        );
        assert_eq!(
            canonical_cockpit_context("aris", "/aris/work/42?lens=board"),
            Some("/aris/work/42?lens=board".into())
        );

        assert_eq!(canonical_cockpit_context("aris", "/cosmon?item=42"), None);
        assert_eq!(canonical_cockpit_context("aris", "/aris-other"), None);
        assert_eq!(canonical_cockpit_context("aris", "/aris/../cosmon"), None);
        assert_eq!(canonical_cockpit_context("aris", "//aris/work"), None);
        assert_eq!(
            canonical_cockpit_context("aris", "https://example.com/aris"),
            None
        );
        assert_eq!(canonical_cockpit_context("aris", "/aris#hidden"), None);
    }

    #[test]
    fn worker_only_intelligence_does_not_hide_exec_setup_requirement() {
        let mut config: runtime::CompanyConfig = toml::from_str(
            r#"name = "worker_only_test"
mission = "Configure Exec separately"
"#,
        )
        .unwrap();
        config.agent_intelligence.insert(
            "writer".into(),
            runtime::AgentIntelligence {
                connection: "direct:openai".into(),
                model: "gpt-5".into(),
            },
        );
        assert!(config.has_configured_model_route());
        assert_eq!(
            company_model_issue(&config).as_deref(),
            Some("Choose an intelligence provider and model in Company → Intelligence provider.")
        );
    }
