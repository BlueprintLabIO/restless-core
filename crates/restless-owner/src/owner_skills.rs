//! Owner surfaces for Sprint 55: the company skill library (Company → Skills
//! and the composer's `$` menu) and the two orchestration conventions that
//! become native Restless primitives: `/goal` (a Goal) and `/loop` (an interval
//! schedule for Exec). Nothing here grants authority.
use super::*;
use serde_json::json;
use uuid::Uuid;

/// An admin runs the company's schedules with the owner.
fn owner_or_admin(principal: &RequestPrincipal) -> Option<Response<Body>> {
    (!matches!(principal.membership_role(), "owner" | "admin")).then(|| {
        api_error(
            StatusCode::FORBIDDEN,
            "schedules",
            "Only the owner or an admin can change schedules.",
        )
    })
}

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
    let usage = match org.skill_work_usage().await {
        Ok(usage) => usage,
        Err(error) => return orgintel_error(error),
    };
    let availability = match org.skill_agent_availability().await {
        Ok(rows) => rows,
        Err(error) => return orgintel_error(error),
    };
    // Work/session activity is owner context; library readership does not expand that access.
    let applications = if principal.membership_role() == "owner" {
        match org.skill_agent_applications().await {
            Ok(rows) => Some(rows),
            Err(error) => return orgintel_error(error),
        }
    } else {
        None
    };
    Json(json!({
        "availability": availability,
        "applications": applications,
        "usage": usage,
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

/// A skill's package as agents would follow it: `SKILL.md` and the files beside it, read live
/// from the company computer, so the owner reviews the real instructions and scripts.
pub(super) async fn package(
    State(state): State<OwnerState>,
    AxumPath((company, skill)): AxumPath<(String, String)>,
) -> Response<Body> {
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let row = match org.skill(&skill).await {
        Ok(Some(row)) => row,
        Ok(None) => {
            return api_error(
                StatusCode::NOT_FOUND,
                "skills",
                "No company skill has that name.",
            )
        }
        Err(error) => return orgintel_error(error),
    };
    match crate::skills::read_package(&company, &row.path).await {
        Ok(package) => Json(json!({
            "skill": row,
            "package": package,
            "changed": package.digest != row.digest,
        }))
        .into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "skills",
            format!("{error:#}"),
        ),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NewSkillInput {
    skill_md: String,
}

/// The owner writes new know-how. It lands as a candidate, which the owner reviews and accepts
/// like any other, so every skill the company follows passed the same review.
pub(super) async fn create(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<NewSkillInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let Some(name) = restless_contracts::skill_package::parse_frontmatter(&input.skill_md)
        .map(|frontmatter| frontmatter.name)
        .filter(|name| restless_orgintel::valid_skill_name(name))
    else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "skills",
            "Give the know-how a name of lowercase letters, digits and hyphens.",
        );
    };
    match org.skill(&name).await {
        Ok(None) => {}
        Ok(Some(_)) => {
            return api_error(
                StatusCode::CONFLICT,
                "skills",
                "Know-how with that name already exists. Choose another name.",
            )
        }
        Err(error) => return orgintel_error(error),
    }
    let dir = format!(
        "{}/{name}",
        restless_contracts::skill_package::CANDIDATE_ROOT
    );
    // The know-how is written to the company computer's skill folder. Writing it is real demand,
    // so a computer that slept wakes; one the owner stopped says so instead of a Docker error.
    if !restless_engine::runtime_sleep::wake_for(&state.daemon, &company, "someone wrote know-how")
        .await
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            "The company computer is stopped, so know-how cannot be saved. Start it from Company → Computer.",
        );
    }
    let observed = match crate::skills::write_skill_md(&company, &dir, &name, &input.skill_md).await
    {
        Ok(observed) => observed,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, "skills", format!("{error:#}")),
    };
    match org
        .register_skill_candidate(&observed, principal.actor_id(), None, None)
        .await
    {
        Ok(row) => Json(row).into_response(),
        Err(error) => orgintel_error(error),
    }
}

/// The Apps catalogue from public registries (Sprint 87): the same list for every company on the
/// plane, refreshed daily.
#[derive(Deserialize, Default)]
pub(super) struct CatalogueQuery {
    #[serde(default)]
    refresh: bool,
}

pub(super) async fn app_catalogue(Query(query): Query<CatalogueQuery>) -> Response<Body> {
    Json(restless_engine::app_catalogue::current_with_refresh(query.refresh).await).into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogueInstallInput {
    key: String,
}

/// Public source preview. It reads only a canonical listed SKILL.md; no company
/// files, install, wake, assignment or account access is involved.
pub(super) async fn catalogue_instructions(
    Query(input): Query<CatalogueInstallInput>,
) -> Response<Body> {
    let item = match restless_engine::app_catalogue::item(&input.key).await {
        Ok(Some(item)) if item.kind == "skill" => item,
        Ok(_) => {
            return api_error(
                StatusCode::NOT_FOUND,
                "catalogue",
                "This know-how is not in the catalogue.",
            )
        }
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "catalogue",
                error.to_string(),
            )
        }
    };
    match restless_engine::app_catalogue::skill_instructions(&item).await {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(skill_md) => Json(json!({ "skill_md": skill_md })).into_response(),
            Err(_) => api_error(
                StatusCode::BAD_GATEWAY,
                "catalogue",
                "The publisher's instructions are not readable text.",
            ),
        },
        Err(error) => api_error(
            StatusCode::BAD_GATEWAY,
            "catalogue",
            format!("The publisher's instructions could not be read: {error:#}"),
        ),
    }
}

/// Install a catalogue skill as candidate know-how: its files are fetched from its repository and
/// written into the company computer's candidate folder, then registered like know-how the owner
/// wrote. The web accepts one without scripts at once; one with scripts goes through the same
/// review as any candidate. A catalogue service is added by its address instead.
pub(super) async fn install_from_catalogue(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<CatalogueInstallInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let item = match restless_engine::app_catalogue::item(&input.key).await {
        Ok(Some(item)) => item,
        Ok(None) => {
            return api_error(
                StatusCode::NOT_FOUND,
                "catalogue",
                "That app is no longer in the catalogue.",
            )
        }
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "catalogue",
                error.to_string(),
            )
        }
    };
    if item.kind != "skill" {
        return api_error(
            StatusCode::BAD_REQUEST,
            "catalogue",
            "Add a service by its address.",
        );
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let skill_md = match restless_engine::app_catalogue::skill_file(&item, "SKILL.md").await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => {
            return api_error(
                StatusCode::BAD_GATEWAY,
                "catalogue",
                format!("The skill could not be fetched: {error:#}"),
            )
        }
    };
    let Some(name) = restless_contracts::skill_package::parse_frontmatter(&skill_md)
        .map(|frontmatter| frontmatter.name)
        .filter(|name| restless_orgintel::valid_skill_name(name))
    else {
        return api_error(
            StatusCode::BAD_GATEWAY,
            "catalogue",
            "The skill's SKILL.md has no usable name.",
        );
    };
    match org.skill(&name).await {
        Ok(None) => {}
        Ok(Some(_)) => {
            return api_error(
                StatusCode::CONFLICT,
                "catalogue",
                "The company already has know-how with this name.",
            )
        }
        Err(error) => return orgintel_error(error),
    }
    if !restless_engine::runtime_sleep::wake_for(&state.daemon, &company, "installing know-how")
        .await
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            "The company computer is stopped, so know-how cannot be installed. Start it from Company → Computer.",
        );
    }
    let dir = format!(
        "{}/{name}",
        restless_contracts::skill_package::CANDIDATE_ROOT
    );
    for file in item
        .files
        .iter()
        .filter(|file| file.as_str() != "SKILL.md")
        .take(60)
    {
        let bytes = match restless_engine::app_catalogue::skill_file(&item, file).await {
            Ok(bytes) => bytes,
            Err(error) => {
                return api_error(
                    StatusCode::BAD_GATEWAY,
                    "catalogue",
                    format!("{file} could not be fetched: {error:#}"),
                )
            }
        };
        if let Err(error) = crate::skills::write_skill_file(&company, &dir, file, &bytes).await {
            return api_error(StatusCode::BAD_REQUEST, "catalogue", format!("{error:#}"));
        }
    }
    let observed = match crate::skills::write_skill_md(&company, &dir, &name, &skill_md).await {
        Ok(observed) => observed,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, "catalogue", format!("{error:#}")),
    };
    match org
        .register_skill_candidate(&observed, principal.actor_id(), None, None)
        .await
    {
        Ok(row) => Json(serde_json::json!({ "skill": row, "runs_scripts": item.runs_scripts }))
            .into_response(),
        Err(error) => orgintel_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EditSkillInput {
    skill_md: String,
    /// The digest the owner edited from; a newer version is not overwritten.
    expected_digest: String,
}

/// The owner corrects a company or candidate skill in place. The new digest is recorded at
/// once; Work that pinned the earlier version keeps it and is told the library changed.
pub(super) async fn edit(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, skill)): AxumPath<(String, String)>,
    Json(input): Json<EditSkillInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let row = match org.skill(&skill).await {
        Ok(Some(row)) => row,
        Ok(None) => {
            return api_error(
                StatusCode::NOT_FOUND,
                "skills",
                "No company skill has that name.",
            )
        }
        Err(error) => return orgintel_error(error),
    };
    match crate::skills::read_package(&company, &row.path).await {
        Ok(current) if current.digest != input.expected_digest => {
            return api_error(
                StatusCode::CONFLICT,
                "skills",
                "This know-how changed since you opened it. Reopen it and edit again.",
            )
        }
        Ok(_) => {}
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "skills",
                format!("{error:#}"),
            )
        }
    }
    let observed = match crate::skills::write_skill_md(
        &company,
        &row.path,
        &row.name,
        &input.skill_md,
    )
    .await
    {
        Ok(observed) => observed,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, "skills", format!("{error:#}")),
    };
    if let Err(error) = org.observe_skills(std::slice::from_ref(&observed)).await {
        return orgintel_error(error);
    }
    match org.skill(&skill).await {
        Ok(Some(row)) => Json(row).into_response(),
        Ok(None) => api_error(
            StatusCode::NOT_FOUND,
            "skills",
            "No company skill has that name.",
        ),
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

/// An absent field leaves that part alone; a null `due_on` clears the date.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GoalDetailsInput {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    done_when: Option<String>,
    #[serde(default, deserialize_with = "present_date")]
    due_on: Option<Option<chrono::NaiveDate>>,
}

fn present_date<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Option<chrono::NaiveDate>>, D::Error> {
    Option::<chrono::NaiveDate>::deserialize(deserializer).map(Some)
}

/// Rename an open Goal, or say what done looks like and by when.
pub(super) async fn set_goal_details(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, goal)): AxumPath<(String, Uuid)>,
    Json(input): Json<GoalDetailsInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_only(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .set_goal_details(
            goal,
            input.title.as_deref(),
            input.done_when.as_deref(),
            input.due_on,
            principal.actor_id(),
        )
        .await
    {
        Ok(true) => Json(json!({ "goal_id": goal, "updated": true })).into_response(),
        Ok(false) => api_error(
            StatusCode::NOT_FOUND,
            "goal",
            "that Goal is closed or does not exist; reopen it first",
        ),
        Err(error) => orgintel_error(error),
    }
}

/// Open a closed Goal again.
pub(super) async fn reopen_goal(
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
    match org.reopen_goal(goal, principal.actor_id()).await {
        Ok(reopened) => Json(json!({ "goal_id": goal, "reopened": reopened })).into_response(),
        Err(error) => orgintel_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LoopInput {
    /// An interval such as `30m`; or a weekday time with `at_local` and `timezone`.
    every: Option<String>,
    /// `HH:MM` on weekdays, in `timezone` (IANA, e.g. `Europe/London`).
    at_local: Option<String>,
    timezone: Option<String>,
    /// The days a time of day runs on: `weekdays` (the default), `daily`, `weekly:1,3` (ISO
    /// weekdays) or `monthly:15`.
    #[serde(default)]
    days: Option<String>,
    prompt: String,
}

/// The owner's day choice as OrgIntel stores it: Monday to Friday is the absent value.
fn calendar_days(days: Option<&str>) -> std::result::Result<Option<String>, Response<Body>> {
    let days = days.map(str::trim).filter(|days| *days != "weekdays");
    restless_orgintel::validate_calendar_days(days).map_err(|_| {
        api_error(
            StatusCode::BAD_REQUEST,
            "schedule",
            "Choose weekdays, every day, one or more days of the week, or a day of the month.",
        )
    })?;
    Ok(days.map(str::to_string))
}

/// A missed weekday run still happens when the plane comes back within two
/// hours (a laptop opened at 9:20 for a 9:00 check-in); a later one is skipped.
const WEEKDAY_MISSED_POLICY: &str = "skip_if_late";
const WEEKDAY_CATCH_UP_SECONDS: i64 = 2 * 60 * 60;

/// The responsibility a repeated schedule already carries, or a new one for
/// the prompt. Repeating the same schedule keeps its binding.
async fn loop_binding(
    org: &restless_orgintel::OrgIntel,
    matching: Option<restless_orgintel::ScheduleRow>,
    prompt: &str,
) -> std::result::Result<(Uuid, i32, String, serde_json::Value), Response<Body>> {
    let Some(row) = matching else {
        return Ok((
            Uuid::new_v4(),
            1,
            prompt.to_string(),
            json!({ "window_seconds": 86_400 }),
        ));
    };
    let (Some(responsibility_id), Some(version)) =
        (row.responsibility_id, row.responsibility_version)
    else {
        return Err(api_error(
            StatusCode::CONFLICT,
            "loop",
            "this existing schedule has no responsibility binding; cancel it before recreating it",
        ));
    };
    match org
        .get_responsibility_version(responsibility_id, version)
        .await
    {
        Ok(Some(version_row)) => Ok((
            responsibility_id,
            version,
            version_row.objective,
            version_row.policy,
        )),
        Ok(None) => Err(api_error(
            StatusCode::CONFLICT,
            "loop",
            "this schedule's responsibility version is missing",
        )),
        Err(error) => Err(orgintel_error(error)),
    }
}

/// A time of day the owner typed, as `HH:MM` (a browser time field may add `:SS`).
fn parse_local_time(value: &str) -> Option<chrono::NaiveTime> {
    let value = value.trim();
    chrono::NaiveTime::parse_from_str(value, "%H:%M")
        .or_else(|_| chrono::NaiveTime::parse_from_str(value, "%H:%M:%S"))
        .ok()
}

/// The shape of an IANA zone name. OrgIntel resolves the zone itself and
/// refuses one it does not know before anything is written.
fn plausible_timezone(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'+'))
}

/// `/loop <interval> <prompt>`: an interval schedule that wakes Exec with the
/// prompt as its reason. Repeating the same loop returns the live schedule.
/// With `at_local` and `timezone` instead of `every`, Exec is woken on
/// weekdays at that local time.
pub(super) async fn add_loop(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<LoopInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_or_admin(&principal) {
        return refusal;
    }
    let prompt = input.prompt.trim();
    if prompt.is_empty() || prompt.chars().count() > 2_000 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "loop",
            "a loop needs a prompt of at most 2,000 characters",
        );
    }
    let every = match (&input.every, &input.at_local, &input.timezone) {
        (Some(_), None, None) if input.days.is_some() => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "loop",
                "days go with a time of day, not an interval",
            )
        }
        (Some(every), None, None) => every,
        (None, Some(at_local), Some(timezone)) => {
            let Some(local_time) = parse_local_time(at_local) else {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "loop",
                    "use a time of day such as 09:00",
                );
            };
            let timezone = timezone.trim();
            if !plausible_timezone(timezone) {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "loop",
                    "use an IANA time zone such as Europe/London",
                );
            }
            let org = match org_for(&state, &company).await {
                Ok(org) => org,
                Err(response) => return response,
            };
            let days = match calendar_days(input.days.as_deref()) {
                Ok(days) => days,
                Err(response) => return response,
            };
            return add_weekday_loop(&org, prompt, local_time, timezone, days.as_deref()).await;
        }
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "loop",
                "choose either an interval, or a weekday time with its time zone",
            )
        }
    };
    let Some(interval_seconds) = restless_contracts::skill_package::parse_interval(every) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "loop",
            "use an interval such as 30m, 2h or 1d",
        );
    };
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
    let (responsibility_id, version, objective, policy) =
        match loop_binding(&org, matching, prompt).await {
            Ok(binding) => binding,
            Err(response) => return response,
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

/// The weekday cadence the CLI's `schedule create-responsibility --weekdays`
/// creates, for Exec. The plane's own timer claims it (`local_mac` is what
/// that timer claims on every plane; nothing claims `always_on` yet).
async fn add_weekday_loop(
    org: &restless_orgintel::OrgIntel,
    prompt: &str,
    local_time: chrono::NaiveTime,
    timezone: &str,
    days: Option<&str>,
) -> Response<Body> {
    let matching = match org.list_schedules(Some("exec"), false).await {
        Ok(rows) => rows.into_iter().find(|row| {
            row.recurrence.as_deref() == Some("weekdays")
                && row.local_time == Some(local_time)
                && row.timezone.as_deref() == Some(timezone)
                && row.calendar_days.as_deref() == days
                && row.reason == prompt
        }),
        Err(error) => return orgintel_error(error),
    };
    let (responsibility_id, version, objective, policy) =
        match loop_binding(org, matching, prompt).await {
            Ok(binding) => binding,
            Err(response) => return response,
        };
    match org
        .create_calendar_schedule_with_responsibility(
            "exec",
            prompt,
            days,
            local_time,
            timezone,
            chrono::Utc::now(),
            WEEKDAY_MISSED_POLICY,
            Some(WEEKDAY_CATCH_UP_SECONDS),
            "local_mac",
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
                "local_time": local_time.format("%H:%M").to_string(),
                "timezone": timezone,
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
    if let Some(refusal) = owner_or_admin(&principal) {
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ScheduleUpdateInput {
    expected_fire_at: chrono::DateTime<chrono::Utc>,
    expected_paused: bool,
    paused: bool,
    every: Option<String>,
    /// A new `HH:MM` for a time-of-day schedule; its zone and days are kept unless given.
    at_local: Option<String>,
    /// A new IANA time zone for a time-of-day schedule.
    #[serde(default)]
    timezone: Option<String>,
    /// New days for a time-of-day schedule, as `LoopInput::days`.
    #[serde(default)]
    days: Option<String>,
    /// New words for what the schedule asks Exec to do.
    #[serde(default)]
    prompt: Option<String>,
}

pub(super) async fn update_schedule(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, schedule)): AxumPath<(String, Uuid)>,
    Json(input): Json<ScheduleUpdateInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_or_admin(&principal) {
        return refusal;
    }
    let calendar = input.at_local.is_some() || input.timezone.is_some() || input.days.is_some();
    if input.every.is_some() && calendar {
        return api_error(
            StatusCode::BAD_REQUEST,
            "schedule",
            "Change either the interval or the time of day, not both.",
        );
    }
    let interval = match input.every.as_deref() {
        Some(value) => match restless_contracts::skill_package::parse_interval(value) {
            Some(seconds) => Some(seconds),
            None => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "schedule",
                    "Choose a cadence such as 30m, 2h or 1d.",
                )
            }
        },
        None => None,
    };
    let local_time = match input.at_local.as_deref() {
        Some(value) => match parse_local_time(value) {
            Some(time) => Some(time),
            None => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "schedule",
                    "Choose a time of day such as 09:00.",
                )
            }
        },
        None => None,
    };
    let timezone = match input.timezone.as_deref().map(str::trim) {
        Some(zone) if !plausible_timezone(zone) => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "schedule",
                "Choose a time zone such as Europe/London.",
            )
        }
        zone => zone.map(str::to_string),
    };
    let days = match input.days.as_deref() {
        Some(days) => match calendar_days(Some(days)) {
            Ok(days) => Some(days),
            Err(response) => return response,
        },
        None => None,
    };
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org
        .change_recurring_schedule(
            schedule,
            input.expected_fire_at,
            input.expected_paused,
            restless_orgintel::ScheduleChange {
                paused: input.paused,
                interval_seconds: interval,
                local_time,
                timezone,
                calendar_days: days,
                reason: input.prompt,
            },
        )
        .await
    {
        Ok(Some(row)) => Json(json!({"schedule": row})).into_response(),
        Ok(None) => api_error(
            StatusCode::CONFLICT,
            "schedule",
            "This schedule changed. Refresh and try again.",
        ),
        Err(error) => orgintel_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ScheduleRunInput {
    expected_fire_at: chrono::DateTime<chrono::Utc>,
}

/// Run a schedule now: the scheduler claims it at once, as if its time had come, and Exec is
/// woken with what it asks. The next run is set from now.
pub(super) async fn run_schedule_now(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, schedule)): AxumPath<(String, Uuid)>,
    Json(input): Json<ScheduleRunInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_or_admin(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.run_schedule_now(schedule, input.expected_fire_at).await {
        Ok(Some(row)) => {
            state.daemon.schedule_wake.notify_one();
            Json(json!({"schedule": row})).into_response()
        }
        Ok(None) => api_error(
            StatusCode::CONFLICT,
            "schedule",
            "This schedule changed or is paused. Refresh and try again.",
        ),
        Err(error) => orgintel_error(error),
    }
}

/// Every run a schedule fired or skipped, newest first, with any recovery of a skipped one.
pub(super) async fn schedule_history(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, schedule)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    if let Some(refusal) = owner_or_admin(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.list_schedule_occurrences(schedule, 20).await {
        Ok(rows) => Json(json!({ "occurrences": rows })).into_response(),
        Err(error) => orgintel_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ScheduleRecoverInput {
    scheduled_for: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    reason: String,
}

/// Recover one skipped run with a single wake; asking twice is a read, not a second wake.
pub(super) async fn recover_schedule_run(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, schedule)): AxumPath<(String, Uuid)>,
    Json(input): Json<ScheduleRecoverInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_or_admin(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let reason = match input.reason.trim() {
        "" => "Recovered a skipped run from Schedules",
        reason => reason,
    };
    match org
        .recover_skipped_schedule(
            schedule,
            input.scheduled_for,
            "exec",
            principal.actor_id(),
            reason,
        )
        .await
    {
        Ok(recovery) => {
            state.daemon.schedule_wake.notify_one();
            Json(json!({ "recovery": recovery })).into_response()
        }
        Err(error) => orgintel_error(error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SchedulePolicyInput {
    /// `skip`, `skip_if_late`, `catch_up_once` or `coalesce_latest`.
    missed_policy: String,
    /// How late a missed run may still start; required for every policy but `skip`.
    #[serde(default)]
    catch_up_grace_seconds: Option<i64>,
}

/// What happens when a run is missed: skip it, run it if not too late, catch up once, or run
/// only the latest of several.
pub(super) async fn set_schedule_policy(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, schedule)): AxumPath<(String, Uuid)>,
    Json(input): Json<SchedulePolicyInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_or_admin(&principal) {
        return refusal;
    }
    let org = match org_for(&state, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let grace = (input.missed_policy != "skip")
        .then_some(input.catch_up_grace_seconds)
        .flatten();
    match org
        .set_schedule_missed_policy(schedule, "exec", &input.missed_policy, grace)
        .await
    {
        Ok(true) => Json(json!({
            "schedule_id": schedule,
            "missed_policy": input.missed_policy,
            "catch_up_grace_seconds": grace,
        }))
        .into_response(),
        Ok(false) => api_error(
            StatusCode::CONFLICT,
            "schedule",
            "This schedule is no longer running. Refresh and try again.",
        ),
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
            .list_schedule_opportunity_occurrences(schedule.id, 10)
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
        // A run missed and skipped, and not recovered since, is the owner's to notice too.
        let last_skipped = match org.list_schedule_occurrences(schedule.id, 1).await {
            Ok(rows) => rows
                .first()
                .filter(|row| row.disposition == "skipped" && row.recovered_at.is_none())
                .map(|row| row.scheduled_for),
            Err(error) => return orgintel_error(error),
        };
        monitored.push(json!({
            "schedule": schedule,
            "last_skipped": last_skipped,
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
    #[serde(default)]
    actor: bool,
}

/// Exercise one bound recurring Exec schedule in a disposable company.
/// The live source schedule is never fired or changed here.
pub(super) async fn test_schedule_trigger(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, schedule)): AxumPath<(String, Uuid)>,
    Json(input): Json<ScheduleTestInput>,
) -> Response<Body> {
    if let Some(refusal) = owner_or_admin(&principal) {
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
    let timeout = input
        .timeout_seconds
        .unwrap_or(if input.actor { 600 } else { 30 });
    let result = if input.actor {
        crate::schedule_test::run_actor(&state.daemon, &company, schedule, timeout).await
    } else {
        crate::schedule_test::run(&state.daemon, &company, schedule, timeout).await
    };
    match result {
        Ok(report) => Json(report).into_response(),
        Err(error) => api_error(
            StatusCode::BAD_REQUEST,
            "schedule_test",
            format!("{error:#}"),
        ),
    }
}
