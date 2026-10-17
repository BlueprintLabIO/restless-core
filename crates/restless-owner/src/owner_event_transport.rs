//! Local HTTP/1 browsers otherwise spend all six per-origin connections on live updates.
//! Keep the existing SSE event body and session lifetime; WebSocket merely carries its bytes.
use super::*;
use axum::extract::ws::{rejection::WebSocketUpgradeRejection, Message, WebSocketUpgrade};
use futures_util::StreamExt;

pub(super) type Upgrade = std::result::Result<WebSocketUpgrade, WebSocketUpgradeRejection>;

pub(super) fn respond(response: Response<Body>, upgrade: Upgrade) -> Response<Body> {
    let Ok(upgrade) = upgrade else {
        return response;
    };
    upgrade.on_upgrade(move |mut socket| async move {
        let mut stream = response.into_body().into_data_stream();
        loop {
            tokio::select! {
                next = stream.next() => match next {
                    Some(Ok(bytes)) => if socket.send(Message::Binary(bytes)).await.is_err() { break; },
                    Some(Err(_)) | None => break,
                },
                incoming = socket.recv() => match incoming {
                    Some(Ok(Message::Ping(data))) => if socket.send(Message::Pong(data)).await.is_err() { break; },
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    _ => {}, // Read-only stream: clients cannot submit commands here.
                }
            }
        }
        let _ = socket.send(Message::Close(None)).await;
    }).into_response()
}
