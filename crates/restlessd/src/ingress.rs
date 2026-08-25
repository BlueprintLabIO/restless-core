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
//! **exactly three things**: verify the signature, durably dedupe/record in
//! Authority, and schedule the OrgIntel projection. It never waits for
//! OrgIntel before answering the provider.
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
use std::{future::Future, pin::Pin};

use anyhow::{bail, Context as _, Result};
use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
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
const MAX_REQUEST_LINE: usize = 8 * 1024;
const MAX_HEADER_LINE: usize = 16 * 1024;
const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_HEADERS: usize = 100;
const CONNECTION_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

/// One verified inbound event, handed to the queue.
#[derive(Debug, Clone)]
pub struct InboundEvent {
    /// The provider's own event id — the dedupe key. Not ours: the whole point
    /// is that a redelivery of the *same* provider event is one event.
    pub provider_event_id: String,
    pub company: String,
    pub body: serde_json::Value,
}

/// What the ingress does with a verified event. The response waits for this
/// future: 202 means Authority durably accepted the event, not merely that a
/// task was spawned and might disappear with the process.
pub trait Sink: Send + Sync + 'static {
    fn accept(&self, event: InboundEvent) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
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
    if svix_id.trim().is_empty()
        || svix_timestamp.trim().is_empty()
        || svix_signature.trim().is_empty()
    {
        bail!("signed webhook headers are incomplete");
    }
    if svix_id.len() > 512 || svix_timestamp.len() > 32 || svix_signature.len() > 8 * 1024 {
        bail!("signed webhook headers exceed their bounded contract");
    }
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
    serve_listener(listener, secret, sink).await
}

/// Serve an already-bound ingress listener. Production normally uses
/// [`serve`]; accepting a listener lets live-provider probes establish local
/// readiness before registering a temporary public webhook, with no bind
/// race or readiness polling.
pub(crate) async fn serve_listener<S: Sink>(
    listener: TcpListener,
    secret: String,
    sink: Arc<S>,
) -> Result<()> {
    let addr = listener
        .local_addr()
        .context("read ingress listener address")?;
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
            match tokio::time::timeout(CONNECTION_DEADLINE, handle(stream, &secret, sink.as_ref()))
                .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::warn!(%peer, error = %format!("{error:#}"), "ingress request rejected")
                }
                Err(_) => {
                    tracing::warn!(%peer, "ingress request exceeded bounded connection deadline")
                }
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

    let request_line = read_bounded_line(&mut reader, MAX_REQUEST_LINE)
        .await?
        .context("connection closed before request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let mut headers: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut header_bytes = 0usize;
    let mut header_count = 0usize;
    loop {
        let Some(line) = read_bounded_line(&mut reader, MAX_HEADER_LINE).await? else {
            break;
        };
        header_bytes = header_bytes.saturating_add(line.len());
        header_count += 1;
        if header_bytes > MAX_HEADER_BYTES || header_count > MAX_HEADERS {
            respond(&mut write, 431, r#"{"error":"headers too large"}"#).await?;
            anyhow::bail!("request headers exceed bounded ingress contract");
        }
        if line.trim().is_empty() {
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

    let Some(length) = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
    else {
        respond(
            &mut write,
            411,
            r#"{"error":"valid content-length required"}"#,
        )
        .await?;
        anyhow::bail!("POST request omitted a valid content-length");
    };
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

    let parsed: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(parsed) => parsed,
        Err(error) => {
            respond(&mut write, 400, r#"{"error":"invalid JSON"}"#).await?;
            return Err(error).context("signed webhook body is not valid JSON");
        }
    };
    // `svix-id` identifies this provider webhook delivery/event. The email id
    // is only correlation metadata: bounce, complaint, reply and unsubscribe
    // transitions can legitimately share it and must remain distinct.
    let provider_event_id = svix_id;

    let event = InboundEvent {
        provider_event_id,
        company,
        body: parsed,
    };
    if let Err(error) = sink.accept(event).await {
        // A non-2xx asks the provider to redeliver. Returning 202 before
        // Authority committed created a crash window in which a real reply
        // could vanish forever.
        respond(&mut write, 503, r#"{"error":"event not recorded"}"#).await?;
        return Err(error.context("verified inbound event was not recorded"));
    }
    // Authority is durable. OrgIntel projection may still be pending or
    // degraded, but provider redelivery is no longer needed for custody.
    respond(&mut write, 202, r#"{"status":"accepted"}"#).await?;
    Ok(())
}

async fn read_bounded_line<R: AsyncRead + Unpin>(
    reader: &mut R,
    limit: usize,
) -> Result<Option<String>> {
    let mut bytes = Vec::with_capacity(limit.min(1024));
    loop {
        let mut byte = [0u8; 1];
        let read = reader.read(&mut byte).await.context("read HTTP line")?;
        if read == 0 {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        if bytes.len() >= limit {
            anyhow::bail!("HTTP line exceeds {limit} bytes");
        }
        bytes.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    String::from_utf8(bytes)
        .map(Some)
        .context("HTTP line is not valid UTF-8")
}

async fn respond<W: tokio::io::AsyncWrite + Unpin>(
    write: &mut W,
    status: u16,
    body: &str,
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        411 => "Length Required",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
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

    #[tokio::test]
    async fn request_lines_are_bounded_before_unlimited_allocation() {
        let (mut writer, mut reader) = tokio::io::duplex(64);
        writer.write_all(b"short\r\n").await.unwrap();
        drop(writer);
        assert_eq!(
            read_bounded_line(&mut reader, 16).await.unwrap().as_deref(),
            Some("short\r\n")
        );

        let (mut writer, mut reader) = tokio::io::duplex(64);
        writer.write_all(b"123456789\n").await.unwrap();
        drop(writer);
        assert!(read_bounded_line(&mut reader, 8).await.is_err());
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
