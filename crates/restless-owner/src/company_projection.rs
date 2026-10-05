//! The plane's side of the content-free company projection (Cloud ADR 0005).
//!
//! Each account plane tells Fleet, for each company on it, how many decisions are waiting for the owner
//! and when work last moved. The record is a short signed token carrying integers and timestamps and
//! nothing else: there is no field in it that could hold a title, a name or a message, and Fleet refuses a
//! record that tries to add one. Fleet verifies it against the public key this plane publishes at
//! [`JWKS_PATH`]. The key is generated here, never leaves this plane, and signs nothing else.
//!
//! The plane pushes; Fleet never reads a company. Records go out when the count changes and on a
//! heartbeat, and stop (down to an occasional probe) while the owner has the projection switched off.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context as _, Result};
use axum::{
    http::{header::CACHE_CONTROL, HeaderValue},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signer as _, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use url::Url;
use uuid::Uuid;

use crate::{attention, entry::EntryMode, runtime, Daemon};

pub const JWKS_PATH: &str = "/.well-known/restless-company-projection-jwks.json";
const KEY_FILE: &str = "company-projection.ed25519";
const TOKEN_TYPE: &str = "restless-company-projection+jwt";
const ALGORITHM: &str = "EdDSA";
const AUDIENCE: &str = "restless-fleet";
const CONTRACT_VERSION: u32 = 2;
const KIND_ATTENTION: &str = "attention";

const URL_ENV: &str = "RESTLESS_PROJECTION_URL";
const INSECURE_ENV: &str = "RESTLESS_PROJECTION_ALLOW_INSECURE_HTTP";

/// How often the plane looks for a change.
const TICK: Duration = Duration::from_secs(15);
/// A record goes out at least this often while nothing changes (Cloud ADR 0005: heartbeat).
const HEARTBEAT: Duration = Duration::from_secs(120);
/// Fleet answers `disabled` when the owner has switched the projection off; the plane then only asks
/// again this rarely, to notice the switch coming back on.
const DISABLED_PROBE: Duration = Duration::from_secs(600);
const FAILURE_BACKOFF: Duration = Duration::from_secs(30);

/// The whole of the v2 `attention` record: v1's count and time plus two more counts and one yes/no.
/// Field-for-field what Fleet accepts; keep them together. Still nothing that could hold content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttentionClaims {
    pub iss: String,
    pub aud: String,
    pub contract_version: u32,
    pub kind: String,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub projected_at: i64,
    pub sequence: i64,
    pub decisions_waiting: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_activity_at: Option<i64>,
    /// Actors with a turn in flight now (v2).
    pub people_working: u32,
    /// Work completed in the 24 hours before `projected_at` (v2).
    pub outcomes_last_day: u32,
    /// Whether Exec has working intelligence and can start (v2).
    pub exec_ready: bool,
}

#[derive(Clone)]
pub struct ProjectionSigner {
    key: Arc<SigningKey>,
    key_id: Arc<str>,
}

#[derive(Debug, Serialize)]
struct TokenHeader<'a> {
    alg: &'a str,
    typ: &'a str,
    kid: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectionJwkSet {
    keys: [ProjectionJwk; 1],
}

#[derive(Debug, Clone, Serialize)]
struct ProjectionJwk {
    kty: &'static str,
    crv: &'static str,
    alg: &'static str,
    #[serde(rename = "use")]
    use_: &'static str,
    kid: String,
    x: String,
}

impl ProjectionSigner {
    pub fn open(root: &Path) -> Result<Self> {
        let path = root.join(KEY_FILE);
        let seed = match read_key(&path) {
            Ok(seed) => seed,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut seed = [0u8; 32];
                seed[..16].copy_from_slice(Uuid::new_v4().as_bytes());
                seed[16..].copy_from_slice(Uuid::new_v4().as_bytes());
                match create_key(&path, &seed) {
                    Ok(()) => seed.to_vec(),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        read_key(&path)
                            .with_context(|| format!("read concurrent {}", path.display()))?
                    }
                    Err(error) => {
                        return Err(error).with_context(|| {
                            format!("create projection signing key {}", path.display())
                        })
                    }
                }
            }
            Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
        };
        let seed: [u8; 32] = seed.try_into().map_err(|seed: Vec<u8>| {
            anyhow::anyhow!(
                "projection signing key {} has {} bytes; expected 32",
                path.display(),
                seed.len()
            )
        })?;
        Ok(Self::from_signing_key(SigningKey::from_bytes(&seed)))
    }

    fn from_signing_key(key: SigningKey) -> Self {
        let fingerprint = format!("{:x}", Sha256::digest(key.verifying_key().as_bytes()));
        Self {
            key: Arc::new(key),
            key_id: format!("company-projection-{}", &fingerprint[..24]).into(),
        }
    }

    #[cfg(test)]
    pub fn for_test(seed: [u8; 32]) -> Self {
        Self::from_signing_key(SigningKey::from_bytes(&seed))
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn jwks(&self) -> ProjectionJwkSet {
        ProjectionJwkSet {
            keys: [ProjectionJwk {
                kty: "OKP",
                crv: "Ed25519",
                alg: ALGORITHM,
                use_: "sig",
                kid: self.key_id().to_string(),
                x: base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(self.key.verifying_key().as_bytes()),
            }],
        }
    }

    pub fn sign(&self, claims: &AttentionClaims) -> Result<String> {
        let encode = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let header = encode.encode(serde_json::to_vec(&TokenHeader {
            alg: ALGORITHM,
            typ: TOKEN_TYPE,
            kid: self.key_id(),
        })?);
        let payload = encode.encode(serde_json::to_vec(claims)?);
        let input = format!("{header}.{payload}");
        let signature = encode.encode(self.key.sign(input.as_bytes()).to_bytes());
        Ok(format!("{input}.{signature}"))
    }
}

fn read_key(path: &Path) -> std::io::Result<Vec<u8>> {
    fs::read(path)
}

fn create_key(path: &Path, seed: &[u8; 32]) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(seed)?;
    file.sync_all()
}

/// The public key, for Fleet to verify against. Nothing else is served here.
pub fn routes<S>(signer: ProjectionSigner) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route(
        JWKS_PATH,
        get(move || {
            let signer = signer.clone();
            async move {
                let mut response: Response = Json(signer.jwks()).into_response();
                response.headers_mut().insert(
                    CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=300"),
                );
                response
            }
        }),
    )
}

/// What a company has to say about itself: counts, a time and whether Exec can start. This is the
/// portfolio card on every host: signed for Fleet, handed unsigned to the local root page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Summary {
    pub decisions_waiting: u32,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub people_working: u32,
    pub outcomes_last_day: u32,
    pub exec_ready: bool,
}

/// Counts the owner's queue, finds when Work last moved and counts Work completed in the last day.
/// Only lengths, counts and a timestamp leave this function; no item is read for its content.
pub fn summarize(
    view: &attention::AttentionView,
    people_working: usize,
    exec_ready: bool,
    now: DateTime<Utc>,
) -> Summary {
    summarize_parts(
        view.items.len(),
        view.work_graph.iter().flat_map(|graph| {
            graph.work.iter().map(|work| {
                (
                    work.updated_at,
                    work.status == restless_orgintel::WorkStatus::Completed,
                )
            })
        }),
        people_working,
        exec_ready,
        now,
    )
}

/// The cap on every count, as in v1: large enough to be honest, small enough to bound.
const MAX_COUNT: u32 = 1_000_000;

fn bounded(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX).min(MAX_COUNT)
}

/// `work` is each Work's last update and whether it is completed. A completed Work's last update is
/// its completion, so it counts as an outcome when that falls within the day before `now`.
pub fn summarize_parts(
    items: usize,
    work: impl IntoIterator<Item = (DateTime<Utc>, bool)>,
    people_working: usize,
    exec_ready: bool,
    now: DateTime<Utc>,
) -> Summary {
    let day_ago = now - chrono::Duration::hours(24);
    let mut last_activity_at = None;
    let mut outcomes = 0usize;
    for (updated_at, completed) in work {
        last_activity_at = last_activity_at.max(Some(updated_at));
        if completed && updated_at > day_ago && updated_at <= now {
            outcomes += 1;
        }
    }
    Summary {
        decisions_waiting: bounded(items),
        last_activity_at,
        people_working: bounded(people_working),
        outcomes_last_day: bounded(outcomes),
        exec_ready,
    }
}

/// The card for one company, as the plane sees it now. Unavailable company state is an error, never
/// a smaller queue or a quieter company.
pub async fn card_for(daemon: &Daemon, company: &str) -> Result<Summary> {
    let config = runtime::CompanyConfig::load(&daemon.root, company)?;
    let org = daemon.orgintel.get(company).await?;
    let view = attention::project(&config, &daemon.authority, Some(&org)).await?;
    let exec_waking = daemon
        .in_flight
        .lock()
        .map(|guard| guard.is_active(company))
        .unwrap_or(false);
    let working = daemon.staff.running_actors(company).len() + usize::from(exec_waking);
    let exec_ready = crate::company::observed_company_model_issue(&config)
        .await
        .is_none();
    Ok(summarize(&view, working, exec_ready, Utc::now()))
}

/// A hosted company's handle is `company_<uuid>`; anything else is not a Fleet company and is skipped.
pub fn company_id_from_handle(handle: &str) -> Option<Uuid> {
    let simple = handle.strip_prefix("company_")?;
    if simple.len() != 32 {
        return None;
    }
    let id = Uuid::parse_str(simple).ok()?;
    (crate::company_bootstrap::company_handle(id) == handle).then_some(id)
}

/// Per-company emission state, and the one decision this module makes: is it time to send?
#[derive(Debug, Default)]
struct Emission {
    last_summary: Option<Summary>,
    last_sent: Option<Instant>,
    quiet_until: Option<Instant>,
    sequence: i64,
}

impl Emission {
    fn due(&self, summary: &Summary, now: Instant) -> bool {
        if self.quiet_until.is_some_and(|until| now < until) {
            return false;
        }
        match (self.last_summary.as_ref(), self.last_sent) {
            (Some(previous), Some(sent)) => {
                previous != summary || now.duration_since(sent) >= HEARTBEAT
            }
            _ => true,
        }
    }

    fn next_sequence(&mut self, now: DateTime<Utc>) -> i64 {
        self.sequence = (self.sequence + 1).max(now.timestamp_millis());
        self.sequence
    }

    fn sent(&mut self, summary: Summary, now: Instant) {
        self.last_summary = Some(summary);
        self.last_sent = Some(now);
        self.quiet_until = None;
    }

    fn switched_off(&mut self, summary: Summary, now: Instant) {
        self.last_summary = Some(summary);
        self.last_sent = Some(now);
        self.quiet_until = Some(now + DISABLED_PROBE);
    }

    fn failed(&mut self, now: Instant) {
        self.quiet_until = Some(now + FAILURE_BACKOFF);
    }
}

#[derive(Clone)]
pub struct ProjectionEmitter {
    url: Url,
    issuer: String,
    plane_id: Uuid,
    client: reqwest::Client,
    signer: ProjectionSigner,
}

#[derive(Debug, Deserialize)]
struct Receipt {
    status: String,
}

impl ProjectionEmitter {
    /// Only a hosted account plane in network mode, and only when told where Fleet listens.
    pub fn from_environment(
        entry: &EntryMode,
        signer: ProjectionSigner,
    ) -> Result<Option<Self>> {
        let Some((_, plane_id, host)) = entry.network_coordinates() else {
            return Ok(None);
        };
        let Some(raw) = std::env::var(URL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
        else {
            return Ok(None);
        };
        let allow_insecure = matches!(std::env::var(INSECURE_ENV).as_deref(), Ok("1") | Ok("true"));
        let url = Url::parse(raw.trim()).with_context(|| format!("parse {URL_ENV}"))?;
        if url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || (url.scheme() != "https" && !(allow_insecure && url.scheme() == "http"))
        {
            bail!("{URL_ENV} must be an https URL without credentials, query or fragment");
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("build the projection client")?;
        Ok(Some(Self {
            url,
            issuer: format!("https://{host}"),
            plane_id,
            client,
            signer,
        }))
    }

    fn claims(
        &self,
        company_id: Uuid,
        sequence: i64,
        summary: &Summary,
        now: DateTime<Utc>,
    ) -> AttentionClaims {
        AttentionClaims {
            iss: self.issuer.clone(),
            aud: AUDIENCE.to_string(),
            contract_version: CONTRACT_VERSION,
            kind: KIND_ATTENTION.to_string(),
            plane_id: self.plane_id,
            company_id,
            projected_at: now.timestamp(),
            sequence,
            decisions_waiting: summary.decisions_waiting,
            last_activity_at: summary.last_activity_at.map(|at| at.timestamp()),
            people_working: summary.people_working,
            outcomes_last_day: summary.outcomes_last_day,
            exec_ready: summary.exec_ready,
        }
    }

    async fn summary_for(daemon: &Daemon, company: &str) -> Result<Summary> {
        // Unavailable company state cannot justify reporting a smaller queue or
        // no recent activity. Skip this observation until OrgIntel is readable.
        card_for(daemon, company).await
    }

    /// Returns what Fleet said: `accepted`, `disabled`, or an error to back off from.
    async fn send(&self, claims: &AttentionClaims) -> Result<Receipt, SendError> {
        let token = self.signer.sign(claims).map_err(|_| SendError::Local)?;
        let response = self
            .client
            .post(self.url.clone())
            .json(&serde_json::json!({ "token": token }))
            .send()
            .await
            .map_err(|_| SendError::Transport)?;
        match response.status().as_u16() {
            200 => response
                .json::<Receipt>()
                .await
                .map_err(|_| SendError::Transport),
            409 => Err(SendError::Replay),
            _ => Err(SendError::Refused),
        }
    }

    pub async fn run(self, daemon: Arc<Daemon>) {
        let mut state: HashMap<Uuid, Emission> = HashMap::new();
        loop {
            match crate::configured_companies(&daemon.root) {
                Ok(companies) => {
                    for company in companies {
                        let Some(company_id) = company_id_from_handle(&company) else {
                            continue;
                        };
                        let emission = state.entry(company_id).or_default();
                        let summary = match Self::summary_for(&daemon, &company).await {
                            Ok(summary) => summary,
                            Err(error) => {
                                tracing::debug!(
                                    "projection skipped a company this round: {error:#}"
                                );
                                continue;
                            }
                        };
                        let now = Instant::now();
                        if !emission.due(&summary, now) {
                            continue;
                        }
                        let sequence = emission.next_sequence(Utc::now());
                        let claims = self.claims(company_id, sequence, &summary, Utc::now());
                        match self.send(&claims).await {
                            Ok(receipt) if receipt.status == "disabled" => {
                                emission.switched_off(summary, now)
                            }
                            Ok(receipt) if receipt.status == "accepted" => {
                                emission.sent(summary, now)
                            }
                            Ok(_) => emission.failed(now),
                            Err(SendError::Replay) => {
                                // Fleet already holds a higher sequence (a restart reset ours). The
                                // clock-based sequence passes it next round.
                                emission.failed(now)
                            }
                            Err(error) => {
                                tracing::warn!(?error, "company projection was not delivered");
                                emission.failed(now)
                            }
                        }
                    }
                }
                Err(error) => tracing::warn!("projection could not list companies: {error:#}"),
            }
            tokio::time::sleep(TICK).await;
        }
    }
}

#[derive(Debug)]
enum SendError {
    Local,
    Transport,
    Replay,
    Refused,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signature, Verifier as _};

    fn emitter(signer: ProjectionSigner) -> ProjectionEmitter {
        ProjectionEmitter {
            url: Url::parse("https://fleet.example.test/v1/projections").unwrap(),
            issuer: "https://owner.example.test".into(),
            plane_id: Uuid::from_u128(1),
            client: reqwest::Client::new(),
            signer,
        }
    }

    fn summary(waiting: u32) -> Summary {
        Summary {
            decisions_waiting: waiting,
            last_activity_at: None,
            people_working: 0,
            outcomes_last_day: 0,
            exec_ready: true,
        }
    }

    #[test]
    fn a_signed_record_verifies_and_carries_only_the_contract_fields() {
        let signer = ProjectionSigner::for_test([5u8; 32]);
        let company = Uuid::from_u128(0xabc);
        let now = Utc::now();
        let claims = emitter(signer.clone()).claims(
            company,
            now.timestamp_millis(),
            &Summary {
                decisions_waiting: 4,
                last_activity_at: Some(now),
                people_working: 2,
                outcomes_last_day: 3,
                exec_ready: true,
            },
            now,
        );
        let token = signer.sign(&claims).unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let signature = Signature::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[2])
                .unwrap(),
        )
        .unwrap();
        signer
            .key
            .verifying_key()
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .unwrap();

        let payload: serde_json::Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[1])
                .unwrap(),
        )
        .unwrap();
        let mut keys: Vec<&str> = payload
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "aud",
                "company_id",
                "contract_version",
                "decisions_waiting",
                "exec_ready",
                "iss",
                "kind",
                "last_activity_at",
                "outcomes_last_day",
                "people_working",
                "plane_id",
                "projected_at",
                "sequence"
            ],
            "the record carries exactly the contract fields and nothing that could hold content"
        );
        assert_eq!(payload["decisions_waiting"], 4);
        assert_eq!(payload["people_working"], 2);
        assert_eq!(payload["outcomes_last_day"], 3);
        assert_eq!(payload["exec_ready"], true);
        assert_eq!(payload["contract_version"], 2);
        assert_eq!(payload["kind"], "attention");
        let header: serde_json::Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[0])
                .unwrap(),
        )
        .unwrap();
        assert_eq!(header["typ"], TOKEN_TYPE);
        assert_eq!(header["kid"], signer.key_id());
    }

    #[test]
    fn the_published_key_is_public_only_and_names_the_signer() {
        let signer = ProjectionSigner::for_test([6u8; 32]);
        let jwks = serde_json::to_value(signer.jwks()).unwrap();
        let key = &jwks["keys"][0];
        assert_eq!(jwks["keys"].as_array().unwrap().len(), 1);
        assert_eq!(key["kty"], "OKP");
        assert_eq!(key["crv"], "Ed25519");
        assert_eq!(key["alg"], "EdDSA");
        assert_eq!(key["use"], "sig");
        assert_eq!(key["kid"], signer.key_id());
        assert!(
            key.get("d").is_none(),
            "the private seed is never published"
        );
        assert_eq!(key.as_object().unwrap().len(), 6);
    }

    #[test]
    fn the_key_is_created_once_and_reused() {
        let root = std::env::temp_dir().join(format!("restless-projection-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let first = ProjectionSigner::open(&root).unwrap();
        let second = ProjectionSigner::open(&root).unwrap();
        assert_eq!(first.key_id(), second.key_id());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = fs::metadata(root.join(KEY_FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(
                mode & 0o077,
                0,
                "the signing key is readable by this user only"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn only_counts_a_time_and_readiness_leave_the_summary() {
        let now = Utc::now();
        let early = now - chrono::Duration::hours(30);
        let recent = now - chrono::Duration::hours(3);
        let late = now - chrono::Duration::minutes(2);
        // Completed two days ago, completed recently, and still active.
        let summary =
            summarize_parts(3, [(early, true), (recent, true), (late, false)], 2, true, now);
        assert_eq!(summary.decisions_waiting, 3);
        assert_eq!(summary.last_activity_at, Some(late));
        assert_eq!(
            summary.outcomes_last_day, 1,
            "only completions within the day count"
        );
        assert_eq!(summary.people_working, 2);
        assert!(summary.exec_ready);
        let nothing = summarize_parts(0, [], 0, false, now);
        assert_eq!(nothing.decisions_waiting, 0);
        assert_eq!(nothing.last_activity_at, None);
        assert_eq!(nothing.outcomes_last_day, 0);
        assert!(!nothing.exec_ready);
        let huge = summarize_parts(usize::MAX, [], usize::MAX, true, now);
        assert_eq!(huge.decisions_waiting, MAX_COUNT);
        assert_eq!(huge.people_working, MAX_COUNT);
    }

    #[test]
    fn hosted_company_handles_map_to_fleet_company_ids_and_nothing_else_does() {
        let id = Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
        let handle = crate::company_bootstrap::company_handle(id);
        assert_eq!(company_id_from_handle(&handle), Some(id));
        assert_eq!(company_id_from_handle("acme"), None);
        assert_eq!(company_id_from_handle("company_not-a-uuid"), None);
        assert_eq!(
            company_id_from_handle(&format!("company_{}", id.hyphenated())),
            None,
            "only the one canonical handle form is a Fleet company"
        );
        assert_eq!(company_id_from_handle(&handle.to_uppercase()), None);
    }

    #[test]
    fn a_record_goes_out_on_change_and_on_the_heartbeat_and_not_otherwise() {
        let now = Instant::now();
        let mut emission = Emission::default();
        assert!(
            emission.due(&summary(2), now),
            "the first look always reports"
        );
        emission.sent(summary(2), now);
        assert!(!emission.due(&summary(2), now + Duration::from_secs(30)));
        assert!(
            emission.due(&summary(3), now + Duration::from_secs(30)),
            "a change goes at once"
        );
        assert!(
            emission.due(&summary(2), now + HEARTBEAT),
            "an unchanged count still beats"
        );
    }

    #[test]
    fn a_switched_off_company_is_only_probed_rarely_and_failures_back_off() {
        let now = Instant::now();
        let mut emission = Emission::default();
        emission.switched_off(summary(5), now);
        assert!(!emission.due(&summary(9), now + Duration::from_secs(300)));
        assert!(emission.due(&summary(9), now + DISABLED_PROBE));

        let mut failing = Emission::default();
        failing.failed(now);
        assert!(!failing.due(&summary(1), now + Duration::from_secs(5)));
        assert!(failing.due(&summary(1), now + FAILURE_BACKOFF));
    }

    #[test]
    fn sequences_only_advance() {
        let mut emission = Emission::default();
        let now = Utc::now();
        let first = emission.next_sequence(now);
        let second = emission.next_sequence(now);
        let behind = emission.next_sequence(now - chrono::Duration::seconds(30));
        assert!(second > first);
        assert!(
            behind > second,
            "a clock that steps back cannot repeat a sequence"
        );
    }

    #[test]
    fn the_emitter_needs_network_mode_and_a_safe_fleet_url() {
        // Local mode has no plane identity and so never emits.
        assert!(ProjectionEmitter::from_environment(
            &EntryMode::Local,
            ProjectionSigner::for_test([1u8; 32])
        )
        .unwrap()
        .is_none());
    }

    /// The wire contract, pinned. `contracts/company-projection.v2.fixture.json` is byte-identical in
    /// Core and in Cloud: Core proves it still signs exactly this record, and Cloud proves it still
    /// accepts it, so neither side can drift without a test failing. The v1 fixture stays beside it
    /// for Cloud, which keeps accepting v1 from planes not yet upgraded. Regenerate with
    /// `RESTLESS_WRITE_PROJECTION_FIXTURE=1 cargo test -p restless-owner projection_fixture`.
    #[test]
    fn projection_fixture_is_the_pinned_wire_contract() {
        let signer = ProjectionSigner::for_test([5u8; 32]);
        let claims = AttentionClaims {
            iss: "https://owner.example.test".into(),
            aud: AUDIENCE.into(),
            contract_version: CONTRACT_VERSION,
            kind: KIND_ATTENTION.into(),
            plane_id: Uuid::from_u128(1),
            company_id: Uuid::from_u128(0xabc),
            projected_at: 1_790_000_000,
            sequence: 1_790_000_000_000,
            decisions_waiting: 4,
            last_activity_at: Some(1_789_999_900),
            people_working: 2,
            outcomes_last_day: 3,
            exec_ready: true,
        };
        let fixture = serde_json::json!({
            "contract": "restless-company-projection",
            "version": 2,
            "verify_at": 1_790_000_010,
            "hostname": "owner.example.test",
            "plane_id": claims.plane_id,
            "company_id": claims.company_id,
            "expect": {
                "decisions_waiting": 4,
                "sequence": 1_790_000_000_000i64,
                "people_working": 2,
                "outcomes_last_day": 3,
                "exec_ready": true
            },
            "jwks": signer.jwks(),
            "token": signer.sign(&claims).unwrap(),
        });
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts/company-projection.v2.fixture.json");
        let rendered = serde_json::to_string_pretty(&fixture).unwrap()
            + "
";
        if std::env::var("RESTLESS_WRITE_PROJECTION_FIXTURE").is_ok() {
            fs::write(&path, &rendered).unwrap();
        }
        assert_eq!(
            fs::read_to_string(&path).unwrap_or_default(),
            rendered,
            "the signed record no longer matches the pinned contract fixture"
        );
    }
}
