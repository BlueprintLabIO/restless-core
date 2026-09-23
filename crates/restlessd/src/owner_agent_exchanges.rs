//! Read-only owner view of recorded internal conversations.
use super::*;

#[derive(Deserialize)]
pub(super) struct ExchangeQuery {
    before: Option<i64>,
}

pub(super) async fn list(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath((company, actor)): AxumPath<(String, String)>,
    Query(query): Query<ExchangeQuery>,
) -> impl IntoResponse {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "agent_exchanges",
            "Only the owner can inspect internal agent exchanges.",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "agent_exchanges",
                format!("{e:#}"),
            )
        }
    };
    let actors = match org.list_actors().await {
        Ok(actors) => actors,
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "agent_exchanges",
                format!("{e:#}"),
            )
        }
    };
    if !actors.iter().any(|a| a.id == actor) {
        return api_error(
            StatusCode::NOT_FOUND,
            "actor",
            "This person no longer exists.",
        );
    }
    let mut messages = match org.agent_exchanges_before(&actor, query.before, 31).await {
        Ok(messages) => messages,
        Err(e) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "agent_exchanges",
                format!("{e:#}"),
            )
        }
    };
    let more = messages.len() > 30;
    messages.truncate(30);
    let next_before = more.then(|| messages.last().expect("full page").id);
    let names: BTreeMap<_, _> = actors.into_iter().map(|a| (a.id, a.display)).collect();
    Json(serde_json::json!({
        "messages": messages.into_iter().map(conversation_message_view).collect::<Vec<_>>(),
        "names": names,
        "next_before": next_before,
    }))
    .into_response()
}
