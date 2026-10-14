//! People own teams: who owns a team, the goals a team delivers, and what a team's person
//! does with its Work. Every handler acts as the caller; OrgIntel decides whether that person
//! may act on that team, so the boundary only lets the calls through.

use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct TeamOwnerInput {
    /// The person's actor id; null hands the team back to the owner.
    #[serde(default)]
    person: Option<String>,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ReasonInput {
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct DelegateInput {
    /// The member's or admin's actor id.
    to: String,
    #[serde(default)]
    note: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct GoalTeamInput {
    /// The team that delivers the goal; null hands it back to the owner.
    #[serde(default)]
    team_id: Option<Uuid>,
}

async fn org_for(
    state: &OwnerState,
    company: &str,
) -> std::result::Result<restless_orgintel::OrgIntel, Response<Body>> {
    state.daemon.orgintel.get(company).await.map_err(|error| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            format!("{error:#}"),
        )
    })
}

fn refused(code: &'static str, error: restless_orgintel::OrgIntelError) -> Response<Body> {
    api_error(StatusCode::FORBIDDEN, code, error.plain_message())
}

/// The owner says which person owns a team, or hands it back. Owner only: the boundary
/// refuses everyone else before this runs.
pub(super) async fn set_team_owner(
    State(state): State<OwnerState>,
    AxumPath((company, team)): AxumPath<(String, Uuid)>,
    Json(input): Json<TeamOwnerInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let reason = match input.reason.trim() {
        "" => "Set by the owner",
        reason => reason,
    };
    let result = match input
        .person
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        Some(person) => {
            org.appoint_responsible_person(team, person, "owner", None, reason)
                .await
        }
        None => {
            org.release_responsible_person(team, "owner", None, reason)
                .await
        }
    };
    match result {
        Ok(()) => Json(serde_json::json!({
            "team_id": team,
            "owner_person": input.person,
        }))
        .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "team_owner", error.plain_message()),
    }
}

/// Who has owned a team, newest first: the record of each appointment and who made it.
pub(super) async fn team_owner_history(
    State(state): State<OwnerState>,
    AxumPath((company, team)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.team_responsibility_history(team).await {
        Ok(events) => Json(events).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "team_owner", error.plain_message()),
    }
}

/// Put a goal under a team, or back with the owner. The owner and Exec organise freely; a
/// person moves goals only between teams they own.
pub(super) async fn set_goal_team(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, goal)): AxumPath<(String, Uuid)>,
    Json(input): Json<GoalTeamInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .set_goal_team(goal, input.team_id, handoff_resolver(&principal))
        .await
    {
        Ok(()) => {
            Json(serde_json::json!({ "goal_id": goal, "team_id": input.team_id })).into_response()
        }
        Err(error) => refused("goal_team", error),
    }
}

/// A team's person passes judgement they should not decide up to the owner.
pub(super) async fn pass_handoff_to_owner(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, handoff)): AxumPath<(String, Uuid)>,
    Json(input): Json<ReasonInput>,
) -> Response<Body> {
    if principal.membership_role() == "owner" {
        return api_error(
            StatusCode::BAD_REQUEST,
            "handoff",
            "this is already the owner's decision",
        );
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .escalate_handoff(handoff, principal.actor_id(), input.reason.trim())
        .await
    {
        Ok(()) => {
            Json(serde_json::json!({ "handoff_id": handoff, "passed_to": "owner" })).into_response()
        }
        Err(error) => refused("handoff", error),
    }
}

/// Hand an owner handoff to a member or admin: it becomes exactly their Inbox item. The owner's
/// (or an admin's, acting with the owner's say); OrgIntel checks the person and the handoff.
pub(super) async fn delegate_handoff(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, handoff)): AxumPath<(String, Uuid)>,
    Json(input): Json<DelegateInput>,
) -> Response<Body> {
    if !matches!(principal.membership_role(), "owner" | "admin") {
        return api_error(
            StatusCode::FORBIDDEN,
            "handoff",
            "handing on an Inbox item is the owner's",
        );
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.delegate_handoff(handoff, input.to.trim(), input.note.trim()).await {
        Ok(()) => Json(serde_json::json!({ "handoff_id": handoff, "handed_to": input.to.trim() }))
            .into_response(),
        Err(error) => refused("handoff", error),
    }
}

/// Start repaired Work again. The owner, or the person who owns the Work's team.
pub(super) async fn resume_work(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, work)): AxumPath<(String, Uuid)>,
    Json(input): Json<ReasonInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .resume_work(work, handoff_resolver(&principal), input.reason.trim())
        .await
    {
        Ok(()) => Json(serde_json::json!({ "work_id": work, "resumed": true })).into_response(),
        Err(error) => refused("work", error),
    }
}

/// Stop Work for good. The owner, or the person who owns the Work's team.
pub(super) async fn abandon_work(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, work)): AxumPath<(String, Uuid)>,
    Json(input): Json<ReasonInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .abandon_work(work, handoff_resolver(&principal), input.reason.trim())
        .await
    {
        Ok(()) => Json(serde_json::json!({ "work_id": work, "abandoned": true })).into_response(),
        Err(error) => refused("work", error),
    }
}
