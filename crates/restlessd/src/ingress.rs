//! The event ingress (S03-T2): the world's authenticated front door.
//!
//! Every sprint so far, a company could only act on what it polled. That is a
//! tax on top of being slightly behind the world it is supposed to be running,
//! and it is unavoidable regardless of email — payments are webhook-native
//! (`company-runtime §11.4`), so this rail is needed the moment any company
//! touches real money. Building it for email first means email rides the rail
//! every later provider will use, rather than getting a one-off receive
//! mechanism that gets rewritten when Stripe arrives.
//!
//! ## Its own failure boundary
//!
//! F12 from sprint 01: one company's hung Docker took down all three. The
//! lesson generalises — a public listener that shares a failure path with the
//! scheduler means a slow, malformed, or flood-abused request can stall the
//! company. So this runs on its own listener and its own task, and does
//! **exactly three things**: verify the signature, dedupe by provider event id,
//! enqueue. It never touches OrgIntel synchronously.
//!
//! ## The trust boundary is the signature, not the network
//!
//! An IP allowlist is a configuration that rots; an HMAC is arithmetic. An
//! unsigned or invalid request is dropped before it reaches anything that can
//! write. This is the sprint's one **Invariant** disposition.
//!
//! Resend signs with Svix: headers `svix-id`, `svix-timestamp`, `svix-signature`,
//! over `{id}.{timestamp}.{body}`, keyed by the base64 body of a `whsec_` secret.

use std::sync::Arc;

use anyhow::{bail, Context as _, Result};
use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::TcpListener;

/// Default port for the ingress. Distinct from the coordination port (7791) so
/// the two cannot be confused in a firewall rule, and so binding one does not
/// imply exposing the other.
pub const INGRESS_PORT: u16 = 7792;

/// How long a signed timestamp stays acceptable. A captured-and-replayed
/// request outside this window is rejected even with a valid signature, which
/// is what makes signature verification a defence against replay rather than
/// only against forgery.
const TOLERANCE_SECS: i64 = 5 * 60;

/// Largest body we will read. A webhook is a small JSON document; anything
/// larger is either a bug or an attempt to exhaust memory, and the failure
/// boundary is worth nothing if the listener can be OOMed through it.
const MAX_BODY: usize = 256 * 1024;

/// One verified inbound event, handed to the queue.
#[derive(Debug, Clone)]
pub struct InboundEvent {
    /// The provider's own event id — the dedupe key. Not ours: the whole point
    /// is that a redelivery of the *same* provider event is one event.
    pub provider_event_id: String,
    pub company: String,
    pub body: serde_json::Value,
}

/// What the ingress does with an accepted event. A trait so the listener can be
/// tested without OrgIntel, a database, or a company.
pub trait Sink: Send + Sync + 'static {
    fn accept(&self, event: InboundEvent);
}

/// Verify a Svix-style signature over the raw body.
///
/// Returns the reason on failure rather than a bare bool: a webhook that is
/// being rejected is exactly when the owner needs to know *why*, and "invalid
/// signature" covers a wrong secret, a stale timestamp, and a truncated body,
/// which have completely different fixes.
pub fn verify_signature(
    secret: &str,
    svix_id: &str,
    svix_timestamp: &str,
    svix_signature: &str,
    body: &[u8],
    now_unix: i64,
) -> Result<()> {
    let timestamp: i64 = svix_timestamp
        .parse()
        .with_context(|| format!("svix-timestamp {svix_timestamp:?} is not a unix timestamp"))?;
    if (now_unix - timestamp).abs() > TOLERANCE_SECS {
        bail!(
            "signed timestamp is {}s away from now, outside the {TOLERANCE_SECS}s tolerance — \
             this is a replay or a badly skewed clock",
            (now_unix - timestamp).abs()
        );
    }

    // `whsec_` prefixed secrets carry a base64 key; a raw secret is used as-is.
    let key = match secret.strip_prefix("whsec_") {
        Some(encoded) => base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .context("webhook secret after `whsec_` is not valid base64")?,
        None => secret.as_bytes().to_vec(),
    };

    let mut mac = <Hmac<Sha256>>::new_from_slice(&key).context("hmac key")?;
    mac.update(svix_id.as_bytes());
    mac.update(b".");
    mac.update(svix_timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    let expected = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    // The header carries a space-separated list of `v1,<sig>` — a secret being
    // rotated produces two, and both must be tried or rotation breaks delivery.
    let matched = svix_signature.split(' ').any(|entry| {
        let candidate = entry.split_once(',').map_or(entry, |(_version, sig)| sig);
        // Constant-time compare: a timing oracle on a signature check is the
        // one place where "it is only a webhook" is wrong.
        constant_time_eq(candidate.as_bytes(), expected.as_bytes())
    });
    if !matched {
        bail!("no signature in `svix-signature` matches the body under this secret");
    }
    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Serve the ingress until the process ends. Spawned on its own task; a failure
/// here is logged and the listener continues, because one bad request must not
/// end the rail.
pub async fn serve<S: Sink>(port: u16, secret: String, sink: Arc<S>) -> Result<()> {
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("bind ingress {addr}"))?;
    tracing::info!(addr = %addr, "event ingress listening");
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(error) => {
                tracing::warn!(%error, "ingress accept failed");
                continue;
            }
        };
        let secret = secret.clone();
        let sink = Arc::clone(&sink);
        tokio::spawn(async move {
            if let Err(error) = handle(stream, &secret, sink.as_ref()).await {
                tracing::warn!(%peer, error = %format!("{error:#}"), "ingress request rejected");
            }
        });
    }
}

/// Minimal HTTP/1.1 request handling. Deliberately hand-rolled rather than
/// pulling a web framework: this endpoint has exactly one route, one method,
/// and three jobs, and a framework would be more surface than the thing it
/// serves. When the owner API needs HTTP (S03-T8 defers it), that is the moment
/// to reconsider — not now.
async fn handle<S: Sink>(stream: tokio::net::TcpStream, secret: &str, sink: &S) -> Result<()> {
    let (read, mut write) = tokio::io::split(stream);
    let mut reader = BufReader::new(read);

    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .await
        .context("read request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let mut headers: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    loop {
        let mut line = String::new();
        let read_bytes = reader.read_line(&mut line).await.context("read header")?;
        if read_bytes == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_lowercase(), value.trim().to_string());
        }
    }

    // Health probe: no signature, no body, no side effect. Exists so "is the
    // rail up" is answerable without forging a webhook — probe, never guess.
    if method == "GET" && path.starts_with("/health") {
        respond(&mut write, 200, r#"{"ok":true,"surface":"event-ingress"}"#).await?;
        return Ok(());
    }
    if method != "POST" {
        respond(&mut write, 405, r#"{"error":"only POST is accepted"}"#).await?;
        return Ok(());
    }

    let length: usize = headers
        .get("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    if length > MAX_BODY {
        respond(&mut write, 413, r#"{"error":"body too large"}"#).await?;
        bail!("body of {length} bytes exceeds the {MAX_BODY} limit");
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).await.context("read body")?;

    // The company is the path segment: /inbound/<company>. Routing by path
    // keeps one listener serving every company without a header convention a
    // provider may not let us set.
    let company = path
        .trim_start_matches('/')
        .split('/')
        .nth(1)
        .unwrap_or("")
        .to_string();
    if company.is_empty() {
        respond(&mut write, 404, r#"{"error":"POST /inbound/<company>"}"#).await?;
        bail!("no company in path {path:?}");
    }

    let (svix_id, svix_timestamp, svix_signature) = (
        headers.get("svix-id").cloned().unwrap_or_default(),
        headers.get("svix-timestamp").cloned().unwrap_or_default(),
        headers.get("svix-signature").cloned().unwrap_or_default(),
    );
    let now = chrono::Utc::now().timestamp();
    if let Err(error) = verify_signature(
        secret,
        &svix_id,
        &svix_timestamp,
        &svix_signature,
        &body,
        now,
    ) {
        // 401 and no detail on the wire: an attacker probing the endpoint
        // learns nothing about which part failed. The reason goes to our log.
        respond(
            &mut write,
            401,
            r#"{"error":"signature verification failed"}"#,
        )
        .await?;
        return Err(error.context("rejecting unsigned or mis-signed inbound event"));
    }

    let parsed: serde_json::Value =
        serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    // Provider event id, preferred from the body, falling back to the svix id.
    let provider_event_id = parsed
        .get("data")
        .and_then(|data| data.get("email_id"))
        .and_then(|id| id.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| svix_id.clone());

    sink.accept(InboundEvent {
        provider_event_id,
        company,
        body: parsed,
    });
    // Accepted, not processed — §4.4: long work returns an accepted identity
    // rather than holding the connection. Resend needs a fast 2xx or it retries.
    respond(&mut write, 202, r#"{"status":"accepted"}"#).await?;
    Ok(())
}

async fn respond<W: tokio::io::AsyncWrite + Unpin>(
    write: &mut W,
    status: u16,
    body: &str,
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    write
        .write_all(response.as_bytes())
        .await
        .context("write response")?;
    write.flush().await.ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "whsec_dGVzdC1zZWNyZXQtZm9yLXNpZ25pbmctb25seQ==";

    fn sign(secret: &str, id: &str, timestamp: i64, body: &[u8]) -> String {
        let key = base64::engine::general_purpose::STANDARD
            .decode(secret.strip_prefix("whsec_").unwrap())
            .unwrap();
        let mut mac = <Hmac<Sha256>>::new_from_slice(&key).unwrap();
        mac.update(id.as_bytes());
        mac.update(b".");
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        format!(
            "v1,{}",
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
        )
    }

    /// The sprint's one Invariant: a forged or unsigned request never reaches
    /// anything that can write.
    #[test]
    fn an_unsigned_or_forged_event_is_rejected() {
        let body = br#"{"type":"email.received"}"#;
        let now = 1_700_000_000;
        assert!(verify_signature(SECRET, "msg_1", &now.to_string(), "", body, now).is_err());
        assert!(verify_signature(SECRET, "msg_1", &now.to_string(), "v1,AAAA", body, now).is_err());
        // And a valid signature over a DIFFERENT body does not transfer.
        let signature = sign(SECRET, "msg_1", now, br#"{"type":"other"}"#);
        assert!(
            verify_signature(SECRET, "msg_1", &now.to_string(), &signature, body, now).is_err()
        );
    }

    #[test]
    fn a_correctly_signed_event_verifies() {
        let body = br#"{"type":"email.received","data":{"email_id":"abc"}}"#;
        let now = 1_700_000_000;
        let signature = sign(SECRET, "msg_1", now, body);
        verify_signature(SECRET, "msg_1", &now.to_string(), &signature, body, now)
            .expect("a correctly signed event must verify");
    }

    /// A captured request replayed tomorrow carries a genuine signature. The
    /// timestamp window is what makes that fail, so it is tested separately
    /// from forgery — they are different attacks with the same symptom.
    #[test]
    fn a_stale_but_genuine_signature_is_still_rejected() {
        let body = br#"{"type":"email.received"}"#;
        let signed_at = 1_700_000_000;
        let signature = sign(SECRET, "msg_1", signed_at, body);
        let much_later = signed_at + 3600;
        let error = verify_signature(
            SECRET,
            "msg_1",
            &signed_at.to_string(),
            &signature,
            body,
            much_later,
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("replay") || error.contains("tolerance"),
            "{error}"
        );
    }

    /// Rotation delivers two signatures in one header. Rejecting the second
    /// would break every delivery during a rotation window.
    #[test]
    fn a_rotating_secret_sends_two_signatures_and_either_may_match() {
        let body = br#"{"type":"email.received"}"#;
        let now = 1_700_000_000;
        let good = sign(SECRET, "msg_1", now, body);
        let header = format!("v1,AAAAinvalidAAAA {good}");
        verify_signature(SECRET, "msg_1", &now.to_string(), &header, body, now)
            .expect("the valid signature in the list must be accepted");
    }
}
