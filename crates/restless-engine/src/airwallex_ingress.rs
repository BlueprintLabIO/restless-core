//! Signed Airwallex webhook ingress on its own failure boundary.
//!
//! The provider-specific signature format differs from the existing Svix
//! ingress, so it gets one small listener rather than a false universal event
//! envelope. Verified payloads only wake authenticated API reconciliation;
//! they cannot attest that money moved by themselves.

use std::sync::Arc;

use anyhow::{bail, Context as _, Result};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};

pub const AIRWALLEX_INGRESS_PORT: u16 = 7795;
const MAX_BODY: usize = 256 * 1024;

pub async fn serve(daemon: Arc<crate::Daemon>) -> Result<()> {
    let address = format!(
        "0.0.0.0:{}",
        crate::port_with_offset(AIRWALLEX_INGRESS_PORT)?
    );
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .with_context(|| format!("bind Airwallex ingress {address}"))?;
    tracing::info!(addr = %address, "Airwallex event ingress listening");
    loop {
        let (stream, peer) = listener.accept().await?;
        let daemon = Arc::clone(&daemon);
        tokio::spawn(async move {
            if let Err(error) = handle(stream, &daemon).await {
                tracing::warn!(%peer, error = %format!("{error:#}"), "Airwallex ingress request rejected");
            }
        });
    }
}

async fn handle(stream: tokio::net::TcpStream, daemon: &crate::Daemon) -> Result<()> {
    let (read, mut write) = tokio::io::split(stream);
    let mut reader = BufReader::new(read);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();

    let mut headers = std::collections::HashMap::new();
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).await?;
        if read == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    if method == "GET" && path == "/health" {
        respond(
            &mut write,
            200,
            r#"{"ok":true,"surface":"airwallex-ingress"}"#,
        )
        .await?;
        return Ok(());
    }
    if method != "POST" {
        respond(&mut write, 405, r#"{"error":"only POST is accepted"}"#).await?;
        return Ok(());
    }
    let company = path
        .strip_prefix("/airwallex/")
        .filter(|value| !value.is_empty() && !value.contains('/'))
        .context("expected POST /airwallex/<company>")?;
    let length: usize = headers
        .get("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    if length == 0 || length > MAX_BODY {
        respond(&mut write, 413, r#"{"error":"invalid body size"}"#).await?;
        bail!("Airwallex body length {length} is outside 1..={MAX_BODY}");
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).await?;
    let timestamp = headers.get("x-timestamp").map(String::as_str).unwrap_or("");
    let signature = headers.get("x-signature").map(String::as_str).unwrap_or("");
    let config = match crate::runtime::CompanyConfig::load(&daemon.root, company) {
        Ok(config) => config,
        Err(error) => {
            respond(&mut write, 404, r#"{"error":"company not found"}"#).await?;
            return Err(error);
        }
    };
    match crate::airwallex::receive_webhook(&config, &daemon.authority, timestamp, signature, &body)
        .await
    {
        Ok(observation) => {
            if let Some(observation) = observation.as_ref() {
                crate::continue_after_payment_observation(
                    daemon,
                    company,
                    &observation.payment,
                    observation.changed,
                )
                .await?;
            }
            respond(&mut write, 200, r#"{"status":"accepted"}"#).await?;
            Ok(())
        }
        Err(error) => {
            // Do not reveal which credential/signature/provider check failed.
            // A non-2xx asks Airwallex to retry an authentic transient failure.
            respond(&mut write, 401, r#"{"error":"event not accepted"}"#).await?;
            Err(error)
        }
    }
}

async fn respond(
    write: &mut tokio::io::WriteHalf<tokio::net::TcpStream>,
    status: u16,
    body: &str,
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    write.write_all(response.as_bytes()).await?;
    write.shutdown().await?;
    Ok(())
}
