//! Direction and permission (S80): requests, applied changes with Undo, standing grants and the
//! day's digest, for whoever owns the direction and for the people who asked.
//!
//! Every answer is read for the person asking: a member sees their own requests and changes, the
//! person a request is routed to may decide it, and the owner sees and decides everything. The
//! governance store is the writer; these handlers only check who is asking and shape the answer.
use super::*;
use restless_engine::direction::{DirectionAppliedPage, DirectionDecisionInput, DirectionGrantList, DirectionRequestPage};
use restless_engine::direction_authority::{self as direction, DirectionError, Names};

/// The id the governance store knows the viewer by: the owner is `owner` whatever their actor.
fn viewer(principal: &RequestPrincipal) -> &str {
    if principal.membership_role() == "owner" { "owner" } else { principal.actor_id() }
}

/// Who decides or undoes: an admin runs the day to day, so acts with the owner's say under their
/// own name. Standing permissions stay the owner's alone.
fn acting(principal: &RequestPrincipal) -> direction::Acting<'_> {
    match principal.membership_role() {
        "owner" => direction::Acting { id: "owner", owner: true },
        "admin" => direction::Acting { id: principal.actor_id(), owner: true },
        _ => direction::Acting { id: principal.actor_id(), owner: false },
    }
}

/// What a person reads: an admin sees what the owner sees, except their own requests.
fn reader<'a>(principal: &'a RequestPrincipal, mine: bool) -> &'a str {
    if principal.membership_role() == "admin" && !mine { "owner" } else { viewer(principal) }
}

async fn setup(
    state: &OwnerState,
    principal: &RequestPrincipal,
    company: &str,
) -> std::result::Result<(restless_orgintel::OrgIntel, Names), Response<Body>> {
    if !principal.permits_company(company) {
        return Err(api_error(StatusCode::NOT_FOUND, "company", "company is unavailable"));
    }
    let org = state.daemon.orgintel.get(company).await.map_err(|error| {
        tracing::error!(%error, "direction: OrgIntel unavailable");
        api_error(StatusCode::SERVICE_UNAVAILABLE, "orgintel", "company coordination is temporarily unavailable")
    })?;
    let names = Names::read(&org).await.map_err(|error| {
        tracing::error!(%error, "direction: names unavailable");
        api_error(StatusCode::SERVICE_UNAVAILABLE, "orgintel", "company coordination is temporarily unavailable")
    })?;
    Ok((org, names))
}

fn refused(error: DirectionError) -> Response<Body> {
    let (status, code) = match &error {
        DirectionError::AlreadyDecided(_) => (StatusCode::CONFLICT, "already_decided"),
        DirectionError::NotYours(_) => (StatusCode::FORBIDDEN, "not_yours"),
        DirectionError::Invalid(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid"),
        DirectionError::Missing(_) => (StatusCode::NOT_FOUND, "missing"),
        DirectionError::Store(inner) => {
            tracing::error!(error = %format!("{inner:#}"), "direction store failed");
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "direction",
                "Direction is temporarily unavailable. Try again in a moment.",
            );
        }
    };
    no_store(
        (
            status,
            Json(serde_json::json!({ "error": "direction", "code": code, "message": error.to_string() })),
        )
            .into_response(),
    )
}

fn unavailable(error: anyhow::Error) -> Response<Body> {
    refused(DirectionError::Store(error))
}

fn no_store(mut response: Response<Body>) -> Response<Body> {
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[derive(Deserialize)]
pub(super) struct RequestsQuery {
    #[serde(default)]
    state: Option<String>,
}

/// `GET /direction/requests?state=open|mine`
pub(super) async fn requests(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<RequestsQuery>,
) -> Response<Body> {
    let (_, names) = match setup(&state, &principal, &company).await {
        Ok(ready) => ready,
        Err(response) => return response,
    };
    let mine = query.state.as_deref() == Some("mine");
    match direction::requests(&state.daemon.authority, &company, &names, reader(&principal, mine), mine).await {
        Ok(items) => no_store(Json(DirectionRequestPage { items }).into_response()),
        Err(error) => unavailable(error),
    }
}

/// `POST /direction/requests/{id}/decision`
pub(super) async fn decide(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, request)): AxumPath<(String, Uuid)>,
    Json(input): Json<DirectionDecisionInput>,
) -> Response<Body> {
    let (org, names) = match setup(&state, &principal, &company).await {
        Ok(ready) => ready,
        Err(response) => return response,
    };
    match direction::decide(&state.daemon.authority, &org, &company, &names, request, acting(&principal), &input).await {
        Ok(decided) => {
            // An approved request is Exec's to carry out at once: wake it with the request.
            if decided.state == restless_engine::direction::RequestState::Approved {
                wake_exec_for(&state, &company, &decided).await;
            } else if decided.state == restless_engine::direction::RequestState::Declined {
                tell_declined(&org, &decided).await;
            }
            no_store(Json(decided).into_response())
        }
        Err(error) => refused(error),
    }
}

/// The person who asked hears a decline where they asked: in their room with Exec, with the reason.
async fn tell_declined(org: &restless_orgintel::OrgIntel, request: &restless_engine::direction::DirectionRequest) {
    let Some(decision) = &request.decision else { return };
    if decision.by.actor_id == request.requester.actor_id {
        return;
    }
    let mut line = format!("{} declined: {}.", decision.by.name, request.summary.trim_end_matches('.'));
    if let Some(reason) = decision.reason.as_deref().filter(|reason| !reason.trim().is_empty()) {
        line.push_str(&format!(" “{}”", reason.trim()));
    }
    if let Err(error) = org.send_message("exec", Some(&request.requester.actor_id), &line).await {
        tracing::warn!(%error, "could not tell the requester about a decline");
    }
}

/// Tell Exec to carry an approved request out on its next wake.
async fn wake_exec_for(
    state: &OwnerState,
    company: &str,
    request: &restless_engine::direction::DirectionRequest,
) {
    let body = format!(
        "{} approved {}'s request: {}. Carry it out now: {}. Then record it with `restless direction done --request {}` (or `--failure \"<why>\"`).",
        request.decision.as_ref().map(|decision| decision.by.name.as_str()).unwrap_or("The owner"),
        request.requester.name,
        request.summary,
        request.change,
        request.id,
    );
    if let Ok(org) = state.daemon.orgintel.get(company).await {
        // From the daemon, not the decider: it is Exec's work order, and the person who decided
        // must not find a command and an id in their own chat under their name.
        if let Err(error) = org.send_message("daemon", Some("exec"), &body).await {
            tracing::warn!(%error, "could not tell Exec about an approved request");
        }
    }
}

#[derive(Deserialize)]
pub(super) struct AppliedQuery {
    #[serde(default)]
    since: Option<DateTime<Utc>>,
}

/// `GET /direction/applied?since=`
pub(super) async fn applied(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<AppliedQuery>,
) -> Response<Body> {
    let (_, names) = match setup(&state, &principal, &company).await {
        Ok(ready) => ready,
        Err(response) => return response,
    };
    match direction::applied(&state.daemon.authority, &company, &names, reader(&principal, false), query.since).await {
        Ok(items) => no_store(Json(DirectionAppliedPage { items }).into_response()),
        Err(error) => unavailable(error),
    }
}

/// `POST /direction/applied/{id}/undo`
pub(super) async fn undo(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, applied)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let (org, names) = match setup(&state, &principal, &company).await {
        Ok(ready) => ready,
        Err(response) => return response,
    };
    match direction::undo(&state.daemon.authority, &org, &company, &names, applied, acting(&principal)).await {
        Ok(entry) => no_store(Json(entry).into_response()),
        Err(error) => refused(error),
    }
}

/// `GET /direction/grants`: the owner's standing permissions.
pub(super) async fn grants(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return refused(DirectionError::NotYours("Standing permissions are the owner's.".into()));
    }
    let (_, names) = match setup(&state, &principal, &company).await {
        Ok(ready) => ready,
        Err(response) => return response,
    };
    match direction::active_grants(&state.daemon.authority, &company, &names).await {
        Ok(items) => no_store(Json(DirectionGrantList { items }).into_response()),
        Err(error) => unavailable(error),
    }
}

/// `POST /direction/grants/{id}/revoke`
pub(super) async fn revoke(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, grant)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return refused(DirectionError::NotYours("Standing permissions are the owner's.".into()));
    }
    if let Err(response) = setup(&state, &principal, &company).await {
        return response;
    }
    match direction::revoke_grant(&state.daemon.authority, &company, grant, viewer(&principal)).await {
        Ok(()) => no_store(StatusCode::NO_CONTENT.into_response()),
        Err(error) => refused(error),
    }
}

#[derive(Deserialize)]
pub(super) struct DigestQuery {
    #[serde(default)]
    day: Option<chrono::NaiveDate>,
}

/// `GET /direction/digest?day=`
pub(super) async fn digest(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<DigestQuery>,
) -> Response<Body> {
    let (_, names) = match setup(&state, &principal, &company).await {
        Ok(ready) => ready,
        Err(response) => return response,
    };
    let day = query.day.unwrap_or_else(|| Utc::now().date_naive());
    match direction::digest(&state.daemon.authority, &company, &names, reader(&principal, false), day).await {
        Ok(digest) => no_store(Json(digest).into_response()),
        Err(error) => unavailable(error),
    }
}
