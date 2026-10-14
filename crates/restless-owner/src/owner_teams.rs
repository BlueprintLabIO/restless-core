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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NewTeamInput {
    name: String,
    /// What the team is accountable for.
    brief: String,
    /// The agent accountable for the team; it joins its own team.
    lead: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TeamChangeInput {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    brief: Option<String>,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TeamLeadInput {
    lead: String,
    #[serde(default)]
    reason: String,
}

fn owner_reason<'a>(reason: &'a str, fallback: &'a str) -> &'a str {
    match reason.trim() {
        "" => fallback,
        reason => reason,
    }
}

/// The owner forms a team around an accountable lead, as `restless team create` does, at the
/// company's own outcome standard. Owner only: the boundary refuses everyone else.
pub(super) async fn create_team(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<NewTeamInput>,
) -> Response<Body> {
    let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
        Ok(config) => config,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist."),
    };
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .create_team_with_standard(
            input.name.trim(),
            input.brief.trim(),
            input.lead.trim(),
            "owner",
            config.outcome_standard,
            restless_orgintel::OutcomeStandardSource::CompanyDefault,
            None,
        )
        .await
    {
        Ok(id) => Json(serde_json::json!({ "team_id": id })).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "team", error.plain_message()),
    }
}

/// Rename a team or revise what it is accountable for.
pub(super) async fn change_team(
    State(state): State<OwnerState>,
    AxumPath((company, team)): AxumPath<(String, Uuid)>,
    Json(input): Json<TeamChangeInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let name = input.name.as_deref().map(str::trim).filter(|n| !n.is_empty());
    let brief = input.brief.as_deref().map(str::trim).filter(|b| !b.is_empty());
    match org
        .update_team(
            team,
            name,
            brief,
            "owner",
            owner_reason(&input.reason, "Changed by the owner from People"),
        )
        .await
    {
        Ok(()) => Json(serde_json::json!({ "team_id": team })).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "team", error.plain_message()),
    }
}

/// Replace a team's accountable lead.
pub(super) async fn set_team_lead(
    State(state): State<OwnerState>,
    AxumPath((company, team)): AxumPath<(String, Uuid)>,
    Json(input): Json<TeamLeadInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .set_team_lead(
            team,
            input.lead.trim(),
            "owner",
            owner_reason(&input.reason, "Lead changed by the owner from People"),
        )
        .await
    {
        Ok(()) => Json(serde_json::json!({ "team_id": team, "lead": input.lead.trim() }))
            .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "team", error.plain_message()),
    }
}

/// Disband a team: its members become unassigned and any judgement it still owed falls to
/// Exec, recorded rather than dropped.
pub(super) async fn disband_team(
    State(state): State<OwnerState>,
    AxumPath((company, team)): AxumPath<(String, Uuid)>,
    Json(input): Json<ReasonInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .disband_team(
            team,
            "owner",
            owner_reason(&input.reason, "Disbanded by the owner from People"),
        )
        .await
    {
        Ok(reassigned) => Json(serde_json::json!({
            "team_id": team,
            "reassigned_judgements": reassigned,
        }))
        .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "team", error.plain_message()),
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

/// Hold Work without retiring it; Resume undoes it. The owner, or the person who owns the
/// Work's team.
pub(super) async fn pause_work(
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
        .pause_work(work, handoff_resolver(&principal), input.reason.trim())
        .await
    {
        Ok(()) => Json(serde_json::json!({ "work_id": work, "paused": true })).into_response(),
        Err(error) => refused("work", error),
    }
}

/// Bring stopped Work back. The owner, or the person who owns the Work's team.
pub(super) async fn restore_work(
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
        .restore_work(work, handoff_resolver(&principal), input.reason.trim())
        .await
    {
        Ok(()) => Json(serde_json::json!({ "work_id": work, "restored": true })).into_response(),
        Err(error) => refused("work", error),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct FeedbackInput {
    #[serde(default)]
    feedback: String,
}

/// Send done Work back with the change wanted. The feedback reaches the next Attempt.
pub(super) async fn reopen_work(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, work)): AxumPath<(String, Uuid)>,
    Json(input): Json<FeedbackInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .reopen_work(work, handoff_resolver(&principal), input.feedback.trim())
        .await
    {
        Ok(()) => Json(serde_json::json!({ "work_id": work, "reopened": true })).into_response(),
        Err(error) => refused("work", error),
    }
}

/// An absent field leaves that part alone; a null `goal_id` takes the Work out of its Goal.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkDetailsInput {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    outcome: Option<String>,
    #[serde(default)]
    expected_artifact: Option<String>,
    #[serde(default)]
    priority: Option<i16>,
    #[serde(default, deserialize_with = "present")]
    goal_id: Option<Option<Uuid>>,
}

fn present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Option<Uuid>>, D::Error> {
    Option::<Uuid>::deserialize(deserializer).map(Some)
}

/// Correct a Work's title, brief, expected output, priority or Goal.
pub(super) async fn set_work_details(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, work)): AxumPath<(String, Uuid)>,
    Json(input): Json<WorkDetailsInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let edit = restless_orgintel::WorkDetailsEdit {
        work_id: work,
        title: input.title.as_deref(),
        outcome: input.outcome.as_deref(),
        expected_artifact: input.expected_artifact.as_deref(),
        priority: input.priority.map(|priority| priority.clamp(-1, 1)),
        goal_id: input.goal_id,
    };
    match org
        .update_work_details(handoff_resolver(&principal), edit)
        .await
    {
        Ok(()) => Json(serde_json::json!({ "work_id": work, "updated": true })).into_response(),
        Err(error) => refused("work", error),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct WorkOwnerInput {
    person: String,
    #[serde(default)]
    reason: String,
}

/// Move unsettled Work to someone else. The owner moves any; the person who owns a team moves
/// Work between its members. OrgIntel decides.
pub(super) async fn reassign_work(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, work)): AxumPath<(String, Uuid)>,
    Json(input): Json<WorkOwnerInput>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let by = handoff_resolver(&principal);
    if by == "owner" {
        // The owner is an actor OrgIntel admits as a coordinator; make sure it is recorded.
        if let Err(error) = org
            .ensure_actor("owner", "owner", "owner", "The Owner")
            .await
        {
            return refused("work", error);
        }
    }
    match org
        .reassign_work(work, input.person.trim(), by, input.reason.trim())
        .await
    {
        Ok(previous) => Json(serde_json::json!({
            "work_id": work,
            "owner_id": input.person.trim(),
            "previous_owner_id": previous,
        }))
        .into_response(),
        Err(error) => refused("work", error),
    }
}

/// The feedback linked to one Work, and which Attempt read each message. Owner only.
pub(super) async fn work_feedback(
    State(state): State<OwnerState>,
    AxumPath((company, work)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.work_feedback(work).await {
        Ok(feedback) => Json(serde_json::json!({ "feedback": feedback })).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            error.plain_message(),
        ),
    }
}

/// What each piece of Work has cost in metered model spend, from the exact request records.
/// Owner only, like the rest of spend.
pub(super) async fn work_spend(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let mut by_work: std::collections::BTreeMap<Uuid, u64> = Default::default();
    for record in state.daemon.spend.records(&company) {
        if let Some(work_id) = record.work_id {
            *by_work.entry(work_id).or_default() += record.cost_micro_usd;
        }
    }
    let work: Vec<_> = by_work
        .into_iter()
        .map(|(work_id, micro)| {
            serde_json::json!({ "work_id": work_id, "spent_usd": micro as f64 / 1_000_000.0 })
        })
        .collect();
    Json(serde_json::json!({ "work": work })).into_response()
}
