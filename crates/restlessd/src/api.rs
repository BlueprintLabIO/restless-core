//! The owner cockpit's HTTP transport.
//!
//! This module deliberately contains **no product logic**. Every route builds
//! the same JSON request the CLI puts on the socket and hands it to
//! `crate::dispatch_value`, which runs the same `authorize` → `dispatch` path.
//! A command reachable here is reachable there, with the same authority gate and
//! the same `{ok, data}` / `{ok, error:{kind, message}}` envelope.
//!
//! That is the whole design, and it is the design because the CLI already *was*
//! the API: a dumb client over a line protocol with a principal on every
//! request, typed errors, and one streaming command. Re-deriving a REST API from
//! product concepts would have produced a second surface to keep in step.
//!
//! ## Authority
//!
//! These routes are **not a listener**. They are mounted by `owner::serve` on
//! the owner gateway, behind the same signed-cookie check as `/api`, and there
//! is exactly one HTTP port for the owner (`7788`).
//!
//! That matters more than tidiness. This module stamps `principal: "owner"` on
//! every request it forwards, so anything that can reach it can act as the
//! owner. On its own loopback listener that stamp rested on "a process on this
//! machine is the owner" — the same claim the unix socket makes, and a claim
//! rather than a proof (`S04-T10`). Once the daemon grew a *token-authenticated*
//! gateway, a second unauthenticated port asserting the same authority was no
//! longer a parallel claim but a way around the stronger one. So the weaker gate
//! was removed rather than kept beside it.
//!
//! What survives is the rule that the stamp travels *through* `authorize` rather
//! than around it, so `OWNER_ONLY` still decides which commands are acts of
//! owner authority. The gate is never "which listener did this arrive on".
//!
//! `public()` — health, the spec, the docs page — carries no company data and
//! stays open, because a health check that needs a credential is a health check
//! nothing runs.
//!
//! ## Stubs
//!
//! Routes under "not implemented yet" answer `{ok: true, data: null, stub: ...}`.
//! They exist so the frontend can be written against the finished shape and so
//! the gap is visible in the OpenAPI document instead of living in someone's
//! head. `docs/api/MISSING.md` says what each one is for. A stub returns null —
//! it never returns a plausible-looking empty list, because a UI cannot tell
//! "no settings" from "not built" and would render the wrong thing confidently.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response as HttpResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_stream::wrappers::ReceiverStream;

use crate::Daemon;

/// The OpenAPI document, embedded from the repo so the served spec and the
/// committed file are the same bytes and cannot drift.
const OPENAPI_YAML: &str = include_str!("../../../docs/api/openapi.yaml");


/// Route paths, named once and used twice: by the router below, and by the test
/// that asserts every one of them is documented in `openapi.yaml`. A path that
/// exists but is undocumented is worse than a missing route — the frontend
/// builds against the document.
pub(crate) mod path {
    pub const HEALTH: &str = "/v1/health";
    pub const OPENAPI: &str = "/v1/openapi.yaml";
    pub const DOCS: &str = "/v1/docs";
    pub const STATUS: &str = "/v1/companies/{company}/status";
    pub const PEOPLE: &str = "/v1/companies/{company}/people";
    pub const GOALS: &str = "/v1/companies/{company}/goals";
    pub const COMMITMENTS: &str = "/v1/companies/{company}/commitments";
    pub const INBOX: &str = "/v1/companies/{company}/inbox";
    pub const EVENTS: &str = "/v1/companies/{company}/events";
    pub const RECEIPTS: &str = "/v1/companies/{company}/receipts";
    pub const SPEND: &str = "/v1/companies/{company}/spend";
    pub const STREAM: &str = "/v1/companies/{company}/stream";
    pub const UP: &str = "/v1/companies/{company}/up";
    pub const TELL: &str = "/v1/companies/{company}/tell";
    pub const WAKE: &str = "/v1/companies/{company}/wake";
    pub const MESSAGES: &str = "/v1/companies/{company}/messages";
    pub const STAFF: &str = "/v1/companies/{company}/staff";
    pub const APPROVALS: &str = "/v1/companies/{company}/approvals";
    pub const COMMITMENT_STATE: &str = "/v1/companies/{company}/commitments/{id}/state";
    pub const CREATE_COMPANY: &str = "/v1/companies";
    // Deliberately absent: the attention queue. `attention::project` is owned by
    // the owner gateway at `GET /api/companies/{company}/attention`, and a
    // second path to the same projection is the duplication the working
    // agreement calls accumulation. The cockpit reads it from `/api`.
    pub const AUTHORITY: &str = "/v1/companies/{company}/authority";
    pub const PERSON: &str = "/v1/companies/{company}/people/{actor}";
    pub const ORG: &str = "/v1/companies/{company}/org";
    pub const ARTIFACTS: &str = "/v1/companies/{company}/artifacts";

    /// Every path above. The drift guard walks this.
    pub const ALL: &[&str] = &[
        HEALTH, OPENAPI, DOCS, STATUS, PEOPLE, GOALS, COMMITMENTS, INBOX, EVENTS,
        RECEIPTS, SPEND, STREAM, TELL, WAKE, MESSAGES, STAFF, APPROVALS,
        UP, COMMITMENT_STATE, CREATE_COMPANY, AUTHORITY, PERSON, ORG, ARTIFACTS,
    ];
}

type Ctx = State<Arc<Daemon>>;

/// Health, spec and docs. No company data passes through these, so they are
/// mounted outside the owner gate — a health check that needs a credential is
/// one that nothing runs.
pub(crate) fn public() -> Router {
    Router::new()
        .route(path::HEALTH, get(health))
        .route(path::OPENAPI, get(openapi))
        .route(path::DOCS, get(docs))
}

/// Everything that touches a company. `owner::serve` mounts this behind the
/// owner cookie; it must never be mounted without one.
pub(crate) fn guarded(daemon: Arc<Daemon>) -> Router {
    Router::new()
        // ---- reads, all backed today --------------------------------------
        .route(path::STATUS, get(status))
        .route(path::PEOPLE, get(people))
        .route(path::GOALS, get(goals))
        .route(path::COMMITMENTS, get(commitments))
        .route(path::INBOX, get(inbox))
        .route(path::EVENTS, get(events))
        .route(path::RECEIPTS, get(receipts))
        .route(path::SPEND, get(spend))
        .route(path::STREAM, get(stream))
        // ---- writes, all backed today -------------------------------------
        .route(path::UP, post(up))
        .route(path::TELL, post(tell))
        .route(path::WAKE, post(wake))
        .route(path::MESSAGES, post(message))
        .route(path::STAFF, post(spawn))
        .route(path::APPROVALS, post(approve))
        .route(path::COMMITMENT_STATE, post(commitment_state))
        .route(path::CREATE_COMPANY, post(create_company))
        // ---- not implemented yet: see docs/api/MISSING.md -----------------
        .route(path::AUTHORITY, get(stub_authority))
        .route(path::PERSON, get(stub_person))
        .route(path::ORG, get(stub_org))
        .route(path::ARTIFACTS, get(stub_artifacts))
        .with_state(daemon)
}

/// Build the socket request, run it, and shape the result as HTTP.
///
/// The envelope is preserved verbatim in the body — the status code is a second
/// signal for proxies and `fetch`, never the only one, because `kind` carries
/// distinctions HTTP has no code for.
async fn run(daemon: &Daemon, mut request: Value) -> HttpResponse {
    if let Some(object) = request.as_object_mut() {
        object.insert("principal".into(), "owner".into());
    }
    let response = crate::dispatch_value(request, daemon).await;
    let status = if response["ok"].as_bool() == Some(true) {
        StatusCode::OK
    } else {
        match response["error"]["kind"].as_str() {
            Some("authority") => StatusCode::FORBIDDEN,
            Some("transport") => StatusCode::BAD_GATEWAY,
            _ => StatusCode::BAD_REQUEST,
        }
    };
    (status, Json(response)).into_response()
}

/// A route whose shape is agreed but whose implementation does not exist.
fn stub(what: &str) -> HttpResponse {
    Json(json!({
        "ok": true,
        "data": Value::Null,
        "stub": {
            "implemented": false,
            "what": what,
            "see": "docs/api/MISSING.md",
        }
    }))
    .into_response()
}

// ---- meta ------------------------------------------------------------------

async fn health() -> HttpResponse {
    Json(json!({ "ok": true, "data": { "service": "restlessd", "api": "v1" } })).into_response()
}

async fn openapi() -> HttpResponse {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/yaml"),
        )],
        OPENAPI_YAML,
    )
        .into_response()
}

/// Redoc over the embedded spec. One script tag, no build step, and the page is
/// the thing you send someone who asks what the API is.
async fn docs() -> HttpResponse {
    axum::response::Html(
        r#"<!doctype html>
<html><head><title>Restless API</title><meta charset="utf-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<style>body{margin:0}</style></head>
<body><redoc spec-url="/v1/openapi.yaml"></redoc>
<script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body></html>"#,
    )
    .into_response()
}

// ---- reads -----------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct LimitQuery {
    limit: Option<i64>,
    capability: Option<String>,
    #[serde(rename = "as")]
    as_actor: Option<String>,
}

async fn status(State(daemon): Ctx, Path(company): Path<String>) -> HttpResponse {
    run(&daemon, json!({ "cmd": "status", "company": company })).await
}

async fn people(State(daemon): Ctx, Path(company): Path<String>) -> HttpResponse {
    run(&daemon, json!({ "cmd": "people", "company": company })).await
}

async fn goals(State(daemon): Ctx, Path(company): Path<String>) -> HttpResponse {
    run(&daemon, json!({ "cmd": "goals", "company": company })).await
}

async fn commitments(State(daemon): Ctx, Path(company): Path<String>) -> HttpResponse {
    run(&daemon, json!({ "cmd": "commitments", "company": company })).await
}

/// Reading the owner's own inbox marks it read; passing `?as=` inspects another
/// actor's without marking — an observer must not hide mail from its addressee.
async fn inbox(
    State(daemon): Ctx,
    Path(company): Path<String>,
    Query(q): Query<LimitQuery>,
) -> HttpResponse {
    run(
        &daemon,
        json!({ "cmd": "inbox", "company": company, "as_actor": q.as_actor }),
    )
    .await
}

async fn events(
    State(daemon): Ctx,
    Path(company): Path<String>,
    Query(q): Query<LimitQuery>,
) -> HttpResponse {
    run(
        &daemon,
        json!({ "cmd": "events", "company": company, "limit": q.limit }),
    )
    .await
}

async fn receipts(
    State(daemon): Ctx,
    Path(company): Path<String>,
    Query(q): Query<LimitQuery>,
) -> HttpResponse {
    run(
        &daemon,
        json!({
            "cmd": "receipts", "company": company,
            "capability": q.capability, "limit": q.limit,
        }),
    )
    .await
}

async fn spend(State(daemon): Ctx, Path(company): Path<String>) -> HttpResponse {
    run(&daemon, json!({ "cmd": "spend", "company": company })).await
}

/// The operational event stream as SSE: a recent snapshot, then new events as
/// they land. Same poll and same watermark as the socket's `watch`, against the
/// same durable table — the stream survives the client's attention, not the
/// other way around.
async fn stream(State(daemon): Ctx, Path(company): Path<String>) -> HttpResponse {
    let (tx, rx) = tokio::sync::mpsc::channel::<std::result::Result<Event, std::io::Error>>(64);
    tokio::spawn(async move {
        let org = match daemon.orgintel.get(&company).await {
            Ok(org) => org,
            Err(error) => {
                let body = json!({ "ok": false, "error": { "kind": "error", "message": format!("{error:#}") } });
                let _ = tx.send(Ok(Event::default().event("error").data(body.to_string()))).await;
                return;
            }
        };
        let mut watermark: i64 = 0;
        match org.list_events(20).await {
            Ok(recent) => {
                for event in recent.iter().rev() {
                    let data = serde_json::to_string(event).unwrap_or_default();
                    if tx.send(Ok(Event::default().data(data))).await.is_err() {
                        return; // client went away
                    }
                    watermark = watermark.max(event.id);
                }
            }
            Err(error) => {
                let body = json!({ "ok": false, "error": { "kind": "error", "message": format!("{error:#}") } });
                let _ = tx.send(Ok(Event::default().event("error").data(body.to_string()))).await;
                return;
            }
        }
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            match org.events_after(watermark).await {
                Ok(events) => {
                    for event in events {
                        let data = serde_json::to_string(&event).unwrap_or_default();
                        if tx.send(Ok(Event::default().data(data))).await.is_err() {
                            return;
                        }
                        watermark = event.id;
                    }
                }
                Err(error) => {
                    let body = json!({ "ok": false, "error": { "kind": "error", "message": format!("{error:#}") } });
                    let _ = tx.send(Ok(Event::default().event("error").data(body.to_string()))).await;
                    return;
                }
            }
        }
    });
    Sse::new(ReceiverStream::new(rx)).into_response()
}

// ---- writes ----------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct UpBody {
    /// Rebuild the company image first. **Minutes**, not seconds — it compiles
    /// Rust and installs a browser. Left to the CLI by default; a browser
    /// request that hangs for ten minutes is not a UI anyone wants.
    #[serde(default)]
    reconcile: bool,
}

async fn up(
    State(daemon): Ctx,
    Path(company): Path<String>,
    Json(input): Json<UpBody>,
) -> HttpResponse {
    run(
        &daemon,
        json!({ "cmd": "up", "company": company, "reconcile": input.reconcile }),
    )
    .await
}

#[derive(Debug, Deserialize)]
struct TellBody {
    body: String,
}

async fn tell(
    State(daemon): Ctx,
    Path(company): Path<String>,
    Json(input): Json<TellBody>,
) -> HttpResponse {
    run(
        &daemon,
        json!({ "cmd": "tell", "company": company, "body": input.body }),
    )
    .await
}

#[derive(Debug, Deserialize)]
struct WakeBody {
    #[serde(default)]
    reason: Option<String>,
}

async fn wake(
    State(daemon): Ctx,
    Path(company): Path<String>,
    Json(input): Json<WakeBody>,
) -> HttpResponse {
    run(
        &daemon,
        json!({ "cmd": "wake", "company": company, "reason": input.reason }),
    )
    .await
}

#[derive(Debug, Deserialize)]
struct MessageBody {
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
    body: String,
}

async fn message(
    State(daemon): Ctx,
    Path(company): Path<String>,
    Json(input): Json<MessageBody>,
) -> HttpResponse {
    run(
        &daemon,
        json!({
            "cmd": "message", "company": company,
            "from": input.from.unwrap_or_else(|| "owner".to_string()),
            "to": input.to, "body": input.body,
        }),
    )
    .await
}

#[derive(Debug, Deserialize)]
struct SpawnBody {
    name: String,
    #[serde(default)]
    repo: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    model: Option<String>,
    task: String,
}

async fn spawn(
    State(daemon): Ctx,
    Path(company): Path<String>,
    Json(input): Json<SpawnBody>,
) -> HttpResponse {
    run(
        &daemon,
        json!({
            "cmd": "spawn", "company": company, "name": input.name,
            "repo": input.repo, "role": input.role, "model": input.model,
            "body": input.task, "from": "owner",
        }),
    )
    .await
}

#[derive(Debug, Deserialize)]
struct CreateCompanyBody {
    name: String,
    model: String,
    #[serde(default)]
    mission: Option<String>,
    #[serde(default)]
    spend_ceiling_usd: Option<f64>,
}

/// The only route whose company comes from the body rather than the path —
/// there is no company yet to put in a path.
///
/// This takes fields and renders the TOML that `company-create` expects, rather
/// than asking a browser to post a config file. The command is the single write
/// path — it is what also initialises the company in Authority, which a second
/// creation path would silently skip.
///
/// The four fields are the whole of it, and that is the point: a company
/// created here has no providers, no credentials, no standing approvals and no
/// sender address, because `CompanyConfig`'s defaults are empty and nothing
/// here fills them. Reach into the world is a later owner decision, one at a
/// time.
async fn create_company(State(daemon): Ctx, Json(input): Json<CreateCompanyBody>) -> HttpResponse {
    let config = match crate::runtime::new_config(
        &input.name,
        input.mission.as_deref().unwrap_or_default(),
        &input.model,
        input.spend_ceiling_usd,
    ) {
        Ok(config) => config,
        // Refused before anything exists: an invalid name that reached `up`
        // would get a Docker volume and a container and *then* fail at the
        // schema step, leaving orphans named after a company that cannot exist.
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "ok": false,
                    "error": { "kind": "invalid", "message": format!("{error:#}") }
                })),
            )
                .into_response()
        }
    };
    let body = match toml::to_string_pretty(&config) {
        Ok(body) => body,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "ok": false,
                    "error": { "kind": "error", "message": format!("render config: {error}") }
                })),
            )
                .into_response()
        }
    };
    run(
        &daemon,
        json!({ "cmd": "company-create", "company": input.name, "body": body }),
    )
    .await
}

#[derive(Debug, Deserialize)]
struct ApproveBody {
    party: String,
}

async fn approve(
    State(daemon): Ctx,
    Path(company): Path<String>,
    Json(input): Json<ApproveBody>,
) -> HttpResponse {
    run(
        &daemon,
        json!({ "cmd": "approve", "company": company, "party": input.party }),
    )
    .await
}

#[derive(Debug, Deserialize)]
struct CommitmentStateBody {
    state: String,
    #[serde(default)]
    resolution: Option<String>,
}

async fn commitment_state(
    State(daemon): Ctx,
    Path((company, id)): Path<(String, String)>,
    Json(input): Json<CommitmentStateBody>,
) -> HttpResponse {
    run(
        &daemon,
        json!({
            "cmd": "commitment-state", "company": company, "id": id,
            "state": input.state,
            "resolution": input.resolution.unwrap_or_default(),
        }),
    )
    .await
}

// ---- not implemented yet ---------------------------------------------------

async fn stub_authority() -> HttpResponse {
    stub("every standing authority setting, as one flat list")
}

async fn stub_person() -> HttpResponse {
    stub("one person's page: focus, work, authority, spend, artifacts")
}

async fn stub_org() -> HttpResponse {
    stub("reporting lines — who answers to whom")
}

async fn stub_artifacts() -> HttpResponse {
    stub("what an actor has produced lately, by path")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt;

    use crate::test_daemon;

    /// The routes as the gateway mounts them, minus the owner cookie layer —
    /// which is `owner`'s to test, and which these routes know nothing about.
    fn app(daemon: Arc<Daemon>) -> Router {
        public().merge(guarded(daemon))
    }

    async fn get(path: &str) -> (StatusCode, Value) {
        let dir = tempfile::tempdir().expect("tempdir");
        send(app(test_daemon(dir.path())), "GET", path, Body::empty()).await
    }

    async fn send(app: Router, method: &str, path: &str, body: Body) -> (StatusCode, Value) {
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(body)
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("body");
        let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, value)
    }

    #[tokio::test]
    async fn health_answers_without_a_database() {
        let (status, body) = get(path::HEALTH).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["ok"], json!(true));
        assert_eq!(body["data"]["api"], json!("v1"));
    }

    /// The frontend builds against the document, so a route the document does
    /// not mention is a route nobody can use on purpose.
    #[test]
    fn every_route_is_documented() {
        let undocumented: Vec<&str> = path::ALL
            .iter()
            .copied()
            .filter(|route| !OPENAPI_YAML.contains(route))
            .collect();
        assert!(
            undocumented.is_empty(),
            "routes missing from docs/api/openapi.yaml: {undocumented:?}"
        );
    }

    /// A stub must be distinguishable from an empty answer. `data: []` would
    /// render as "nothing here"; `data: null` cannot be mistaken for content.
    #[tokio::test]
    async fn a_stub_returns_null_and_says_so() {
        for route in [
            "/v1/companies/acme/authority",
            "/v1/companies/acme/people/sage",
            "/v1/companies/acme/org",
            "/v1/companies/acme/artifacts",
        ] {
            let (status, body) = get(route).await;
            assert_eq!(status, StatusCode::OK, "{route}");
            assert!(body["data"].is_null(), "{route} must answer null");
            assert_eq!(body["stub"]["implemented"], json!(false), "{route}");
        }
    }

    /// A backed read with nothing behind it must fail *inside the envelope*,
    /// with a kind the UI can switch on — not as a panic and not as a bare 500.
    #[tokio::test]
    async fn a_backed_read_fails_in_the_envelope() {
        let (status, body) = get("/v1/companies/acme/goals").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["ok"], json!(false));
        assert!(
            body["error"]["kind"].is_string(),
            "failure must carry a kind: {body}"
        );
        assert!(body["error"]["message"].is_string());
    }

    /// Creating over an existing company would silently destroy its providers,
    /// credentials and standing approvals — the owner's accumulated decisions.
    /// Tested here because `company-create` is where the check now lives, and
    /// the refusal must arrive before Authority or Postgres are touched, which
    /// is why this passes without either running.
    #[tokio::test]
    async fn creating_over_an_existing_company_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("companies")).expect("companies dir");
        let existing = crate::runtime::new_config("thymelake", "first", "zai/glm-5.2", None)
            .expect("build config");
        crate::runtime::CompanyConfig::save(dir.path(), &existing).expect("save");

        let (status, body) = send(
            app(test_daemon(dir.path())),
            "POST",
            path::CREATE_COMPANY,
            Body::from(
                json!({ "name": "thymelake", "model": "zai/glm-5.2", "mission": "second" })
                    .to_string(),
            ),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["kind"], json!("conflict"), "{body}");
        let survivor = crate::runtime::CompanyConfig::load(dir.path(), "thymelake").expect("load");
        assert_eq!(survivor.mission, "first", "the original must survive");
    }

    /// The name becomes a Postgres schema, a Docker volume and a container. A
    /// bad one must be refused before any of the three is created, or `up`
    /// leaves orphans named after a company that cannot exist.
    #[tokio::test]
    async fn an_unusable_company_name_never_reaches_the_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (status, body) = send(
            app(test_daemon(dir.path())),
            "POST",
            path::CREATE_COMPANY,
            Body::from(json!({ "name": "Thyme Lake", "model": "zai/glm-5.2" }).to_string()),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["kind"], json!("invalid"), "{body}");
        assert!(
            !dir.path().join("companies").exists(),
            "a refused name must leave nothing behind"
        );
    }

    /// The shim must not become a second, weaker authority gate: owner-only
    /// commands are still decided by `authorize`, and the stamp travels through
    /// it rather than around it.
    #[tokio::test]
    async fn the_shim_runs_the_same_authority_gate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let daemon = test_daemon(dir.path());
        // Same path the routes take, but claiming a principal that may not approve.
        let mut request = json!({ "cmd": "approve", "company": "acme", "party": "x" });
        request["principal"] = json!("company/exec");
        let refused = crate::dispatch_value(request, &daemon).await;
        assert_eq!(refused["ok"], json!(false));
        assert_eq!(refused["error"]["kind"], json!("authority"));
    }
}
