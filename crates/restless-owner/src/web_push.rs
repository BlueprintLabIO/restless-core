//! Standard Web Push (RFC 8030) from the account plane, so what needs a person reaches them with
//! every cockpit tab closed.
//!
//! The plane holds one VAPID key pair (RFC 8292): generated here, kept beside the other plane keys
//! with owner-only permissions, and never handed to a company. Browsers subscribe with its public
//! half, so a push service accepts a message for those subscriptions only when this plane signed
//! it. Each message is encrypted to the browser (RFC 8291, `aes128gcm`), so the push service
//! carries it without reading it.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ring::rand::{SecureRandom as _, SystemRandom};
use ring::signature::{EcdsaKeyPair, KeyPair as _, ECDSA_P256_SHA256_FIXED_SIGNING};
use ring::{aead, agreement, hkdf};

const KEY_FILE: &str = "web-push-vapid.pk8";
/// Push services accept at least 4096 bytes of body; this leaves room for the encryption header,
/// the padding delimiter and the tag.
const MAX_PLAINTEXT: usize = 3800;
const RECORD_SIZE: u32 = 4096;
/// How long a push service keeps a message for a browser that is offline.
const TTL_SECONDS: u32 = 24 * 60 * 60;

/// Push services browsers use. A subscription naming anything else is refused, so a person cannot
/// make the plane post to an arbitrary address.
const PUSH_SERVICE_SUFFIXES: [&str; 4] = [
    "googleapis.com",
    "push.services.mozilla.com",
    "push.apple.com",
    "notify.windows.com",
];

#[derive(Clone)]
pub struct WebPush {
    key: Arc<EcdsaKeyPair>,
    public: Arc<str>,
    rng: SystemRandom,
}

impl std::fmt::Debug for WebPush {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebPush")
            .field("public", &self.public)
            .finish_non_exhaustive()
    }
}

/// What a push service said about one message.
#[derive(Debug, PartialEq, Eq)]
pub enum Sent {
    Accepted,
    /// The browser unsubscribed or the subscription expired: forget it.
    Gone,
}

impl WebPush {
    /// Open this plane's key pair, creating it on first use.
    pub fn open(root: &Path) -> Result<Self> {
        let path = root.join(KEY_FILE);
        let rng = SystemRandom::new();
        let pkcs8 = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let generated =
                    EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                        .map_err(|_| anyhow::anyhow!("could not generate a Web Push key"))?;
                match create_key(&path, generated.as_ref()) {
                    Ok(()) => generated.as_ref().to_vec(),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        fs::read(&path)
                            .with_context(|| format!("read concurrent {}", path.display()))?
                    }
                    Err(error) => {
                        return Err(error)
                            .with_context(|| format!("create Web Push key {}", path.display()))
                    }
                }
            }
            Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
        };
        Self::from_pkcs8(&pkcs8, rng)
    }

    fn from_pkcs8(pkcs8: &[u8], rng: SystemRandom) -> Result<Self> {
        let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8, &rng)
            .map_err(|error| anyhow::anyhow!("Web Push key is not a P-256 key: {error}"))?;
        let public = URL_SAFE_NO_PAD.encode(key.public_key().as_ref()).into();
        Ok(Self {
            key: Arc::new(key),
            public,
            rng,
        })
    }

    /// The public key browsers subscribe with (`applicationServerKey`), base64url.
    pub fn public_key(&self) -> &str {
        &self.public
    }

    /// `Authorization: vapid t=…, k=…` for one push service origin, valid for twelve hours.
    fn authorization(&self, endpoint: &url::Url, subject: &str) -> Result<String> {
        let audience = endpoint.origin().ascii_serialization();
        let expires = chrono::Utc::now().timestamp() + 12 * 60 * 60;
        let header = URL_SAFE_NO_PAD.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&serde_json::json!({
            "aud": audience,
            "exp": expires,
            "sub": subject,
        }))?);
        let input = format!("{header}.{claims}");
        let signature = self
            .key
            .sign(&self.rng, input.as_bytes())
            .map_err(|_| anyhow::anyhow!("could not sign a Web Push request"))?;
        Ok(format!(
            "vapid t={input}.{}, k={}",
            URL_SAFE_NO_PAD.encode(signature.as_ref()),
            self.public
        ))
    }

    /// Encrypt `payload` for one browser and post it to its push service. `subject` is a contact
    /// the push service can reach the plane's operator at (`https:` or `mailto:`).
    pub async fn send(
        &self,
        http: &reqwest::Client,
        endpoint: &str,
        p256dh: &str,
        auth: &str,
        subject: &str,
        payload: &[u8],
    ) -> Result<Sent> {
        let url = push_service(endpoint).context("not a browser push service")?;
        let body = encrypt(&self.rng, payload, p256dh, auth)?;
        let response = http
            .post(url.clone())
            .header("Authorization", self.authorization(&url, subject)?)
            .header("Content-Encoding", "aes128gcm")
            .header("Content-Type", "application/octet-stream")
            .header("TTL", TTL_SECONDS.to_string())
            .header("Urgency", "high")
            .body(body)
            .timeout(Duration::from_secs(15))
            .send()
            .await
            // The endpoint is a capability for that browser; keep it out of logs.
            .map_err(|error| {
                anyhow::anyhow!("push service unreachable: {}", error.without_url())
            })?;
        match response.status().as_u16() {
            200..=299 => Ok(Sent::Accepted),
            404 | 410 => Ok(Sent::Gone),
            status => bail!("push service refused the message ({status})"),
        }
    }
}

/// The subscription endpoint, when it is an `https` address of a browser push service.
pub fn push_service(endpoint: &str) -> Option<url::Url> {
    let url = url::Url::parse(endpoint).ok()?;
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || endpoint.len() > 2048
    {
        return None;
    }
    let url::Host::Domain(host) = url.host()? else {
        return None;
    };
    let host = host.to_ascii_lowercase();
    PUSH_SERVICE_SUFFIXES
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
        .then_some(url)
}

/// Whether a browser's subscription keys have the shape RFC 8291 requires.
pub fn valid_keys(p256dh: &str, auth: &str) -> bool {
    let point = URL_SAFE_NO_PAD.decode(p256dh.trim_end_matches('='));
    let secret = URL_SAFE_NO_PAD.decode(auth.trim_end_matches('='));
    matches!((point, secret), (Ok(point), Ok(secret)) if point.len() == 65 && point[0] == 4 && secret.len() == 16)
}

struct Len(usize);

impl hkdf::KeyType for Len {
    fn len(&self) -> usize {
        self.0
    }
}

fn expand(prk: &hkdf::Prk, info: &[&[u8]], out: &mut [u8]) -> Result<()> {
    prk.expand(info, Len(out.len()))
        .and_then(|okm| okm.fill(out))
        .map_err(|_| anyhow::anyhow!("Web Push key derivation failed"))
}

/// RFC 8291 message encryption: one `aes128gcm` record keyed from an ephemeral ECDH agreement
/// with the browser's key and its authentication secret.
fn encrypt(rng: &SystemRandom, payload: &[u8], p256dh: &str, auth: &str) -> Result<Vec<u8>> {
    if payload.len() > MAX_PLAINTEXT {
        bail!("Web Push message is too long");
    }
    if !valid_keys(p256dh, auth) {
        bail!("browser push keys are malformed");
    }
    let browser_public = URL_SAFE_NO_PAD.decode(p256dh.trim_end_matches('='))?;
    let auth_secret = URL_SAFE_NO_PAD.decode(auth.trim_end_matches('='))?;

    let ephemeral = agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, rng)
        .map_err(|_| anyhow::anyhow!("could not generate a Web Push message key"))?;
    let plane_public = ephemeral
        .compute_public_key()
        .map_err(|_| anyhow::anyhow!("could not compute a Web Push message key"))?;
    let plane_public = plane_public.as_ref().to_vec();
    let mut ikm = [0u8; 32];
    agreement::agree_ephemeral(
        ephemeral,
        &agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, &browser_public),
        |shared| {
            let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, &auth_secret).extract(shared);
            expand(
                &prk,
                &[b"WebPush: info\0", &browser_public, &plane_public],
                &mut ikm,
            )
        },
    )
    .map_err(|_| anyhow::anyhow!("browser push key is not a P-256 point"))??;

    let mut salt = [0u8; 16];
    rng.fill(&mut salt)
        .map_err(|_| anyhow::anyhow!("could not draw a Web Push salt"))?;
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, &salt).extract(&ikm);
    let mut key = [0u8; 16];
    let mut nonce = [0u8; 12];
    expand(&prk, &[b"Content-Encoding: aes128gcm\0"], &mut key)?;
    expand(&prk, &[b"Content-Encoding: nonce\0"], &mut nonce)?;

    let mut record = Vec::with_capacity(payload.len() + 17);
    record.extend_from_slice(payload);
    // The last (and only) record's padding delimiter.
    record.push(2);
    aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_128_GCM, &key)
            .map_err(|_| anyhow::anyhow!("Web Push content key rejected"))?,
    )
    .seal_in_place_append_tag(
        aead::Nonce::assume_unique_for_key(nonce),
        aead::Aad::empty(),
        &mut record,
    )
    .map_err(|_| anyhow::anyhow!("Web Push encryption failed"))?;

    let mut body = Vec::with_capacity(16 + 4 + 1 + plane_public.len() + record.len());
    body.extend_from_slice(&salt);
    body.extend_from_slice(&RECORD_SIZE.to_be_bytes());
    body.push(plane_public.len() as u8);
    body.extend_from_slice(&plane_public);
    body.extend_from_slice(&record);
    Ok(body)
}

fn create_key(path: &Path, pkcs8: &[u8]) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(pkcs8)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plane posts only to browser push services: a subscription is person-supplied input, so
    /// anything else would let a member make the plane call an address of their choosing.
    #[test]
    fn only_browser_push_services_are_reachable() {
        for allowed in [
            "https://fcm.googleapis.com/fcm/send/abc",
            "https://updates.push.services.mozilla.com/wpush/v2/abc",
            "https://web.push.apple.com/QGx",
            "https://wns2-par02p.notify.windows.com/w/?token=abc",
        ] {
            assert!(push_service(allowed).is_some(), "{allowed}");
        }
        for refused in [
            "http://fcm.googleapis.com/fcm/send/abc",
            "https://fcm.googleapis.com:8443/fcm/send/abc",
            "https://169.254.169.254/latest",
            "https://localhost/push",
            "https://evilgoogleapis.com/x",
            "https://googleapis.com.evil.example/x",
            "https://user@fcm.googleapis.com/x",
            "not a url",
        ] {
            assert!(push_service(refused).is_none(), "{refused}");
        }
    }
}
