//! `POST /api/companies/<company>/email/import-sent`: the owner attaches emails sent outside
//! Restless, so replies to them reach Exec with what they answer (`restless_engine::sent_import`).
use axum::http::StatusCode;
use axum::{
    extract::{Path as AxumPath, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Deserialize;

use restless_engine::sent_import;

use super::{api_error, OwnerState, RequestPrincipal};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ImportInput {
    ids: Vec<String>,
}

pub(super) async fn import_sent(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<ImportInput>,
) -> Response {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "email",
            "Only the owner attaches emails sent outside Restless.",
        );
    }
    if crate::runtime::CompanyConfig::load(&state.daemon.root, &company).is_err() {
        return api_error(StatusCode::NOT_FOUND, "company", "Company does not exist.");
    }
    match sent_import::import(
        &state.daemon.root,
        state.daemon.authority.pool(),
        &company,
        &input.ids,
        principal.actor_id(),
    )
    .await
    {
        Ok(imported) => Json(serde_json::json!({ "imported": imported })).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "email", error.to_string()),
    }
}
