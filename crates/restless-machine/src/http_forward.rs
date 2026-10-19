//! Forward one HTTP request to a Docker socket, carrying a hijacked connection (`exec`, `attach`)
//! through when Docker upgrades it. Shared by the machine's filtering proxy and the plane's router.

use std::path::Path;

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt as _;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;

pub type Body = BoxBody<Bytes, std::io::Error>;

pub fn incoming(body: Incoming) -> Body {
    body.map_err(std::io::Error::other).boxed()
}

/// Send `request` to the Docker socket at `socket`.
pub async fn forward(socket: &Path, request: Request<Body>) -> anyhow::Result<Response<Incoming>> {
    let stream = UnixStream::connect(socket).await?;
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream)).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.with_upgrades().await {
            tracing::debug!(%error, "docker socket connection ended");
        }
    });
    Ok(sender.send_request(request).await?)
}

/// Forward and, when Docker switches protocols, join the caller's upgraded connection to
/// Docker's.
pub async fn forward_upgrading(
    socket: &Path,
    request: Request<Body>,
    upgrade: Option<hyper::upgrade::OnUpgrade>,
) -> anyhow::Result<Response<Body>> {
    let mut response = forward(socket, request).await?;
    if response.status() == StatusCode::SWITCHING_PROTOCOLS {
        if let Some(caller) = upgrade {
            let docker = hyper::upgrade::on(&mut response);
            tokio::spawn(async move {
                let (Ok(caller), Ok(docker)) = (caller.await, docker.await) else {
                    return;
                };
                let (mut caller, mut docker) = (TokioIo::new(caller), TokioIo::new(docker));
                let _ = tokio::io::copy_bidirectional(&mut caller, &mut docker).await;
            });
        }
    }
    let (parts, body) = response.into_parts();
    Ok(Response::from_parts(parts, incoming(body)))
}
