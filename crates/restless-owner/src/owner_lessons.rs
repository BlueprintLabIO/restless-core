//! Company lessons, as the owner sees them: what they have ruled, how often each problem came
//! back, and retiring a rule that no longer stands. Owner only: the membership boundary refuses
//! everyone else before these run, and each handler checks again.

use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct RetireLessonInput {
    #[serde(default)]
    reason: String,
}

async fn owner_org(
    state: &OwnerState,
    principal: &RequestPrincipal,
    company: &str,
) -> std::result::Result<restless_orgintel::OrgIntel, Response<Body>> {
    if principal.membership_role() != "owner" {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "lessons",
            "Lessons are the owner's.",
        ));
    }
    state.daemon.orgintel.get(company).await.map_err(|error| {
        api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            logged(error, "The company is temporarily unavailable. Try again."),
        )
    })
}

fn changed(result: restless_orgintel::Result<restless_orgintel::LessonRow>) -> Response<Body> {
    match result {
        Ok(lesson) => Json(lesson).into_response(),
        Err(error @ restless_orgintel::OrgIntelError::Db(_)) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "lessons",
            logged(error, "That was not saved. Try again."),
        ),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "lessons", error.plain_message()),
    }
}

/// `GET /companies/{company}/lessons`: active lessons, newest first.
pub(super) async fn list(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let org = match owner_org(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.list_lessons(false).await {
        Ok(items) => Json(serde_json::json!({ "items": items })).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "lessons",
            logged(error, "Lessons could not be read. Try again."),
        ),
    }
}

/// `POST /companies/{company}/lessons/{lesson}/retire`
pub(super) async fn retire(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, lesson)): AxumPath<(String, Uuid)>,
    Json(input): Json<RetireLessonInput>,
) -> Response<Body> {
    let org = match owner_org(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    let reason = match input.reason.trim() {
        "" => "Retired by the owner",
        reason => reason,
    };
    changed(org.retire_lesson(lesson, "owner", reason).await)
}

/// `POST /companies/{company}/lessons/{lesson}/restore`: the Undo for a retire.
pub(super) async fn restore(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, lesson)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let org = match owner_org(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    changed(org.restore_lesson(lesson, "owner").await)
}
