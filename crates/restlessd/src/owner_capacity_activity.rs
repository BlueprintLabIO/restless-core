//! Fleet's bounded, read-only view of Runtime activity.
//!
//! The account plane stays up when a company Runtime sleeps. Fleet may ask
//! this endpoint whether that exact cell is either busy or owed durable work;
//! it never receives Work text, Message text, file paths, or credentials.

use std::sync::Arc;

use anyhow::{Context as _, Result};
use axum::{
    extract::{FromRef, Path as AxumPath, State},
    http::{
        header::{AUTHORIZATION, HOST},
        HeaderMap, StatusCode,
    },
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::OwnerState;
use crate::{entry::EntryMode, Daemon};

pub(super) const CAPACITY_ACTIVITY_PATH_PREFIX: &str = "/v1/cells/";
const CONTRACT_VERSION: u32 = 1;
const TOKEN_ENV: &str = "RESTLESS_ACTIVITY_TOKEN";

#[derive(Clone)]
pub(super) enum CapacityActivityService {
    Disabled,
    Enabled {
        deployment: Arc<CapacityActivityDeployment>,
        source: CapacityActivitySource,
    },
}

#[derive(Clone)]
pub(super) struct CapacityActivityDeployment {
    owner_id: Uuid,
    plane_hostname: String,
    token: Vec<u8>,
}

#[derive(Clone)]
pub(super) enum CapacityActivitySource {
    Daemon(Arc<Daemon>),
    #[cfg(test)]
    Fixed {
        company_id: Uuid,
        cell_id: Uuid,
        runtime_id: String,
        desired_revision: i64,
        protected_kinds: Vec<&'static str>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapacityActivityRequest {
    contract_version: u32,
    owner_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    runtime_id: String,
    desired_revision: i64,
}

#[derive(Debug, Serialize)]
struct CapacityActivityObservation {
    contract_version: u32,
    owner_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    runtime_id: String,
    desired_revision: i64,
    protected_activity: bool,
    protected_kinds: Vec<&'static str>,
    observed_at: chrono::DateTime<Utc>,
    valid_until: chrono::DateTime<Utc>,
}

impl CapacityActivityService {
    pub(super) fn from_environment(daemon: &Arc<Daemon>, entry: &EntryMode) -> Result<Self> {
        let Some((owner_id, _, plane_hostname)) = entry.network_coordinates() else {
            return Ok(Self::Disabled);
        };
        let token = std::env::var(TOKEN_ENV)
            .with_context(|| format!("network entry mode requires {TOKEN_ENV}"))?;
        if !(32..=512).contains(&token.len())
            || token.trim() != token
            || token.bytes().any(|byte| byte.is_ascii_control())
        {
            anyhow::bail!("{TOKEN_ENV} must be one bounded bearer value");
        }
        Ok(Self::Enabled {
            deployment: Arc::new(CapacityActivityDeployment {
                owner_id,
                plane_hostname: plane_hostname.to_string(),
                token: token.into_bytes(),
            }),
            source: CapacityActivitySource::Daemon(daemon.clone()),
        })
    }

    fn deployment(&self) -> Option<&CapacityActivityDeployment> {
        match self {
            Self::Disabled => None,
            Self::Enabled { deployment, .. } => Some(deployment),
        }
    }

    fn source(&self) -> Option<&CapacityActivitySource> {
        match self {
            Self::Disabled => None,
            Self::Enabled { source, .. } => Some(source),
        }
    }
}

impl FromRef<OwnerState> for CapacityActivityService {
    fn from_ref(state: &OwnerState) -> Self {
        state.capacity_activity.clone()
    }
}

pub(super) fn routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    CapacityActivityService: FromRef<S>,
{
    Router::new().route(
        "/v1/cells/{cell_id}/capacity-activity",
        post(observe_capacity_activity),
    )
}

pub(super) fn is_capacity_activity_path(path: &str) -> bool {
    path.strip_prefix(CAPACITY_ACTIVITY_PATH_PREFIX)
        .and_then(|rest| rest.strip_suffix("/capacity-activity"))
        .is_some_and(|cell| cell.parse::<Uuid>().is_ok())
}

async fn observe_capacity_activity(
    State(service): State<CapacityActivityService>,
    AxumPath(path_cell_id): AxumPath<Uuid>,
    headers: HeaderMap,
    Json(request): Json<CapacityActivityRequest>,
) -> Response {
    let Some(deployment) = service.deployment() else {
        return refusal(StatusCode::NOT_FOUND, "capacity_activity_unavailable");
    };
    if !authorized(&headers, deployment) {
        return refusal(StatusCode::UNAUTHORIZED, "capacity_activity_unauthorized");
    }
    if request.contract_version != CONTRACT_VERSION
        || request.owner_id.is_nil()
        || request.company_id.is_nil()
        || request.cell_id.is_nil()
        || request.cell_id != path_cell_id
        || request.owner_id != deployment.owner_id
        || request.desired_revision < 1
        || request.runtime_id != format!("restless-cell-{path_cell_id}")
    {
        return refusal(StatusCode::CONFLICT, "capacity_activity_identity_mismatch");
    }

    let Some(source) = service.source() else {
        return refusal(StatusCode::NOT_FOUND, "capacity_activity_unavailable");
    };
    let protected_kinds = match source.observe(&request).await {
        Ok(kinds) => kinds,
        Err(CapacityActivityFailure::Identity) => {
            return refusal(StatusCode::CONFLICT, "capacity_activity_identity_mismatch");
        }
        Err(CapacityActivityFailure::Revision) => {
            return refusal(StatusCode::CONFLICT, "capacity_activity_revision_mismatch");
        }
        Err(CapacityActivityFailure::Unavailable(error)) => {
            tracing::warn!(%path_cell_id, %error, "could not observe company activity");
            return refusal(
                StatusCode::SERVICE_UNAVAILABLE,
                "capacity_activity_unavailable",
            );
        }
    };

    let observed_at = Utc::now();
    Json(CapacityActivityObservation {
        contract_version: CONTRACT_VERSION,
        owner_id: request.owner_id,
        company_id: request.company_id,
        cell_id: request.cell_id,
        runtime_id: request.runtime_id,
        desired_revision: request.desired_revision,
        protected_activity: !protected_kinds.is_empty(),
        protected_kinds,
        observed_at,
        valid_until: observed_at + Duration::seconds(20),
    })
    .into_response()
}

enum CapacityActivityFailure {
    Identity,
    Revision,
    Unavailable(anyhow::Error),
}

impl CapacityActivitySource {
    async fn observe(
        &self,
        request: &CapacityActivityRequest,
    ) -> std::result::Result<Vec<&'static str>, CapacityActivityFailure> {
        match self {
            Self::Daemon(daemon) => observe_daemon(daemon, request).await,
            #[cfg(test)]
            Self::Fixed {
                company_id,
                cell_id,
                runtime_id,
                desired_revision,
                protected_kinds,
            } => {
                if request.company_id != *company_id
                    || request.cell_id != *cell_id
                    || request.runtime_id != *runtime_id
                {
                    return Err(CapacityActivityFailure::Identity);
                }
                if request.desired_revision != *desired_revision {
                    return Err(CapacityActivityFailure::Revision);
                }
                Ok(protected_kinds.clone())
            }
        }
    }
}

async fn observe_daemon(
    daemon: &Daemon,
    request: &CapacityActivityRequest,
) -> std::result::Result<Vec<&'static str>, CapacityActivityFailure> {
    let company = company_for_cell(daemon, request.company_id, request.cell_id)
        .await
        .map_err(CapacityActivityFailure::Unavailable)?
        .ok_or(CapacityActivityFailure::Identity)?;
    let org = daemon
        .orgintel
        .get(&company)
        .await
        .map_err(CapacityActivityFailure::Unavailable)?;
    match org
        .admit_runtime_activity_revision(
            restless_orgintel::CompanyAccessIdentity {
                company_id: request.company_id,
                cell_id: request.cell_id,
            },
            &request.runtime_id,
            request.desired_revision,
        )
        .await
        .map_err(|error| CapacityActivityFailure::Unavailable(error.into()))?
    {
        true => {}
        false => return Err(CapacityActivityFailure::Revision),
    }

    let mut protected_kinds = Vec::with_capacity(4);
    let in_flight = daemon
        .in_flight
        .lock()
        .map_err(|_| {
            CapacityActivityFailure::Unavailable(anyhow::anyhow!(
                "in-flight activity lock poisoned"
            ))
        })?
        .is_active(&company);
    if in_flight || !daemon.staff.running_actors(&company).is_empty() {
        protected_kinds.push("attempt");
    }
    if !org
        .actors_owing_message_mentions(1)
        .await
        .map_err(|error| CapacityActivityFailure::Unavailable(error.into()))?
        .is_empty()
        || !org
            .actors_owing_document_mentions()
            .await
            .map_err(|error| CapacityActivityFailure::Unavailable(error.into()))?
            .is_empty()
    {
        protected_kinds.push("mention");
    }
    if org
        .has_ready_work()
        .await
        .map_err(|error| CapacityActivityFailure::Unavailable(error.into()))?
    {
        protected_kinds.push("ready_work");
    }
    if daemon.lifecycle.is_recovering()
        || daemon.lifecycle.is_draining()
        || daemon.lifecycle.active() > 0
    {
        protected_kinds.push("restore");
    }
    Ok(protected_kinds)
}

async fn company_for_cell(
    daemon: &Daemon,
    company_id: Uuid,
    cell_id: Uuid,
) -> Result<Option<String>> {
    for company in crate::configured_companies(&daemon.root)? {
        let org = daemon.orgintel.get(&company).await?;
        if org.company_access_identity().await?
            == Some(restless_orgintel::CompanyAccessIdentity {
                company_id,
                cell_id,
            })
        {
            return Ok(Some(company));
        }
    }
    Ok(None)
}

fn authorized(headers: &HeaderMap, deployment: &CapacityActivityDeployment) -> bool {
    let host_matches = headers
        .get(HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<axum::http::uri::Authority>().ok())
        .is_some_and(|authority| {
            authority
                .host()
                .eq_ignore_ascii_case(&deployment.plane_hostname)
                && authority.port_u16().is_none_or(|port| port == 443)
        });
    let mut values = headers.get_all(AUTHORIZATION).iter();
    let bearer_matches = values
        .next()
        .and_then(|value| value.as_bytes().strip_prefix(b"Bearer "))
        .is_some_and(|candidate| constant_time_equal(candidate, &deployment.token));
    host_matches && bearer_matches && values.next().is_none()
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let length = left.len().max(right.len());
    for index in 0..length {
        difference |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    difference == 0
}

fn refusal(status: StatusCode, code: &'static str) -> Response {
    (status, Json(json!({ "error": code }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{header::CONTENT_TYPE, Request},
    };
    use tower::ServiceExt as _;

    #[test]
    fn machine_route_matches_only_one_exact_cell_path() {
        let cell = Uuid::new_v4();
        assert!(is_capacity_activity_path(&format!(
            "/v1/cells/{cell}/capacity-activity"
        )));
        assert!(!is_capacity_activity_path(&format!(
            "/api/v1/cells/{cell}/capacity-activity"
        )));
        assert!(!is_capacity_activity_path(
            "/v1/cells/not-a-cell/capacity-activity"
        ));
    }

    #[test]
    fn bearer_comparison_is_exact() {
        assert!(constant_time_equal(b"exact-token", b"exact-token"));
        assert!(!constant_time_equal(b"exact-token", b"exact-token-extra"));
        assert!(!constant_time_equal(b"other-token", b"exact-token"));
    }

    #[tokio::test]
    async fn route_enforces_host_bearer_company_cell_and_revision_boundaries() {
        let owner_id = Uuid::new_v4();
        let company_id = Uuid::new_v4();
        let cell_id = Uuid::new_v4();
        let runtime_id = format!("restless-cell-{cell_id}");
        let token = "capacity-activity-test-token-at-least-32-characters";
        let service = CapacityActivityService::Enabled {
            deployment: Arc::new(CapacityActivityDeployment {
                owner_id,
                plane_hostname: "plane.restless.test".into(),
                token: token.as_bytes().to_vec(),
            }),
            source: CapacityActivitySource::Fixed {
                company_id,
                cell_id,
                runtime_id: runtime_id.clone(),
                desired_revision: 7,
                protected_kinds: vec!["mention"],
            },
        };
        let app = routes::<CapacityActivityService>().with_state(service);
        let request = |path_cell_id: Uuid,
                       host: &str,
                       bearer: &str,
                       body_company_id: Uuid,
                       body_cell_id: Uuid,
                       desired_revision: i64| {
            Request::builder()
                .method("POST")
                .uri(format!("/v1/cells/{path_cell_id}/capacity-activity"))
                .header(HOST, host)
                .header(AUTHORIZATION, format!("Bearer {bearer}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "contract_version": 1,
                        "owner_id": owner_id,
                        "company_id": body_company_id,
                        "cell_id": body_cell_id,
                        "runtime_id": runtime_id.clone(),
                        "desired_revision": desired_revision,
                    }))
                    .unwrap(),
                ))
                .unwrap()
        };

        let response = app
            .clone()
            .oneshot(request(
                cell_id,
                "plane.restless.test",
                token,
                company_id,
                cell_id,
                7,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 8 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(body["desired_revision"], 7);
        assert_eq!(body["protected_kinds"], json!(["mention"]));

        for rejected in [
            request(
                cell_id,
                "attacker.restless.test",
                token,
                company_id,
                cell_id,
                7,
            ),
            request(
                cell_id,
                "plane.restless.test",
                "wrong-token-at-least-32-characters-long",
                company_id,
                cell_id,
                7,
            ),
        ] {
            assert_eq!(
                app.clone().oneshot(rejected).await.unwrap().status(),
                StatusCode::UNAUTHORIZED
            );
        }
        for rejected in [
            request(
                cell_id,
                "plane.restless.test",
                token,
                Uuid::new_v4(),
                cell_id,
                7,
            ),
            request(
                Uuid::new_v4(),
                "plane.restless.test",
                token,
                company_id,
                cell_id,
                7,
            ),
            request(
                cell_id,
                "plane.restless.test",
                token,
                company_id,
                cell_id,
                6,
            ),
        ] {
            assert_eq!(
                app.clone().oneshot(rejected).await.unwrap().status(),
                StatusCode::CONFLICT
            );
        }
    }
}
