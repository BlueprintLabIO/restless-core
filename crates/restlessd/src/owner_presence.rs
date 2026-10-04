//! Who else is in this company's cockpit right now. Presence is ephemeral
//! process memory: each open cockpit heartbeats, and anyone silent for a
//! little over a minute has left. It is never stored or governed.

use super::*;
use std::sync::LazyLock;

/// A cockpit heartbeats every 30 seconds; two missed beats means gone.
const PRESENT_FOR: Duration = Duration::from_secs(75);

static PRESENCE: LazyLock<Mutex<HashMap<String, HashMap<String, SystemTime>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Serialize)]
struct PresentPerson {
    actor_id: String,
    display: String,
}

/// Record this principal as present and return the other people present.
/// Only humans appear: agents show their work where it happens, not as a
/// roster of faces.
pub(super) async fn heartbeat(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    let me = principal.actor_id().to_string();
    let now = SystemTime::now();
    let others: Vec<String> = {
        let mut presence = PRESENCE.lock().expect("presence registry");
        let company_presence = presence.entry(company.clone()).or_default();
        company_presence.insert(me.clone(), now);
        company_presence
            .retain(|_, seen| now.duration_since(*seen).unwrap_or_default() <= PRESENT_FOR);
        company_presence
            .keys()
            .filter(|actor| **actor != me)
            .cloned()
            .collect()
    };
    if others.is_empty() {
        return Json(serde_json::json!({ "present": [] })).into_response();
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            );
        }
    };
    let actors = match org.list_actors().await {
        Ok(actors) => actors,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            );
        }
    };
    let present: Vec<PresentPerson> = others
        .into_iter()
        .filter_map(|actor_id| {
            actors
                .iter()
                .find(|actor| {
                    actor.id == actor_id && matches!(actor.kind.as_str(), "owner" | "human")
                })
                .map(|actor| PresentPerson {
                    actor_id: actor.id.clone(),
                    display: actor.display.clone(),
                })
        })
        .collect();
    Json(serde_json::json!({ "present": present })).into_response()
}

/// Leaving the cockpit removes this principal at once rather than waiting out
/// the heartbeat window.
pub(super) async fn leave(
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if let Some(company_presence) = PRESENCE
        .lock()
        .expect("presence registry")
        .get_mut(&company)
    {
        company_presence.remove(principal.actor_id());
    }
    StatusCode::NO_CONTENT.into_response()
}
