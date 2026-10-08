//! Human entry remains the same company/Actor boundary as Rooms and Docs.
use super::*;
use crate::sheet_commands::{self, SheetOperation};
use serde_json::{json, Value};
fn no_store(mut response: Response<Body>) -> Response<Body> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(super) fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    RoomApiState: FromRef<S>,
{
    Router::new()
        .route("/companies/{company}/sheets", get(list).post(create))
        .route("/companies/{company}/sheets/{sheet}", get(read))
        .route("/companies/{company}/sheets-archived", get(list_archived))
        .route("/companies/{company}/sheets/{sheet}/archive", axum::routing::post(archive))
        .route(
            "/companies/{company}/sheets/{sheet}/operations",
            post(operation),
        )
        .route(
            "/companies/{company}/sheets/{sheet}/participants/{actor}",
            axum::routing::put(share).delete(unshare),
        )
        .route(
            "/companies/{company}/sheets/{sheet}/versions",
            get(versions).post(checkpoint),
        )
        .route(
            "/companies/{company}/sheets/{sheet}/versions/{version}",
            get(version),
        )
        .route(
            "/companies/{company}/sheets/{sheet}/versions/{version}/restore",
            post(restore),
        )
        .route(
            "/companies/{company}/sheets/{sheet}/collaboration",
            get(collaboration),
        )
        .layer(DefaultBodyLimit::max(restless_orgintel::MAX_SHEET_BYTES))
}
fn failure(error: anyhow::Error) -> Response<Body> {
    let message = error.to_string();
    let status = if message.contains("conflict") {
        StatusCode::CONFLICT
    } else if message.contains("unavailable") {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    no_store(api_error(status, "sheet", message))
}
async fn org(
    state: &RoomApiState,
    principal: &RequestPrincipal,
    company: &str,
) -> Result<restless_orgintel::OrgIntel> {
    anyhow::ensure!(principal.permits_company(company), "sheet is unavailable");
    let org = state.orgintel(company).await?;
    anyhow::ensure!(
        org.active_actor(principal.actor_id())
            .await?
            .is_some_and(|a| a.actor_class == "human"),
        "sheet is unavailable"
    );
    Ok(org)
}
async fn run(
    state: RoomApiState,
    principal: RequestPrincipal,
    company: String,
    op: SheetOperation,
) -> Response<Body> {
    match async {
        let org = org(&state, &principal, &company).await?;
        sheet_commands::execute(&org, principal.actor_id(), op).await
    }
    .await
    {
        Ok(value) => no_store(Json(value).into_response()),
        Err(e) => failure(e),
    }
}
async fn list(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath(c): AxumPath<String>,
) -> Response<Body> {
    run(s, p, c, SheetOperation::List).await
}
async fn list_archived(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath(c): AxumPath<String>,
) -> Response<Body> {
    run(s, p, c, SheetOperation::Archived).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchiveInput {
    archived: bool,
}
async fn archive(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath((c, sheet)): AxumPath<(String, Uuid)>,
    Json(i): Json<ArchiveInput>,
) -> Response<Body> {
    run(s, p, c, SheetOperation::Archive { sheet, archived: i.archived }).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    id: Uuid,
    title: String,
    #[serde(default = "visibility")]
    visibility: String,
}
fn visibility() -> String {
    "company".into()
}
async fn create(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath(c): AxumPath<String>,
    Json(i): Json<Create>,
) -> Response<Body> {
    run(
        s,
        p,
        c,
        SheetOperation::Create {
            id: i.id,
            title: i.title,
            visibility: i.visibility,
        },
    )
    .await
}
async fn read(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath((c, id)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    run(
        s,
        p,
        c,
        SheetOperation::Read {
            sheet: id,
            query: None,
        },
    )
    .await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    action: Value,
    expected_revision: Option<String>,
    key: Option<Uuid>,
}
async fn operation(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath((c, id)): AxumPath<(String, Uuid)>,
    Json(i): Json<Operation>,
) -> Response<Body> {
    let op = if sheet_commands::is_read_action(&i.action) {
        SheetOperation::Read {
            sheet: id,
            query: Some(i.action),
        }
    } else {
        let (Some(expected_revision), Some(key)) = (i.expected_revision, i.key) else {
            return failure(anyhow::anyhow!(
                "sheet edits require expected_revision and key"
            ));
        };
        SheetOperation::Edit {
            sheet: id,
            expected_revision,
            key,
            action: i.action,
        }
    };
    run(s, p, c, op).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Share {
    access: String,
}
async fn share(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath((c, id, actor)): AxumPath<(String, Uuid, String)>,
    Json(i): Json<Share>,
) -> Response<Body> {
    run(
        s,
        p,
        c,
        SheetOperation::Share {
            sheet: id,
            actor,
            access: Some(i.access),
        },
    )
    .await
}
async fn unshare(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath((c, id, actor)): AxumPath<(String, Uuid, String)>,
) -> Response<Body> {
    run(
        s,
        p,
        c,
        SheetOperation::Share {
            sheet: id,
            actor,
            access: None,
        },
    )
    .await
}
async fn versions(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath((c, id)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    run(s, p, c, SheetOperation::Versions { sheet: id }).await
}
async fn version(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath((c, id, version)): AxumPath<(String, Uuid, Uuid)>,
) -> Response<Body> {
    run(s, p, c, SheetOperation::Version { sheet: id, version }).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Restore {
    id: Uuid,
    title: String,
}
async fn restore(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath((c, id, version)): AxumPath<(String, Uuid, Uuid)>,
    Json(i): Json<Restore>,
) -> Response<Body> {
    run(
        s,
        p,
        c,
        SheetOperation::RestoreCopy {
            sheet: id,
            version,
            id: i.id,
            title: i.title,
        },
    )
    .await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    expected_revision: String,
    key: Uuid,
    title: String,
}
async fn checkpoint(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    AxumPath((c, id)): AxumPath<(String, Uuid)>,
    Json(i): Json<Checkpoint>,
) -> Response<Body> {
    run(
        s,
        p,
        c,
        SheetOperation::Checkpoint {
            sheet: id,
            expected_revision: i.expected_revision,
            key: i.key,
            title: i.title,
        },
    )
    .await
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Connect {
    #[serde(default)]
    after: i64,
    client_id: Option<Uuid>,
    reconnect_token: Option<String>,
}
async fn collaboration(
    State(s): State<RoomApiState>,
    RoomPrincipal(p): RoomPrincipal,
    lease: Option<Extension<SessionLease>>,
    AxumPath((c, id)): AxumPath<(String, Uuid)>,
    Query(q): Query<Connect>,
    ws: WebSocketUpgrade,
) -> Response<Body> {
    if s.network_mode && lease.as_ref().is_none_or(|Extension(l)| l.is_ended()) {
        return failure(anyhow::anyhow!("sheet is unavailable"));
    }
    let org = match org(&s, &p, &c).await {
        Ok(org) => org,
        Err(e) => return failure(e),
    };
    if q.after < 0 {
        return failure(anyhow::anyhow!("invalid sheet cursor"));
    }
    let initial = match org.sheet_state(id, p.actor_id(), false, q.after).await {
        Ok(v) => v,
        Err(e) => return failure(e.into()),
    };
    if q.after > initial.sheet.sequence {
        return failure(anyhow::anyhow!(
            "sheet cursor is ahead of the accepted head"
        ));
    }
    let resume = match (q.client_id, q.reconnect_token.as_deref()) {
        (None, None) => None,
        (Some(id), Some(token)) => Some((id, token)),
        _ => return failure(anyhow::anyhow!("invalid reconnect identity")),
    };
    let client = match org.claim_sheet_client(id, p.actor_id(), resume).await {
        Ok(c) => c,
        Err(e) => return failure(e.into()),
    };
    ws.max_message_size(restless_orgintel::MAX_SHEET_BYTES)
        .on_upgrade(move |socket| async move {
            let _ = session(
                socket,
                org,
                id,
                p.actor_id().to_string(),
                client,
                q.after,
                initial,
                lease.map(|Extension(l)| l),
            )
            .await;
        })
        .into_response()
}
/// Live cursors on one sheet, shared by every open session of it. Cursors are
/// ephemeral presence: never stored, never part of the revision log.
static SHEET_PRESENCE: std::sync::LazyLock<
    Mutex<HashMap<Uuid, tokio::sync::broadcast::Sender<(Uuid, Value)>>>,
> = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn sheet_presence(id: Uuid) -> tokio::sync::broadcast::Sender<(Uuid, Value)> {
    let mut hubs = SHEET_PRESENCE.lock().expect("sheet presence");
    hubs.retain(|_, hub| hub.receiver_count() > 0);
    hubs.entry(id)
        .or_insert_with(|| tokio::sync::broadcast::channel(64).0)
        .clone()
}

/// Where an agent's revision landed, so its cursor can show it working.
fn revision_position(message: &Value) -> Option<Value> {
    message["commands"].as_array()?.iter().find_map(|command| {
        Some(json!({
            "sheetId": command["sheetId"].as_str()?,
            "col": command["col"].as_u64()?,
            "row": command["row"].as_u64()?,
        }))
    })
}

async fn session(
    mut socket: WebSocket,
    org: restless_orgintel::OrgIntel,
    id: Uuid,
    actor: String,
    client: restless_orgintel::SheetClient,
    mut cursor: i64,
    initial: restless_orgintel::SheetState,
    lease: Option<SessionLease>,
) -> Result<()> {
    socket.send(AxumMessage::Text(json!({"type":"BOOTSTRAP","client_id":client.client_id,"reconnect_token":client.reconnect_token,"state":initial}).to_string().into())).await?;
    cursor = initial.sheet.sequence.max(cursor);
    let names: HashMap<String, String> = org
        .list_actors()
        .await
        .map(|actors| actors.into_iter().map(|a| (a.id, a.display)).collect())
        .unwrap_or_default();
    let display = |actor_id: &str| names.get(actor_id).cloned().unwrap_or_else(|| actor_id.to_string());
    let hub = sheet_presence(id);
    let mut presence = hub.subscribe();
    let mut interval = tokio::time::interval(Duration::from_millis(200));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _=optional_session_ended(lease.as_ref())=>break,
            _=interval.tick()=>{
                if !org.sheet_client_active(id,&actor,client.client_id,client.generation).await? {break;}
                let state=match org.sheet_state(id,&actor,false,cursor).await {Ok(s)=>s,Err(_)=>break};
                for m in state.messages {
                    cursor=m.sequence;
                    socket.send(AxumMessage::Text(json!({"type":"MESSAGE","sequence":cursor,"message":m.message}).to_string().into())).await?;
                    // Someone else's edit moves their cursor to where it landed;
                    // for an agent this is the only cursor it has.
                    if m.actor_id != actor && m.message["type"] == "REMOTE_REVISION" {
                        if let (Some(position), Some(client_id)) = (revision_position(&m.message), m.message["clientId"].as_str()) {
                            let moved = json!({"type":"CLIENT_MOVED","version":1,"client":{"id":client_id,"name":display(&m.actor_id),"position":position}});
                            socket.send(AxumMessage::Text(json!({"type":"PRESENCE","message":moved}).to_string().into())).await?;
                        }
                    }
                }
                if state.access!="edit" {socket.send(AxumMessage::Text(json!({"type":"ACCESS","access":state.access}).to_string().into())).await?;}
            }
            shared=presence.recv()=>{
                match shared {
                    Ok((from, message)) if from != client.client_id => {
                        socket.send(AxumMessage::Text(json!({"type":"PRESENCE","message":message}).to_string().into())).await?;
                    }
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming=socket.recv()=>{
                let Some(incoming)=incoming else {break};
                let incoming=incoming?;
                let AxumMessage::Text(text)=incoming else {if matches!(incoming,AxumMessage::Close(_)){break;}continue;};
                // A cursor goes to the other sessions on this sheet, under this
                // session's bound identity and name; it is never stored.
                if let Ok(mut message) = serde_json::from_str::<Value>(&text) {
                    let kind = message["type"].as_str().unwrap_or("").to_string();
                    if matches!(kind.as_str(), "CLIENT_JOINED" | "CLIENT_MOVED") {
                        message["client"]["id"] = json!(client.client_id.to_string());
                        message["client"]["name"] = json!(display(&actor));
                        let _ = hub.send((client.client_id, message));
                        continue;
                    }
                    if kind == "CLIENT_LEFT" {
                        let _ = hub.send((client.client_id, json!({"type":"CLIENT_LEFT","version":1,"clientId":client.client_id.to_string()})));
                        continue;
                    }
                }
                let result=sheet_commands::accept_browser(&org,id,&actor,&client,&text).await;
                if let Err(e)=result {
                    socket.send(AxumMessage::Text(json!({"type":"ERROR","message":e.to_string()}).to_string().into())).await?;
                    break;
                }
            }
        }
    }
    let _ = hub.send((client.client_id, json!({"type":"CLIENT_LEFT","version":1,"clientId":client.client_id.to_string()})));
    let _ = socket.close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{FutureExt, SinkExt, StreamExt};
    use std::panic::AssertUnwindSafe;
    use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

    #[tokio::test]
    async fn native_sheets_verified_member_http_ws_and_revocation() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            return;
        };
        let _owned_worker = sheet_commands::SHEET_TEST_LOCK.lock().await;
        sheet_commands::reset_test_worker().await;
        assert!(url::Url::parse(&url).unwrap().path().ends_with("_test"));
        let company = format!(
            "sheet_member_{}_test",
            &Uuid::new_v4().simple().to_string()[..12]
        );
        let org = restless_orgintel::OrgIntel::ensure(&url, &company)
            .await
            .unwrap();
        org.ensure_actor("owner", "owner", "owner", "Owner")
            .await
            .unwrap();
        org.ensure_actor("alice", "human", "member", "Alice")
            .await
            .unwrap();
        let sheet = Uuid::new_v4();
        sheet_commands::execute(
            &org,
            "owner",
            SheetOperation::Create {
                id: sheet,
                title: "Private deals".into(),
                visibility: "participants".into(),
            },
        )
        .await
        .unwrap();
        org.share_sheet(sheet, "owner", "alice", Some("edit"))
            .await
            .unwrap();
        let sessions = Arc::new(SessionStore::default());
        let identity = VerifiedIdentity {
            user: "alice-user".into(),
            issuer: Some("https://issuer.restless.test".into()),
            owner: "company-owner".into(),
            scope: CompanyScope::Company {
                company: company.clone(),
            },
            role: "member".into(),
            actor: Some("alice".into()),
            company_id: Some(Uuid::new_v4()),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some("membership-alice".into()),
            membership_version: Some(1),
            display_name: None,
        };
        let token = sessions.establish(identity, Duration::from_secs(60));
        let mut handles = HashMap::new();
        handles.insert(company.clone(), org.clone());
        let mut state = RoomApiState::fixed(handles, url.clone());
        state.network_mode = true;
        let authenticated = sessions.clone();
        // Authentication produces the same verified principal and lease used
        // by entry. Exercise its actual membership boundary with the handlers.
        let app = Router::new()
            .nest("/api", routes::<RoomApiState>())
            .layer(axum::middleware::from_fn(
                move |mut request: Request, next: Next| {
                    let sessions = authenticated.clone();
                    async move {
                        let Some(lease) = cookie_value(request.headers(), SESSION_COOKIE)
                            .and_then(|t| sessions.resolve_lease(&t))
                        else {
                            return api_error(
                                StatusCode::UNAUTHORIZED,
                                "session",
                                "session required",
                            );
                        };
                        let principal = RequestPrincipal::from_verified(&lease.identity).unwrap();
                        if let Some(refusal) = membership_boundary_violation(
                            request.method(),
                            request.uri().path(),
                            &principal,
                        ) {
                            return api_error(refusal.status, refusal.code, refusal.message);
                        }
                        request.extensions_mut().insert(principal);
                        request.extensions_mut().insert(lease);
                        next.run(request).await
                    }
                },
            ))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let outcome = AssertUnwindSafe(async {
            let client = reqwest::Client::new();
            let origin = format!("http://{address}");
            let path = format!("/api/companies/{company}/sheets/{sheet}");
            let cookie = format!("{SESSION_COOKIE}={token}");
            assert_eq!(
                client
                    .get(format!("{origin}{path}"))
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::UNAUTHORIZED
            );
            let response = client
                .get(format!("{origin}{path}"))
                .header(COOKIE, &cookie)
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body: Value = response.json().await.unwrap();
            assert_eq!(body["access"], "edit");
            let denied = client
                .post(format!("{origin}/api/companies/{company}/up"))
                .header(COOKIE, &cookie)
                .send()
                .await
                .unwrap();
            assert_eq!(denied.status(), StatusCode::FORBIDDEN);
            let mut request = format!("ws://{address}{path}/collaboration")
                .into_client_request()
                .unwrap();
            request
                .headers_mut()
                .insert(COOKIE, HeaderValue::from_str(&cookie).unwrap());
            let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
            let bootstrap = socket.next().await.unwrap().unwrap();
            let bootstrap: Value = serde_json::from_str(bootstrap.to_text().unwrap()).unwrap();
            assert_eq!(bootstrap["state"]["access"], "edit");
            let current = org.sheet_state(sheet, "alice", true, 0).await.unwrap();
            let mut input = sheet_commands::model_input(&current);
            input["client_id"] = bootstrap["client_id"].clone();
            input["operation"] =
                json!({"action":"set_cells","values":[["Shared member",2100,3100,"=C1-B1"]]});
            let output = sheet_commands::model(input).await.unwrap();
            for message in output["messages"].as_array().unwrap() {
                socket
                    .send(Message::Text(message.to_string().into()))
                    .await
                    .unwrap();
            }
            for _ in 0..4 {
                let accepted = tokio::time::timeout(Duration::from_secs(3), socket.next())
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    serde_json::from_str::<Value>(accepted.to_text().unwrap()).unwrap()["type"],
                    "MESSAGE"
                );
            }
            // Another actor's edit reaches this session with that actor's
            // cursor on the cell it changed; agents have no other cursor.
            let head = org.sheet_state(sheet, "owner", false, 0).await.unwrap().sheet.head_revision;
            sheet_commands::execute(
                &org,
                "owner",
                SheetOperation::Edit {
                    sheet,
                    expected_revision: head,
                    key: Uuid::new_v4(),
                    action: json!({"action":"set_cells","start":"E7","values":[["by owner"]]}),
                },
            )
            .await
            .unwrap();
            let mut cursor = None;
            for _ in 0..4 {
                let Ok(Some(Ok(frame))) =
                    tokio::time::timeout(Duration::from_secs(3), socket.next()).await
                else {
                    break;
                };
                let frame: Value = serde_json::from_str(frame.to_text().unwrap()).unwrap();
                if frame["type"] == "PRESENCE" {
                    cursor = Some(frame["message"].clone());
                    break;
                }
            }
            let cursor = cursor.expect("an edit by another actor moves their cursor");
            assert_eq!(cursor["type"], "CLIENT_MOVED");
            assert_eq!(cursor["client"]["name"], "Owner");
            assert_eq!(cursor["client"]["position"], json!({"sheetId":"Sheet1","col":4,"row":6}));
            let read = client
                .post(format!("{origin}{path}/operations"))
                .header(COOKIE, &cookie)
                .json(&json!({"action":{"action":"get_range","range":"D1"}}))
                .send()
                .await
                .unwrap();
            assert_eq!(read.status(), StatusCode::OK);
            assert_eq!(
                read.json::<Value>().await.unwrap()["result"]["rows"][0][0]["value"],
                1000
            );
            sessions.revoke(&token);
            let closed = tokio::time::timeout(Duration::from_secs(3), socket.next())
                .await
                .unwrap();
            assert!(
                closed.is_none()
                    || closed.is_some_and(|r| r.is_err() || r.is_ok_and(|m| m.is_close()))
            );
            assert_eq!(
                client
                    .get(format!("{origin}{path}"))
                    .header(COOKIE, &cookie)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::UNAUTHORIZED
            );
        })
        .catch_unwind()
        .await;
        server.abort();
        let _ = server.await;
        org.close().await;
        let cleanup = sqlx::PgPool::connect(&url).await.unwrap();
        sqlx::query(&format!("DROP SCHEMA {company} CASCADE"))
            .execute(&cleanup)
            .await
            .unwrap();
        cleanup.close().await;
        sheet_commands::reset_test_worker().await;
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}
