//! What the company put online, under the owner's hand: authorise a prepared publication, mint
//! and revoke invitations to it, and stop it. Each handler calls the same Authority publication
//! manager the `restless publish` commands reach; only the Authority owner may act.

use super::*;

/// One publication's invitations, newest first, and whether it has been stopped.
pub(super) async fn show(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, publication)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Err(refusal) = require_authority_owner(&state, &company, &principal).await {
        return refusal;
    }
    let records = match state
        .daemon
        .publication
        .show(&company, Some(&publication))
        .await
    {
        Ok(records) => records,
        Err(error) => {
            return api_error(StatusCode::BAD_REQUEST, "publication", format!("{error:#}"))
        }
    };
    let bodies = |kind: &str| -> Vec<serde_json::Value> {
        records
            .get(kind)
            .and_then(serde_json::Value::as_array)
            .map(|rows| rows.iter().map(|row| row["body"].clone()).collect())
            .unwrap_or_default()
    };
    let revoked: std::collections::BTreeSet<String> = bodies("publication_invitation_revoked")
        .iter()
        .filter_map(|body| body["invitation_id"].as_str().map(str::to_string))
        .collect();
    let mut invitations: Vec<serde_json::Value> = bodies("publication_invitation")
        .into_iter()
        // The owner's own launch passes are minted per open and expire within minutes.
        .filter(|body| {
            !body["invitation_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("owner-launch-"))
        })
        .map(|body| {
            let id = body["invitation_id"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            serde_json::json!({
                "invitation_id": id,
                "subject": body["subject"],
                "expires_at": body["expires_at"],
                "revoked": revoked.contains(&id),
            })
        })
        .collect();
    invitations.reverse();
    Json(serde_json::json!({
        "publication_id": publication,
        "invitations": invitations,
        "stopped": !bodies("publication_stopped").is_empty(),
    }))
    .into_response()
}

/// Authorise the exact prepared publication: its audience, expiry, resource limits and build.
pub(super) async fn authorize(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, publication)): AxumPath<(String, String)>,
) -> Response<Body> {
    if let Err(refusal) = require_authority_owner(&state, &company, &principal).await {
        return refusal;
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    match state
        .daemon
        .publication
        .authorize(&org, &company, &publication)
        .await
    {
        Ok(_) => Json(serde_json::json!({ "publication_id": publication, "authorized": true }))
            .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "publication", format!("{error:#}")),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InviteInput {
    /// Who the invitation is for, as the publication's audience names people.
    invitee: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// Mint a scoped, expiring invitation. Its access code is returned once and never stored.
pub(super) async fn invite(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, publication)): AxumPath<(String, String)>,
    Json(input): Json<InviteInput>,
) -> Response<Body> {
    if let Err(refusal) = require_authority_owner(&state, &company, &principal).await {
        return refusal;
    }
    let invitee = input.invitee.trim();
    if invitee.is_empty() || invitee.len() > 200 {
        return api_error(
            StatusCode::BAD_REQUEST,
            "publication",
            "Name who the invitation is for.",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    let invitation = format!("invite-{}", Uuid::new_v4().simple());
    match state
        .daemon
        .publication
        .invite(
            &org,
            &company,
            &publication,
            &invitation,
            invitee,
            input.expires_at,
        )
        .await
    {
        Ok(value) => Json(serde_json::json!({
            "invitation_id": invitation,
            "subject": invitee,
            "expires_at": input.expires_at,
            "token": value["token"],
        }))
        .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "publication", format!("{error:#}")),
    }
}

/// Revoke one invitation at once.
pub(super) async fn revoke(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, _publication, invitation)): AxumPath<(String, String, String)>,
) -> Response<Body> {
    if let Err(refusal) = require_authority_owner(&state, &company, &principal).await {
        return refusal;
    }
    match state
        .daemon
        .publication
        .revoke_invitation(&company, &invitation)
        .await
    {
        Ok(_) => Json(serde_json::json!({ "invitation_id": invitation, "revoked": true }))
            .into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "publication", format!("{error:#}")),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StopInput {
    #[serde(default)]
    reason: String,
}

/// Stop publishing, or decline a prepared publication: the provider stops and its cleanup is
/// recorded.
pub(super) async fn stop(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, publication)): AxumPath<(String, String)>,
    Json(input): Json<StopInput>,
) -> Response<Body> {
    if let Err(refusal) = require_authority_owner(&state, &company, &principal).await {
        return refusal;
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    let reason = match input.reason.trim() {
        "" => "Stopped by the owner",
        reason => reason,
    };
    match state
        .daemon
        .publication
        .stop(&org, &company, &publication, reason)
        .await
    {
        Ok(value) => Json(value).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "publication", format!("{error:#}")),
    }
}
