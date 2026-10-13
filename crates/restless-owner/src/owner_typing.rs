//! Who is typing in a Room right now (S73). Ephemeral process memory, as presence is: a signal
//! reaches the Room's open streams and is kept nowhere, so typing never touches the database or
//! the company's change hints.

use super::*;
use std::sync::LazyLock;

/// One person typing in one Room of one company.
#[derive(Debug, Clone)]
pub(super) struct RoomTyping {
    pub company: String,
    pub room_id: Uuid,
    pub actor_id: String,
}

/// Every open Room stream subscribes; a stream that falls behind simply misses a signal.
pub(super) static TYPING: LazyLock<tokio::sync::broadcast::Sender<RoomTyping>> =
    LazyLock::new(|| tokio::sync::broadcast::channel(256).0);

/// Say this person is typing in the Room. Only someone in the Room may.
pub(super) async fn room_typing(
    State(state): State<RoomApiState>,
    RoomPrincipal(principal): RoomPrincipal,
    AxumPath((company, room)): AxumPath<(String, Uuid)>,
) -> Response<Body> {
    let org = match room_orgintel(&state, &principal, &company).await {
        Ok(org) => org,
        Err(response) => return response,
    };
    match org.room_open_to(room, principal.actor_id()).await {
        Ok(true) => {
            let _ = TYPING.send(RoomTyping {
                company,
                room_id: room,
                actor_id: principal.actor_id().to_string(),
            });
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => api_error(StatusCode::NOT_FOUND, "room", "the Room is unavailable"),
        Err(error) => {
            tracing::error!(%error, "typing signal could not be checked");
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                "company collaboration is temporarily unavailable",
            )
        }
    }
}
