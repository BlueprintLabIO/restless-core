//! Incoming email (ADR 0017): Restless connects a company's Resend account so replies reach it.
//!
//! Three parts, all host-side with the company's own Resend key:
//!
//! - **Connect.** Once the company has a key that can see a verified domain, Restless registers the
//!   company's `email.received`, `email.bounced` and `email.complained` webhook at its door
//!   (`hooks.rs`), keeps the one-time signing secret in the company Vault as `resend.webhook`, and
//!   records the domains it receives for. The key the owner gave the company is the consent; there
//!   is nothing else to set up.
//! - **Poll.** A cursor over Resend's Receiving API, every two minutes, delivers whatever the
//!   webhook did not. Both paths land on one Authority record per email id
//!   ([`crate::inbound::dedupe_id`]), so nothing reaches Exec twice.
//! - **Health.** When the poll finds mail the webhook never delivered, when Resend disabled the
//!   webhook, or when deliveries fail, the owner gets one Inbox card with a way through
//!   ([`attention_item`]) and the same words in Company › Integrations › Email.
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

use crate::hooks::{self, Endpoint, Provider};
use crate::runtime::CompanyConfig;

/// The events the company's webhook asks for: replies, and the two lifecycle facts that change
/// what the company may send next.
pub const EVENTS: [&str; 3] = ["email.received", "email.bounced", "email.complained"];
/// How often the reconciler polls each connected company's Receiving API.
pub const POLL_EVERY: Duration = Duration::from_secs(120);
/// How often a company without a working webhook is connected again.
const CONNECT_EVERY: chrono::TimeDelta = chrono::TimeDelta::minutes(10);
/// How often a webhook that looks healthy is checked with Resend.
const VERIFY_EVERY: chrono::TimeDelta = chrono::TimeDelta::hours(1);
/// Mail younger than this is left to the webhook, so a normal delivery is never counted missed.
const WEBHOOK_GRACE: chrono::TimeDelta = chrono::TimeDelta::minutes(3);
/// How far back the first poll looks. Older mail is history; mail from before Restless connected
/// is delivered only when it answers someone the company wrote to.
const BACKFILL: chrono::TimeDelta = chrono::TimeDelta::days(7);
/// How long a missed delivery or a failure keeps the warning up.
const WARNING_WINDOW: chrono::TimeDelta = chrono::TimeDelta::hours(24);
/// The pages of Receiving API one poll reads at most (100 emails each).
const MAX_POLL_PAGES: usize = 5;
/// The subject the self-check email carries, followed by its nonce.
pub const CHECK_SUBJECT: &str = "Restless incoming email check";

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("create Resend client")
}

/// What one Resend API call came to. The key never appears in an error.
#[derive(Debug)]
enum Call {
    Ok(Value),
    /// The key cannot do this: a sending-only key, a deleted or rotated key.
    Forbidden(u16),
    NotFound,
    Failed(String),
}

async fn call(key: &str, method: reqwest::Method, path: &str, body: Option<Value>) -> Result<Call> {
    let mut request = client()?
        .request(method, format!("{}{path}", crate::email::resend_api_base()))
        .bearer_auth(key.trim());
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return Ok(Call::Failed(format!(
                "Resend could not be reached ({})",
                error.without_url()
            )))
        }
    };
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Ok(Call::Forbidden(status.as_u16()));
    }
    if status.as_u16() == 404 {
        return Ok(Call::NotFound);
    }
    if !status.is_success() {
        return Ok(Call::Failed(format!(
            "Resend answered HTTP {}",
            status.as_u16()
        )));
    }
    let bytes = response.bytes().await.context("read Resend response")?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Ok(Call::Failed("Resend's answer was too large".into()));
    }
    Ok(Call::Ok(if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).context("parse Resend response")?
    }))
}

// ----- status ---------------------------------------------------------------------------------

/// Incoming email, as the owner and Exec read it. Read from what Restless holds, never a live
/// provider call on read.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IncomingStatus {
    /// `not_connected` (no endpoint yet), `connecting` (being set up), `connected` (webhook and
    /// poll), `polling` (no public address: the poll alone, every two minutes), `failing`.
    pub state: &'static str,
    /// One line in the owner's words.
    pub detail: String,
    /// What to do, when there is something to do.
    pub action: Option<String>,
    pub domains: Vec<String>,
    pub registered_at: Option<DateTime<Utc>>,
    pub last_event_at: Option<DateTime<Utc>>,
    pub last_poll_at: Option<DateTime<Utc>>,
    pub check_sent_at: Option<DateTime<Utc>>,
    pub check_passed_at: Option<DateTime<Utc>>,
}

pub fn status(endpoint: Option<&Endpoint>, now: DateTime<Utc>) -> IncomingStatus {
    let Some(endpoint) = endpoint else {
        return IncomingStatus {
            state: "not_connected",
            detail: "Replies don't reach the company yet.".into(),
            action: Some("Restless connects incoming email once the company has a Resend key. Connect it from Company › Integrations › Email.".into()),
            domains: Vec::new(),
            registered_at: None,
            last_event_at: None,
            last_poll_at: None,
            check_sent_at: None,
            check_passed_at: None,
        };
    };
    let base = |state, detail: String, action: Option<String>| IncomingStatus {
        state,
        detail,
        action,
        domains: endpoint.receiving_domains.clone(),
        registered_at: endpoint.registered_at,
        last_event_at: endpoint.last_event_at,
        last_poll_at: endpoint.last_poll_at,
        check_sent_at: endpoint.check_sent_at,
        check_passed_at: endpoint.check_passed_at,
    };
    if let Some(problem) = problem(endpoint, now) {
        return base("failing", problem.detail, Some(problem.action));
    }
    if endpoint.registered_at.is_some() {
        return base(
            "connected",
            format!(
                "Replies to {} reach Exec as they arrive.",
                domains_text(&endpoint.receiving_domains)
            ),
            None,
        );
    }
    if endpoint.last_poll_at.is_some() && hooks::public_base().is_none() {
        return base(
            "polling",
            format!(
                "Restless checks {} for replies every two minutes; this install has no public address for an instant webhook.",
                domains_text(&endpoint.receiving_domains)
            ),
            None,
        );
    }
    base(
        "connecting",
        "Restless is connecting incoming email.".into(),
        None,
    )
}

fn domains_text(domains: &[String]) -> String {
    match domains {
        [] => "the company's addresses".into(),
        [one] => one.clone(),
        many => many.join(", "),
    }
}

struct Problem {
    /// Stable per kind of problem, so the Inbox card keeps its identity.
    key: &'static str,
    title: &'static str,
    detail: String,
    action: String,
}

/// The one thing wrong, if anything, in order of what to fix first.
fn problem(endpoint: &Endpoint, now: DateTime<Utc>) -> Option<Problem> {
    if let Some(error) = &endpoint.registration_error {
        return Some(Problem {
            key: "registration",
            title: "Incoming email isn't connected",
            detail: error.clone(),
            action: "Fix it in Company › Integrations › Email, then press Connect.".into(),
        });
    }
    if let Some(error) = &endpoint.poll_error {
        if endpoint.registered_at.is_none() {
            return Some(Problem {
                key: "poll",
                title: "Restless can't check the company's email",
                detail: error.clone(),
                action: "Check the Resend key in Company › Vault, then press Connect in Company › Integrations › Email.".into(),
            });
        }
    }
    let recent = |at: Option<DateTime<Utc>>| at.is_some_and(|at| now - at < WARNING_WINDOW);
    if endpoint.registered_at.is_some() && recent(endpoint.last_missed_at) {
        return Some(Problem {
            key: "missed",
            title: "Replies are arriving late",
            detail: format!(
                "Resend's webhook did not deliver {} recent {}; Restless found {} by checking instead, so {} reached Exec minutes late.",
                endpoint.missed,
                if endpoint.missed == 1 { "email" } else { "emails" },
                if endpoint.missed == 1 { "it" } else { "them" },
                if endpoint.missed == 1 { "it" } else { "they" },
            ),
            action: "Press Connect in Company › Integrations › Email to register the webhook again.".into(),
        });
    }
    if recent(endpoint.last_failure_at)
        && endpoint.last_event_at.is_none_or(|event| {
            endpoint
                .last_failure_at
                .is_some_and(|failure| failure > event)
        })
    {
        return Some(Problem {
            key: "failing",
            title: "Incoming email deliveries are failing",
            detail: endpoint
                .last_failure
                .clone()
                .unwrap_or_else(|| "The latest delivery was refused.".into()),
            action:
                "Press Connect in Company › Integrations › Email to register the webhook again."
                    .into(),
        });
    }
    None
}

/// What the owner's Inbox card says when incoming email needs them.
pub struct Warning {
    /// Stable per kind of problem, so the card keeps its identity while it stands.
    pub key: &'static str,
    pub title: &'static str,
    pub detail: String,
    pub action: String,
    pub at: DateTime<Utc>,
}

/// The warning, if incoming email needs the owner (the Inbox card is `attention`'s to draw).
pub async fn warning(company: &str, pool: &PgPool) -> Option<Warning> {
    let endpoint = hooks::endpoint(pool, company, Provider::Resend)
        .await
        .ok()??;
    let problem = problem(&endpoint, Utc::now())?;
    let at = [
        endpoint.last_missed_at,
        endpoint.last_failure_at,
        endpoint.last_connect_at,
        endpoint.last_poll_at,
        Some(endpoint.created_at),
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(endpoint.created_at);
    Some(Warning {
        key: problem.key,
        title: problem.title,
        detail: problem.detail,
        action: problem.action,
        at,
    })
}

// ----- connect --------------------------------------------------------------------------------

/// Connect (or reconnect) one company's incoming email. Safe to call again at any time: a working
/// webhook is kept, a missing or disabled one is registered again. The owner's Connect button
/// passes `force`, which replaces even a webhook that looks healthy (with a new signing secret)
/// and clears the record of missed deliveries. Returns the status it leaves.
pub async fn connect(daemon: &crate::Daemon, company: &str, force: bool) -> Result<IncomingStatus> {
    let pool = daemon.authority.pool();
    let config = CompanyConfig::load(&daemon.root, company)?;
    let key = match crate::credential::resolve_resend_send(&config).await {
        Ok(key) => key,
        Err(_) => {
            // No key, no endpoint: there is nothing to receive with yet.
            if hooks::endpoint(pool, company, Provider::Resend)
                .await?
                .is_some()
            {
                touch_connect(pool, company).await?;
                set_registration_error(pool, company, Some("The company's Resend key could not be read from the Vault. Add it in Company › Vault.")).await?;
                return Ok(status(reload(pool, company).await?.as_ref(), Utc::now()));
            }
            return Ok(status(None, Utc::now()));
        }
    };
    let mut endpoint = hooks::ensure_endpoint(pool, company, Provider::Resend).await?;
    touch_connect(pool, company).await?;
    if force {
        endpoint.registered_at = None;
        sqlx::query(
            "UPDATE restless_authority.hook_endpoints SET missed=0, last_missed_at=NULL \
             WHERE company=$1 AND provider=$2",
        )
        .bind(company)
        .bind(Provider::Resend.name())
        .execute(pool)
        .await
        .context("clear missed deliveries")?;
    }
    let outcome = connect_with_key(daemon, &config, &key, &endpoint).await;
    if let Err(error) = &outcome {
        tracing::warn!(company, "incoming email not connected: {error:#}");
    }
    set_registration_error(
        pool,
        company,
        outcome
            .as_ref()
            .err()
            .map(|error| error.to_string())
            .as_deref(),
    )
    .await?;
    Ok(status(reload(pool, company).await?.as_ref(), Utc::now()))
}

async fn touch_connect(pool: &PgPool, company: &str) -> Result<()> {
    sqlx::query(
        "UPDATE restless_authority.hook_endpoints SET last_connect_at=now() \
         WHERE company=$1 AND provider=$2",
    )
    .bind(company)
    .bind(Provider::Resend.name())
    .execute(pool)
    .await
    .context("record incoming email connect")?;
    Ok(())
}

async fn reload(pool: &PgPool, company: &str) -> Result<Option<Endpoint>> {
    hooks::endpoint(pool, company, Provider::Resend).await
}

async fn set_registration_error(pool: &PgPool, company: &str, error: Option<&str>) -> Result<()> {
    sqlx::query(
        "UPDATE restless_authority.hook_endpoints SET registration_error=$3 \
         WHERE company=$1 AND provider=$2",
    )
    .bind(company)
    .bind(Provider::Resend.name())
    .bind(error.map(|error| error.chars().take(300).collect::<String>()))
    .execute(pool)
    .await
    .context("record incoming email registration")?;
    Ok(())
}

/// Every refusal here is a sentence the owner can act on.
async fn connect_with_key(
    daemon: &crate::Daemon,
    config: &CompanyConfig,
    key: &str,
    endpoint: &Endpoint,
) -> Result<()> {
    let pool = daemon.authority.pool();
    let domains = receiving_domains(daemon, config, key).await?;
    sqlx::query(
        "UPDATE restless_authority.hook_endpoints SET receiving_domains=$3 \
         WHERE company=$1 AND provider=$2",
    )
    .bind(&config.name)
    .bind(Provider::Resend.name())
    .bind(serde_json::json!(domains))
    .execute(pool)
    .await
    .context("record receiving domains")?;
    let Some(base) = hooks::public_base() else {
        // A local install: nothing on the internet can call it, and the poll is the delivery.
        return Ok(());
    };
    let url = hooks::public_url(&base, Provider::Resend, &endpoint.token);
    if let Some(id) = &endpoint.provider_webhook_id {
        match call(key, reqwest::Method::GET, &format!("/webhooks/{id}"), None).await? {
            Call::Ok(webhook) => {
                let webhook = webhook
                    .get("data")
                    .filter(|data| data.is_object())
                    .unwrap_or(&webhook);
                let enabled = webhook
                    .get("status")
                    .and_then(Value::as_str)
                    .is_none_or(|status| status == "enabled");
                let same = webhook
                    .get("endpoint")
                    .and_then(Value::as_str)
                    .is_none_or(|endpoint| endpoint == url);
                if enabled && same && endpoint.registered_at.is_some() {
                    return Ok(());
                }
                // Disabled by Resend after failures, or pointing elsewhere: replace it.
                let _ = call(
                    key,
                    reqwest::Method::DELETE,
                    &format!("/webhooks/{id}"),
                    None,
                )
                .await;
            }
            Call::NotFound => {}
            Call::Forbidden(code) => bail!(forbidden(code)),
            Call::Failed(reason) => bail!("{reason}. Restless will try again in a few minutes."),
        }
    }
    let created = match call(
        key,
        reqwest::Method::POST,
        "/webhooks",
        Some(serde_json::json!({ "endpoint": url, "events": EVENTS })),
    )
    .await?
    {
        Call::Ok(created) => created,
        Call::Forbidden(code) => bail!(forbidden(code)),
        Call::NotFound => bail!("Resend has no webhooks API for this key's account."),
        Call::Failed(reason) => bail!(
            "{reason} when Restless registered the webhook. It will try again in a few minutes."
        ),
    };
    let mut created = created;
    if created.get("id").is_none() {
        created = created
            .get_mut("data")
            .map(Value::take)
            .unwrap_or(Value::Null);
    }
    let id = created
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 512)
        .context("Resend did not say which webhook it created")?
        .to_string();
    let secret = created
        .get_mut("signing_secret")
        .map(Value::take)
        .and_then(|value| value.as_str().map(str::to_owned))
        .filter(|secret| secret.starts_with("whsec_") && secret.len() <= 8 * 1024);
    let Some(secret) = secret else {
        let _ = call(
            key,
            reqwest::Method::DELETE,
            &format!("/webhooks/{id}"),
            None,
        )
        .await;
        bail!("Resend did not return the webhook's signing secret, so its deliveries could not be checked. Restless removed it and will try again.");
    };
    if let Err(error) = store_signing_secret(daemon, &config.name, &secret).await {
        let _ = call(
            key,
            reqwest::Method::DELETE,
            &format!("/webhooks/{id}"),
            None,
        )
        .await;
        return Err(error);
    }
    sqlx::query(
        "UPDATE restless_authority.hook_endpoints SET provider_webhook_id=$3, registered_at=now(), \
         last_failure_at=NULL, last_failure=NULL, last_missed_at=NULL, missed=0 \
         WHERE company=$1 AND provider=$2",
    )
    .bind(&config.name)
    .bind(Provider::Resend.name())
    .bind(&id)
    .execute(pool)
    .await
    .context("record registered webhook")?;
    tracing::info!(company = %config.name, "incoming email webhook registered");
    Ok(())
}

fn forbidden(code: u16) -> String {
    format!(
        "The company's Resend key can only send (HTTP {code}), so Restless can't read replies or register a webhook. Add a full-access Resend key in Company › Vault."
    )
}

/// The verified domains this company receives for: those its own addresses use when it has any,
/// else every verified domain on the account.
async fn receiving_domains(
    daemon: &crate::Daemon,
    config: &CompanyConfig,
    key: &str,
) -> Result<Vec<String>> {
    let listed = match call(key, reqwest::Method::GET, "/domains", None).await? {
        Call::Ok(listed) => listed,
        Call::Forbidden(code) => bail!(forbidden(code)),
        Call::NotFound => bail!("Resend has no domains for this key's account."),
        Call::Failed(reason) => bail!("{reason} when Restless read the company's domains. It will try again in a few minutes."),
    };
    let verified: Vec<String> = listed
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|domain| domain.get("status").and_then(Value::as_str) == Some("verified"))
        .filter(|domain| {
            // When Resend says whether a domain receives, believe it.
            domain
                .pointer("/capabilities/receiving")
                .and_then(Value::as_str)
                .is_none_or(|receiving| receiving == "enabled")
        })
        .filter_map(|domain| domain.get("name").and_then(Value::as_str))
        .map(str::to_ascii_lowercase)
        .collect();
    if verified.is_empty() {
        bail!("The Resend account has no verified domain that receives email. Verify the company's domain in Resend, with receiving on.");
    }
    let own = own_domains(daemon, &config.name).await.unwrap_or_default();
    let mine: Vec<String> = verified
        .iter()
        .filter(|domain| {
            own.iter()
                .any(|own| own == *domain || own.ends_with(&format!(".{domain}")))
        })
        .cloned()
        .collect();
    Ok(if mine.is_empty() { verified } else { mine })
}

/// The domains the company has sent from: its mandates' senders and its recorded sends.
async fn own_domains(daemon: &crate::Daemon, company: &str) -> Result<Vec<String>> {
    let senders: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT lower(body->>'sender') FROM restless_authority.records \
         WHERE company=$1 AND kind IN ('email_send_reserved','email_test_send','email_imported_send') \
           AND body ? 'sender'",
    )
    .bind(company)
    .fetch_all(daemon.authority.pool())
    .await
    .context("read the company's senders")?;
    let mut domains: Vec<String> = senders
        .iter()
        .chain(
            daemon
                .authority
                .list_email_mandates(company)
                .await
                .unwrap_or_default()
                .iter()
                .map(|mandate| &mandate.sender),
        )
        .filter_map(|sender| crate::inbound::address_domain(sender))
        .collect();
    domains.sort();
    domains.dedup();
    Ok(domains)
}

async fn store_signing_secret(daemon: &crate::Daemon, company: &str, secret: &str) -> Result<()> {
    let reference = format!(
        "infisical:/companies/{company}/{}",
        crate::credential::RESEND_WEBHOOK_BINDING
    );
    crate::credential::store_reference(&reference, secret)
        .await
        .context("The company Vault did not accept the webhook's signing secret")?;
    let mut config = CompanyConfig::load(&daemon.root, company)?;
    if config
        .credentials
        .get(crate::credential::RESEND_WEBHOOK_BINDING)
        != Some(&reference)
    {
        config
            .credentials
            .insert(crate::credential::RESEND_WEBHOOK_BINDING.into(), reference);
        CompanyConfig::save(&daemon.root, &config)?;
    }
    Ok(())
}

// ----- self-check -----------------------------------------------------------------------------

/// The owner's Test: one email from the company's domain to itself. It arrives the way a reply
/// would, so seeing it come back proves the whole path; it is never shown to Exec.
pub async fn send_check(daemon: &crate::Daemon, company: &str) -> Result<IncomingStatus> {
    let pool = daemon.authority.pool();
    let endpoint = hooks::endpoint(pool, company, Provider::Resend)
        .await?
        .context("Connect incoming email first.")?;
    let domain = endpoint.receiving_domains.first().context(
        "Restless doesn't know which domain receives the company's email yet. Press Connect first.",
    )?;
    let config = CompanyConfig::load(&daemon.root, company)?;
    let key = crate::credential::resolve_resend_send(&config)
        .await
        .context("The company has no usable Resend key. Add it in the Vault first.")?;
    let nonce = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
    let address = format!("restless-check@{domain}");
    let input: crate::email::EmailSendRequest = serde_json::from_value(serde_json::json!({
        "from": address,
        "to": address,
        "subject": format!("{CHECK_SUBJECT} {nonce}"),
        "text": "Restless sent this to itself to check that replies reach the company. Nobody needs to read it.",
        "effect_key": format!("incoming-check-{nonce}"),
    }))?;
    let prepared = crate::email::prepare_email(input, Vec::new())?;
    match prepared.send(&key).await? {
        crate::email::EmailSendOutcome::Accepted { .. } => {}
        crate::email::EmailSendOutcome::Rejected { status } => {
            bail!("Resend refused the test email (HTTP {status}). Check that {domain} is verified for sending.")
        }
        crate::email::EmailSendOutcome::Unknown { reason } => {
            bail!("Resend's answer to the test email is unknown: {reason}")
        }
    }
    sqlx::query(
        "UPDATE restless_authority.hook_endpoints SET check_nonce=$3, check_sent_at=now(), \
         check_passed_at=NULL WHERE company=$1 AND provider=$2",
    )
    .bind(company)
    .bind(Provider::Resend.name())
    .bind(&nonce)
    .execute(pool)
    .await
    .context("record incoming email check")?;
    Ok(status(reload(pool, company).await?.as_ref(), Utc::now()))
}

/// The nonce of a self-check email, when this subject is one.
pub fn check_nonce(subject: &str) -> Option<&str> {
    let rest = subject.split(CHECK_SUBJECT).nth(1)?.trim();
    (rest.len() == 12 && rest.bytes().all(|byte| byte.is_ascii_hexdigit())).then_some(rest)
}

/// Whether this received email is the company's own pending or passed self-check, which is
/// recorded but never shown to Exec.
pub async fn is_own_check(pool: &PgPool, company: &str, subject: &str) -> bool {
    let Some(nonce) = check_nonce(subject) else {
        return false;
    };
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.hook_endpoints \
         WHERE company=$1 AND provider=$2 AND check_nonce=$3)",
    )
    .bind(company)
    .bind(Provider::Resend.name())
    .bind(nonce)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

// ----- after each recorded event ----------------------------------------------------------------

/// What the record of one event says about the rail: a self-check came back, or the poll found
/// mail the webhook should have brought.
pub async fn after_record(
    daemon: &crate::Daemon,
    event: &crate::ingress::InboundEvent,
    endpoint: Option<&Endpoint>,
    via: crate::inbound::Via,
    inserted: bool,
) -> Result<()> {
    let Some(endpoint) = endpoint else {
        return Ok(());
    };
    let pool = daemon.authority.pool();
    let data = event.body.get("data").unwrap_or(&Value::Null);
    let subject = data
        .get("subject")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if let Some(nonce) = check_nonce(subject) {
        sqlx::query(
            "UPDATE restless_authority.hook_endpoints SET check_passed_at=COALESCE(check_passed_at, now()) \
             WHERE company=$1 AND provider=$2 AND check_nonce=$3",
        )
        .bind(&event.company)
        .bind(Provider::Resend.name())
        .bind(nonce)
        .execute(pool)
        .await
        .context("record incoming email check")?;
    }
    if via == crate::inbound::Via::Poll && inserted {
        let created = data
            .get("created_at")
            .and_then(Value::as_str)
            .and_then(parse_time);
        if let (Some(registered), Some(created)) = (endpoint.registered_at, created) {
            if created > registered {
                sqlx::query(
                    "UPDATE restless_authority.hook_endpoints SET missed=missed+1, last_missed_at=now() \
                     WHERE company=$1 AND provider=$2",
                )
                .bind(&event.company)
                .bind(Provider::Resend.name())
                .execute(pool)
                .await
                .context("record missed webhook delivery")?;
                tracing::warn!(company = %event.company, "poll found received email the webhook never delivered");
            }
        }
    }
    Ok(())
}

fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&Utc))
        // Resend writes `2026-11-04 01:02:03.456+00`.
        .or_else(|| {
            chrono::DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f%#z")
                .ok()
                .map(|time| time.with_timezone(&Utc))
        })
}

// ----- poll -------------------------------------------------------------------------------------

/// Read the company's received mail since the cursor and hand each new email to the same receiver
/// the webhook uses. Returns how many emails it recorded for the first time.
pub async fn poll(daemon: &crate::Daemon, company: &str) -> Result<usize> {
    let pool = daemon.authority.pool();
    let Some(endpoint) = hooks::endpoint(pool, company, Provider::Resend).await? else {
        return Ok(0);
    };
    let config = CompanyConfig::load(&daemon.root, company)?;
    let outcome = poll_with(daemon, &config, &endpoint).await;
    let (cursor, error) = match &outcome {
        Ok((_, cursor)) => (*cursor, None),
        Err(error) => (endpoint.poll_cursor, Some(error.to_string())),
    };
    sqlx::query(
        "UPDATE restless_authority.hook_endpoints SET last_poll_at=now(), poll_cursor=$3, poll_error=$4 \
         WHERE company=$1 AND provider=$2",
    )
    .bind(company)
    .bind(Provider::Resend.name())
    .bind(cursor)
    .bind(error.map(|error| error.chars().take(300).collect::<String>()))
    .execute(pool)
    .await
    .context("record incoming email poll")?;
    outcome.map(|(recorded, _)| recorded)
}

async fn poll_with(
    daemon: &crate::Daemon,
    config: &CompanyConfig,
    endpoint: &Endpoint,
) -> Result<(usize, Option<DateTime<Utc>>)> {
    let key = crate::credential::resolve_resend_send(config)
        .await
        .map_err(|_| {
            anyhow::anyhow!("The company's Resend key could not be read from the Vault.")
        })?;
    let now = Utc::now();
    let cursor = endpoint.poll_cursor.unwrap_or(now - BACKFILL);
    // With a webhook, the newest few minutes are the webhook's to deliver.
    let horizon = if endpoint.registered_at.is_some() {
        now - WEBHOOK_GRACE
    } else {
        now
    };
    let mut emails: Vec<(DateTime<Utc>, Value)> = Vec::new();
    let mut after: Option<String> = None;
    'pages: for _ in 0..MAX_POLL_PAGES {
        let path = match &after {
            Some(after) => format!("/emails/receiving?limit=100&after={after}"),
            None => "/emails/receiving?limit=100".to_string(),
        };
        let page = match call(&key, reqwest::Method::GET, &path, None).await? {
            Call::Ok(page) => page,
            Call::Forbidden(code) => bail!(forbidden(code)),
            Call::NotFound => bail!("Resend has no Receiving API for this key's account."),
            Call::Failed(reason) => bail!("{reason} when Restless checked for replies."),
        };
        let items = page
            .get("data")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for item in &items {
            let Some(created) = item
                .get("created_at")
                .and_then(Value::as_str)
                .and_then(parse_time)
            else {
                continue;
            };
            // The list is newest first; once past the cursor, every later page is older still.
            if created <= cursor {
                break 'pages;
            }
            emails.push((created, item.clone()));
        }
        if page.get("has_more").and_then(Value::as_bool) != Some(true) {
            break;
        }
        after = items
            .last()
            .and_then(|item| item.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        if after.is_none() {
            break;
        }
    }
    emails.sort_by_key(|(created, _)| *created);
    let correspondents = correspondents(daemon, &config.name)
        .await
        .unwrap_or_default();
    let mut recorded = 0;
    let mut advanced = endpoint.poll_cursor;
    for (created, item) in emails {
        if created > horizon {
            // Left for the webhook; the cursor stops here so the next poll looks again.
            break;
        }
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            advanced = Some(created);
            continue;
        };
        let from = item.get("from").and_then(Value::as_str).unwrap_or_default();
        let answers_us = crate::inbound::bare_address(from)
            .is_some_and(|from| correspondents.iter().any(|known| known == &from));
        // Mail from before Restless connected is history unless it answers the company.
        if created < endpoint.created_at && !answers_us {
            advanced = Some(created);
            continue;
        }
        let event = crate::ingress::InboundEvent {
            provider_event_id: format!("poll:{id}"),
            company: config.name.clone(),
            body: serde_json::json!({
                "type": "email.received",
                "created_at": item.get("created_at"),
                "data": {
                    "email_id": id,
                    "created_at": item.get("created_at"),
                    "from": item.get("from"),
                    "to": item.get("to"),
                    "cc": item.get("cc"),
                    "subject": item.get("subject"),
                },
            }),
        };
        match crate::inbound::accept_resend(daemon, event, crate::inbound::Via::Poll).await? {
            crate::inbound::Accepted::Recorded { inserted: true, .. } => recorded += 1,
            _ => {}
        }
        advanced = Some(created);
    }
    Ok((
        recorded,
        advanced.max(endpoint.poll_cursor).or(Some(cursor)),
    ))
}

/// Everyone the company has emailed: its own sends and sends attached to it.
pub async fn correspondents(daemon: &crate::Daemon, company: &str) -> Result<Vec<String>> {
    let recipients: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT lower(body->>'recipient') FROM restless_authority.records \
         WHERE company=$1 AND kind IN ('email_send_reserved','email_imported_send') \
           AND body ? 'recipient'",
    )
    .bind(company)
    .fetch_all(daemon.authority.pool())
    .await
    .context("read the company's correspondents")?;
    Ok(recipients
        .iter()
        .filter_map(|recipient| crate::inbound::bare_address(recipient))
        .collect())
}

// ----- the reconciler ---------------------------------------------------------------------------

/// Connect what can be connected and poll what is connected, forever, one company at a time.
/// Companies are independent failure domains: one broken key never delays another's mail.
pub async fn run(daemon: Arc<crate::Daemon>) {
    // Let the daemon's own startup settle first.
    tokio::time::sleep(Duration::from_secs(20)).await;
    loop {
        if let Err(error) = pass(&daemon).await {
            tracing::warn!("incoming email pass deferred: {error:#}");
        }
        tokio::time::sleep(POLL_EVERY).await;
    }
}

async fn pass(daemon: &crate::Daemon) -> Result<()> {
    let pool = daemon.authority.pool();
    let now = Utc::now();
    for company in live_companies(&daemon.root) {
        let endpoint = hooks::endpoint(pool, &company, Provider::Resend).await?;
        // Connect a company the first time it has a key; after that, keep its webhook registered
        // (or its domains current, without one), retrying at most every CONNECT_EVERY.
        let due = match &endpoint {
            None => true,
            Some(endpoint) => {
                // A healthy webhook is still looked at hourly: Resend disables one that kept
                // failing without telling anyone, and that must not wait for mail to go missing.
                let wait = if endpoint.registered_at.is_none()
                    || endpoint.registration_error.is_some()
                    || problem(endpoint, now).is_some()
                {
                    CONNECT_EVERY
                } else {
                    VERIFY_EVERY
                };
                endpoint.last_connect_at.is_none_or(|at| now - at >= wait)
            }
        };
        if due {
            let Ok(config) = CompanyConfig::load(&daemon.root, &company) else {
                continue;
            };
            // A company with no Resend key at all is not tried until it has one.
            if endpoint.is_none() && crate::email_integration::key_name(&config, &[]).is_none() {
                continue;
            }
            if let Err(error) = connect(daemon, &company, false).await {
                tracing::warn!(company, "incoming email connect deferred: {error:#}");
            }
        }
        if hooks::endpoint(pool, &company, Provider::Resend)
            .await?
            .is_some()
        {
            match poll(daemon, &company).await {
                Ok(0) => {}
                Ok(count) => tracing::info!(company, count, "incoming email poll recorded mail"),
                Err(error) => tracing::warn!(company, "incoming email poll deferred: {error:#}"),
            }
        }
    }
    Ok(())
}

fn live_companies(root: &std::path::Path) -> Vec<String> {
    let mut companies: Vec<String> = std::fs::read_dir(root.join("companies"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry
                .file_name()
                .to_string_lossy()
                .strip_suffix(".toml")?
                .to_string();
            crate::runtime::validate_company_name(&name)
                .ok()
                .map(|()| name)
        })
        .collect();
    companies.sort();
    companies
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint() -> Endpoint {
        let at = Utc::now() - chrono::TimeDelta::days(2);
        Endpoint {
            company: "reno_test".into(),
            provider: "resend".into(),
            token: hooks::mint_token(None),
            provider_webhook_id: Some("wh_1".into()),
            receiving_domains: vec!["renostudio.au".into()],
            registered_at: Some(at),
            registration_error: None,
            last_connect_at: None,
            last_event_at: None,
            events: 0,
            last_failure_at: None,
            last_failure: None,
            failures: 0,
            last_poll_at: Some(at),
            poll_cursor: None,
            poll_error: None,
            missed: 0,
            last_missed_at: None,
            check_nonce: None,
            check_sent_at: None,
            check_passed_at: None,
            created_at: at,
        }
    }

    #[test]
    fn the_status_says_connected_or_the_one_thing_to_fix() {
        let now = Utc::now();
        assert_eq!(status(None, now).state, "not_connected");
        let healthy = endpoint();
        let connected = status(Some(&healthy), now);
        assert_eq!(connected.state, "connected");
        assert!(connected.detail.contains("renostudio.au"));
        assert_eq!(connected.action, None);

        let mut missed = endpoint();
        missed.missed = 2;
        missed.last_missed_at = Some(now - chrono::TimeDelta::minutes(5));
        let late = status(Some(&missed), now);
        assert_eq!(late.state, "failing");
        assert!(late.detail.contains("2 recent emails"));
        assert!(late.action.unwrap().contains("Connect"));
        // A day later the warning has done its job.
        missed.last_missed_at = Some(now - chrono::TimeDelta::hours(30));
        assert_eq!(status(Some(&missed), now).state, "connected");

        let mut refused = endpoint();
        refused.last_failure_at = Some(now);
        refused.last_failure = Some("a delivery failed its signature check".into());
        assert_eq!(status(Some(&refused), now).state, "failing");
        // A good delivery after the failure clears it.
        refused.last_event_at = Some(now + chrono::TimeDelta::seconds(1));
        assert_eq!(status(Some(&refused), now).state, "connected");

        let mut sending_only = endpoint();
        sending_only.registered_at = None;
        sending_only.registration_error = Some(forbidden(401));
        let blocked = status(Some(&sending_only), now);
        assert_eq!(blocked.state, "failing");
        assert!(blocked.detail.contains("full-access Resend key"));
    }

    #[test]
    fn a_self_check_is_known_by_its_nonce_only() {
        assert_eq!(
            check_nonce(&format!("{CHECK_SUBJECT} 0123456789ab")),
            Some("0123456789ab")
        );
        assert_eq!(
            check_nonce(&format!("Re: {CHECK_SUBJECT} 0123456789ab")),
            Some("0123456789ab")
        );
        assert_eq!(check_nonce(CHECK_SUBJECT), None);
        assert_eq!(
            check_nonce(&format!("{CHECK_SUBJECT} please read this")),
            None
        );
        assert_eq!(check_nonce("Two things on your website"), None);
    }

    #[test]
    fn resend_times_parse_in_both_shapes() {
        assert!(parse_time("2026-11-04T01:02:03.456Z").is_some());
        assert!(parse_time("2026-11-04 01:02:03.456789+00").is_some());
        assert!(parse_time("yesterday").is_none());
    }

    /// A stand-in for Resend's domains and Receiving API.
    #[derive(Clone, Default)]
    struct MockResend {
        received: Arc<std::sync::Mutex<Vec<Value>>>,
    }

    impl MockResend {
        fn receive(&self, from: &str, subject: &str, created: DateTime<Utc>) -> String {
            let id = uuid::Uuid::new_v4().to_string();
            self.received.lock().unwrap().push(serde_json::json!({
                "id": id,
                "object": "email",
                "created_at": created.to_rfc3339(),
                "from": from,
                "to": ["hi@ie-test.example"],
                "subject": subject,
                "text": format!("body of {subject}"),
                "headers": {},
            }));
            id
        }

        async fn serve(&self) -> String {
            use axum::{extract::Path, routing::get, Json, Router};
            let list = self.clone();
            let one = self.clone();
            let app = Router::new()
                .route(
                    "/domains",
                    get(|| async {
                        Json(serde_json::json!({"data": [
                            {"name": "ie-test.example", "status": "verified"},
                            {"name": "pending.example", "status": "pending"}
                        ]}))
                    }),
                )
                .route(
                    "/emails/receiving",
                    get(move || {
                        let list = list.clone();
                        async move {
                            let mut data = list.received.lock().unwrap().clone();
                            data.reverse();
                            Json(serde_json::json!({"data": data, "has_more": false}))
                        }
                    }),
                )
                .route(
                    "/emails/receiving/{id}",
                    get(move |Path(id): Path<String>| {
                        let one = one.clone();
                        async move {
                            let found = one
                                .received
                                .lock()
                                .unwrap()
                                .iter()
                                .find(|email| email["id"] == id.as_str())
                                .cloned();
                            Json(found.unwrap_or(Value::Null))
                        }
                    }),
                );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            tokio::spawn(async move { axum::serve(listener, app).await.ok() });
            format!("http://{address}")
        }
    }

    /// The poll delivers what the webhook did not, once; mail from before Restless connected is
    /// delivered only when it answers someone the company wrote to; a poll that finds mail the
    /// webhook should have brought raises the warning; and each reply reaches the Exec's inbox
    /// once, waking it at once through the cell's message notification.
    #[tokio::test]
    async fn the_poll_fills_what_the_webhook_missed_without_duplicates() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping the incoming email poll scenario"
            );
            return;
        };
        let mock = MockResend::default();
        let base = mock.serve().await;
        // SAFETY: only Resend calls read these, and nothing else in this binary sets them.
        unsafe {
            std::env::set_var("RESTLESS_TEST_RESEND_API_BASE", &base);
            std::env::set_var("INCOMING_TEST_RESEND_KEY", "re_incoming_test");
        }
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let company = format!("ie{}_test", &suffix[..12]);
        let root = std::env::temp_dir().join(format!("restless-incoming-{suffix}"));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let mut config: CompanyConfig = toml::from_str(&format!("name = '{company}'")).unwrap();
        config.credentials.insert(
            crate::credential::RESEND_SEND_BINDING.into(),
            "env:INCOMING_TEST_RESEND_KEY".into(),
        );
        CompanyConfig::save(&root, &config).unwrap();
        let authority = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .unwrap();
        let daemon = crate::Daemon {
            root: root.clone(),
            capabilities: crate::capability::CapabilityIssuer::open(&root).unwrap(),
            spend: crate::spend::SpendLedger::open(&root).unwrap(),
            publication: crate::publication::PublicationManager::new(&root, authority.clone())
                .unwrap(),
            launch: crate::launch::LaunchBroker::new(&root).unwrap(),
            authority: authority.clone(),
            orgintel: crate::OrgIntelRegistry {
                database_url: database_url.clone(),
                root: root.clone(),
                handles: std::sync::Mutex::new(std::collections::HashMap::new()),
            },
            staff: crate::staff::StaffRegistry::default(),
            activities: crate::activity::AgentActivityStreams::default(),
            cell_wakes: crate::cell_wake::CellWakeHub::default(),
            runtime_bridges: crate::runtime_bridge::RuntimeBridgeRegistry::default(),
            lifecycle: restless_contracts::appliance::LifecycleGate::default(),
            in_flight: Arc::new(std::sync::Mutex::new(crate::schedule::WakeClaims::default())),
            schedule_wake: Arc::new(tokio::sync::Notify::new()),
        };
        let pool = authority.pool().clone();
        // The company once wrote to Alex.
        authority
            .emit(
                &company,
                "email_send_reserved",
                Some("exec"),
                serde_json::json!({"recipient": "admin@aleon.example", "sender": "hi@ie-test.example", "effect_key": "outreach-1"}),
            )
            .await
            .unwrap();
        let hour_ago = Utc::now() - chrono::TimeDelta::hours(1);
        let alex_early = mock.receive("Alex <admin@aleon.example>", "Re: your website", hour_ago);
        let stranger_early = mock.receive("spam@elsewhere.example", "Buy now", hour_ago);

        // Without a public address, connecting records the domains and the poll is the delivery.
        let connected = connect(&daemon, &company, false).await.unwrap();
        assert_eq!(connected.domains, vec!["ie-test.example".to_string()]);
        assert_eq!(connected.state, "connecting");

        let org = daemon.orgintel.get(&company).await.unwrap();
        let cell = daemon.orgintel.cell_database_url(&company).await.unwrap();
        let mut listener = sqlx::postgres::PgListener::connect(&cell).await.unwrap();
        listener.listen("restless_orgintel").await.unwrap();

        let started = std::time::Instant::now();
        assert_eq!(
            poll(&daemon, &company).await.unwrap(),
            1,
            "only Alex's early reply"
        );
        let wake = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let note = listener.recv().await.unwrap();
                let value: Value = serde_json::from_str(note.payload()).unwrap();
                if value["kind"] == "message" && value["body"]["to"] == "exec" {
                    return value;
                }
            }
        })
        .await
        .expect("the reply notifies the scheduler, which wakes the Exec");
        assert_eq!(wake["body"]["from"], "world");
        assert!(started.elapsed() < std::time::Duration::from_secs(5));

        let inbox = org.inbox(Some("exec")).await.unwrap();
        assert_eq!(inbox.len(), 1);
        assert!(inbox[0].body.contains(&alex_early));
        assert!(inbox[0].body.contains("body of Re: your website"));
        assert!(!inbox
            .iter()
            .any(|message| message.body.contains(&stranger_early)));

        // New mail after connecting: delivered whoever sent it, and once, even when the webhook
        // brings it again.
        let new = mock.receive(
            "new@customer.example",
            "A question",
            Utc::now() - chrono::TimeDelta::seconds(5),
        );
        assert_eq!(poll(&daemon, &company).await.unwrap(), 1);
        assert_eq!(
            poll(&daemon, &company).await.unwrap(),
            0,
            "the cursor moved on"
        );
        let pushed = crate::inbound::accept_resend(
            &daemon,
            crate::ingress::InboundEvent {
                provider_event_id: "msg_late".into(),
                company: company.clone(),
                body: serde_json::json!({"type": "email.received", "data": {"email_id": new, "to": ["hi@ie-test.example"], "subject": "A question"}}),
            },
            crate::inbound::Via::Webhook,
        )
        .await
        .unwrap();
        assert!(matches!(
            pushed,
            crate::inbound::Accepted::Recorded {
                inserted: false,
                ..
            }
        ));
        assert_eq!(org.inbox(Some("exec")).await.unwrap().len(), 2);
        assert!(warning(&company, &pool).await.is_none());

        // Once a webhook is registered, mail it should have brought but the poll found is late.
        sqlx::query(
            "UPDATE restless_authority.hook_endpoints SET registered_at=now() - interval '10 minutes', \
             poll_cursor=now() - interval '20 minutes', created_at=now() - interval '30 minutes', provider_webhook_id='wh_test' WHERE company=$1",
        )
        .bind(&company)
        .execute(&pool)
        .await
        .unwrap();
        mock.receive(
            "late@customer.example",
            "Still there?",
            Utc::now() - chrono::TimeDelta::minutes(5),
        );
        let fresh = mock.receive("fresh@customer.example", "Just now", Utc::now());
        assert_eq!(
            poll(&daemon, &company).await.unwrap(),
            1,
            "the newest is left to the webhook"
        );
        let late = warning(&company, &pool)
            .await
            .expect("missed mail is the owner's to know");
        assert_eq!(late.key, "missed");
        assert!(late.detail.contains("1 recent email"));
        let shown = status(
            hooks::endpoint(&pool, &company, Provider::Resend)
                .await
                .unwrap()
                .as_ref(),
            Utc::now(),
        );
        assert_eq!(shown.state, "failing");
        assert!(!org
            .inbox(Some("exec"))
            .await
            .unwrap()
            .iter()
            .any(|m| m.body.contains(&fresh)));

        org.close().await;
        crate::cell::destroy_database(&root, &daemon.orgintel.database_url, &company)
            .await
            .unwrap();
        sqlx::query("DELETE FROM restless_authority.hook_endpoints WHERE company=$1")
            .bind(&company)
            .execute(&pool)
            .await
            .unwrap();
        authority.delete_test_company(&company).await.unwrap();
        std::fs::remove_dir_all(&root).ok();
    }
}
