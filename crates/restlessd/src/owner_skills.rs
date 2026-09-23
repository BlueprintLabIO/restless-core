//! Owner surfaces for Sprint 55: the company skill library (Company → Skills
//! and the composer's `$` menu) and the two orchestration conventions that
//! become native Restless primitives: `/goal` (a Goal) and `/loop` (an interval
//! schedule for Exec). Nothing here grants authority.
use super::*;
use serde_json::json;
use uuid::Uuid;

fn owner_only(principal: &RequestPrincipal) -> Option<Response<Body>> {
    (principal.membership_role() != "owner").then(|| {
        api_error(
            StatusCode::FORBIDDEN,
            "skills",
            "Only the owner can change company skills, goals and loops.",
        )
    })
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

fn orgintel_error(error: restless_orgintel::OrgIntelError) -> Response<Body> {
    match error {
        restless_orgintel::OrgIntelError::InvalidSkill(message)
        | restless_orgintel::OrgIntelError::InvalidWork(message) => {
            api_error(StatusCode::BAD_REQUEST, "skills", message)
        }
        other => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            format!("{other:#}"),
        ),
    }
}

/// The library, refreshed by a live scan of the company computer when it is
/// running. A failed scan is reported honestly beside the recorded library.
pub(super) async fn list(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let scan = crate::skills::refresh_library(&org, &company).await;
    let usable = match org.actor_skills(principal.actor_id()).await {
        Ok(rows) => rows.into_iter().map(|row| row.name).collect::<Vec<_>>(),
        Err(error) => return orgintel_error(error),
    };
    let (skills, assignments) = match (org.list_skills().await, org.list_skill_assignments().await)
    {
        (Ok(skills), Ok(assignments)) => (skills, assignments),
        (Err(error), _) | (_, Err(error)) => return orgintel_error(error),
    };
    Json(json!({
        "skills": skills,
        "assignments": assignments,
        "usable": usable,
        "scan": match scan {
            Ok(()) => json!({ "state": "observed" }),
            Err(error) => json!({ "state": "unavailable", "message": format!("{error:#}") }),
        },
    }))
    .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DispositionInput {
    disposition: String,
}

pub(super) async fn set_disposition(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, skill)): AxumPath<(String, String)>,
    Json(input): Json<DispositionInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .set_skill_disposition(&skill, &input.disposition, principal.actor_id())
        .await
    {
        Ok(row) => Json(row).into_response(),
        Err(error) => orgintel_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AssignmentInput {
    scope: String,
    #[serde(default)]
    scope_id: String,
    /// `true` grants, `false` removes, `null` returns to the wider scope.
    enabled: Option<bool>,
}

pub(super) async fn assign(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, skill)): AxumPath<(String, String)>,
    Json(input): Json<AssignmentInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .assign_skill(
            &skill,
            &input.scope,
            &input.scope_id,
            input.enabled,
            principal.actor_id(),
        )
        .await
    {
        Ok(()) => Json(json!({ "skill": skill, "scope": input.scope, "enabled": input.enabled }))
            .into_response(),
        Err(error) => orgintel_error(error),
    }
}

pub(super) async fn list_goals(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.list_goals().await {
        Ok(goals) => Json(json!({ "goals": goals })).into_response(),
        Err(error) => orgintel_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GoalInput {
    objective: String,
}

/// `/goal <objective>`: a durable Goal. The composer then sends the owner's
/// words to Exec, which routes Work to one accountable lead and attaches it.
pub(super) async fn add_goal(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<GoalInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let objective = input.objective.trim();
    if objective.is_empty() || objective.chars().count() > 4_000 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "goal",
            "a goal needs an objective of at most 4,000 characters",
        );
    }
    let title = objective
        .lines()
        .next()
        .unwrap_or(objective)
        .chars()
        .take(160)
        .collect::<String>();
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.add_goal(&title, objective, principal.actor_id()).await {
        Ok(goal_id) => (
            StatusCode::CREATED,
            Json(json!({ "goal_id": goal_id, "title": title })),
        )
            .into_response(),
        Err(error) => orgintel_error(error),
    }
}

pub(super) async fn close_goal(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, goal)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.close_goal(goal, principal.actor_id()).await {
        Ok(closed) => Json(json!({ "goal_id": goal, "closed": closed })).into_response(),
        Err(error) => orgintel_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LoopInput {
    every: String,
    prompt: String,
}

/// `/loop <interval> <prompt>`: an interval schedule that wakes Exec with the
/// prompt as its reason. Repeating the same loop returns the live schedule.
pub(super) async fn add_loop(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<LoopInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let Some(interval_seconds) = restlessd::skill_package::parse_interval(&input.every) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "loop",
            "use an interval such as 30m, 2h or 1d",
        );
    };
    let prompt = input.prompt.trim();
    if prompt.is_empty() || prompt.chars().count() > 2_000 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "loop",
            "a loop needs a prompt of at most 2,000 characters",
        );
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let matching = match org.list_schedules(Some("exec"), false).await {
        Ok(rows) => rows.into_iter().find(|row| {
            row.recurrence.as_deref() == Some("interval")
                && row.interval_seconds == Some(interval_seconds)
                && row.reason == prompt
        }),
        Err(error) => return orgintel_error(error),
    };
    let (responsibility_id, version, objective, policy) = match matching {
        Some(row) => {
            let (Some(responsibility_id), Some(version)) =
                (row.responsibility_id, row.responsibility_version)
            else {
                return api_error(
                    StatusCode::CONFLICT,
                    "loop",
                    "this existing interval has no responsibility binding; cancel it before recreating it",
                );
            };
            let version_row = match org
                .get_responsibility_version(responsibility_id, version)
                .await
            {
                Ok(Some(row)) => row,
                Ok(None) => {
                    return api_error(
                        StatusCode::CONFLICT,
                        "loop",
                        "this interval's responsibility version is missing",
                    )
                }
                Err(error) => return orgintel_error(error),
            };
            (
                responsibility_id,
                version,
                version_row.objective,
                version_row.policy,
            )
        }
        None => (
            Uuid::new_v4(),
            1,
            prompt.to_string(),
            json!({ "window_seconds": 86_400 }),
        ),
    };
    match org
        .create_interval_schedule_with_responsibility(
            "exec",
            prompt,
            interval_seconds,
            chrono::Utc::now(),
            responsibility_id,
            version,
            &objective,
            policy,
        )
        .await
    {
        Ok((schedule_id, next_fire_at, created)) => (
            if created {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            },
            Json(json!({
                "schedule_id": schedule_id,
                "interval_seconds": interval_seconds,
                "next_fire_at": next_fire_at,
                "created": created,
            })),
        )
            .into_response(),
        Err(error) => orgintel_error(error),
    }
}

pub(super) async fn list_loops(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.list_schedules(Some("exec"), false).await {
        Ok(rows) => {
            let loops = rows
                .into_iter()
                .filter(|row| row.recurrence.as_deref() == Some("interval"))
                .collect::<Vec<_>>();
            Json(json!({ "loops": loops })).into_response()
        }
        Err(error) => orgintel_error(error),
    }
}

pub(super) async fn cancel_loop(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, schedule)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .cancel_schedule(schedule, "exec", "owner stopped the loop")
        .await
    {
        Ok(cancelled) => {
            Json(json!({ "schedule_id": schedule, "cancelled": cancelled })).into_response()
        }
        Err(error) => orgintel_error(error),
    }
}

/// A compact view of live recurring Exec schedules and the latest admitted
/// opportunities associated with each schedule.
pub(super) async fn schedule_monitor(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let schedules = match org.list_schedules(Some("exec"), false).await {
        Ok(rows) => rows
            .into_iter()
            .filter(|row| row.recurrence.is_some())
            .collect::<Vec<_>>(),
        Err(error) => return orgintel_error(error),
    };
    let mut monitored = Vec::with_capacity(schedules.len());
    for schedule in schedules {
        let links = match org
            .list_schedule_opportunity_occurrences(schedule.id, 3)
            .await
        {
            Ok(rows) => rows,
            Err(error) => return orgintel_error(error),
        };
        let mut recent_outcomes = Vec::with_capacity(links.len());
        for link in links {
            let Some(opportunity_id) = link.opportunity_id else {
                continue;
            };
            let opportunity = match org.get_opportunity(opportunity_id).await {
                Ok(Some(row)) => row,
                Ok(None) => continue,
                Err(error) => return orgintel_error(error),
            };
            recent_outcomes.push(json!({
                "scheduled_for": link.scheduled_for,
                "admission": link.admission,
                "opportunity_id": opportunity.id,
                "state": opportunity.state,
                "outcome": opportunity.outcome,
                "outcome_reason": opportunity.outcome_reason,
                "created_at": opportunity.created_at,
                "settled_at": opportunity.settled_at,
            }));
        }
        // Before a newly migrated recurring schedule fires, show live
        // canaries for its responsibility without mislabeling them as
        // occurrences of this particular schedule.
        let mut prior_responsibility_outcomes = Vec::new();
        if recent_outcomes.is_empty() {
            if let Some(responsibility_id) = schedule.responsibility_id {
                let prior = match org.list_opportunities(Some(responsibility_id), 3).await {
                    Ok(rows) => rows,
                    Err(error) => return orgintel_error(error),
                };
                prior_responsibility_outcomes = prior
                    .into_iter()
                    .map(|opportunity| {
                        json!({
                            "opportunity_id": opportunity.id,
                            "state": opportunity.state,
                            "outcome": opportunity.outcome,
                            "outcome_reason": opportunity.outcome_reason,
                            "created_at": opportunity.created_at,
                            "settled_at": opportunity.settled_at,
                        })
                    })
                    .collect();
            }
        }
        let testable =
            schedule.responsibility_id.is_some() && schedule.responsibility_version.is_some();
        monitored.push(json!({
            "schedule": schedule,
            "recent_outcomes": recent_outcomes,
            "prior_responsibility_outcomes": prior_responsibility_outcomes,
            "testable": testable,
        }));
    }
    Json(json!({ "schedules": monitored })).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ScheduleTestInput {
    #[serde(default)]
    timeout_seconds: Option<u64>,
}

/// Exercise one bound recurring Exec schedule through the scheduler-only disposable
/// company path. The source schedule is never fired or changed by this call.
pub(super) async fn test_schedule_trigger(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, schedule)): AxumPath<(String, Uuid)>,
    Json(input): Json<ScheduleTestInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let source = match org.list_schedules(Some("exec"), true).await {
        Ok(rows) => rows.into_iter().find(|row| row.id == schedule),
        Err(error) => return orgintel_error(error),
    };
    let Some(source) = source else {
        return api_error(
            StatusCode::NOT_FOUND,
            "schedule",
            "schedule was not found for this company",
        );
    };
    if source.recurrence.is_none()
        || source.responsibility_id.is_none()
        || source.responsibility_version.is_none()
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "schedule",
            "test trigger requires a bound recurring Exec schedule",
        );
    }
    match crate::schedule_test::run(
        &state.daemon,
        &company,
        schedule,
        input.timeout_seconds.unwrap_or(30),
    )
    .await
    {
        Ok(report) => Json(report).into_response(),
        Err(error) => api_error(
            StatusCode::BAD_REQUEST,
            "schedule_test",
            format!("{error:#}"),
        ),
    }
}
