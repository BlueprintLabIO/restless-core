//! Owner-only Work read model; no separate writer or workflow state.
use super::*;

#[derive(Deserialize)]
pub(super) struct WorkGraphQuery {
    #[serde(default)]
    context: bool,
}

/// Work is read independently of the decision queue. The shell needs current references;
/// history and gate output are fetched only by surfaces that actually render them.
pub(super) async fn view(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<WorkGraphQuery>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "membership_role",
            "Only the company owner can read the full Work graph.",
        );
    }
    if let Err(error) = runtime::CompanyConfig::load(&state.daemon.root, &company) {
        return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}"));
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
    if query.context {
        return match tokio::try_join!(org.list_work(), org.list_artifact_refs(None)) {
            Ok((work, artifacts)) => Json(serde_json::json!({ "work": work, "artifacts": artifacts.into_iter().filter(|artifact| artifact.work_id.is_some()).collect::<Vec<_>>() })).into_response(),
            Err(error) => api_error(StatusCode::SERVICE_UNAVAILABLE, "orgintel", format!("{error:#}")),
        };
    }
    match org.work_graph_snapshot().await {
        Ok(graph) => Json(graph).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            format!("{error:#}"),
        ),
    }
}
