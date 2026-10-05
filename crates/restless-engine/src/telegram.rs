//! Telegram as the owner's first out-of-browser Attention channel (ADR 0015).
//!
//! The owner brings a bot (BotFather) and stores only a credential reference.
//! They pair one private chat by sending the bot a one-time code from the
//! cockpit; the chat id lives here, in the account plane's Authority store,
//! never in a company container. Long polling (`getUpdates`) means a
//! self-hosted Core needs no public webhook.
//!
//! Attention stays the only definition of owner-worthy work. Each current
//! item is claimed once in `telegram_deliveries` before it is sent, so a
//! crash never sends it twice; a send that provably failed releases the claim
//! for the next pass. An approval button carries only a delivery id: the row
//! binds that button to one exact Authority record (or call key plus command
//! digest, or mandate proposal), and the decision runs through the same engine
//! functions as the cockpit's buttons, as the company's Authority owner.

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

use crate::attention::AttentionItem;
use crate::authority::AuthorityStore;

pub const API_BASE_ENV: &str = "RESTLESS_TELEGRAM_API_URL";
const DEFAULT_API_BASE: &str = "https://api.telegram.org";
const PAIRING_TTL_MINUTES: i64 = 30;
/// Bounds the burst when a chat is first paired against a long queue.
pub const MAX_SENDS_PER_PASS: usize = 10;
const MAX_TEXT_CHARS: usize = 3500;

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.telegram_links (\
           company TEXT PRIMARY KEY, token_reference TEXT NOT NULL, \
           bot_username TEXT NOT NULL, cockpit_origin TEXT NOT NULL, \
           started_by TEXT NOT NULL, pairing_code TEXT, code_expires_at TIMESTAMPTZ, \
           chat_id BIGINT, paired_at TIMESTAMPTZ, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now()\
         )",
    )
    .execute(pool)
    .await
    .context("create telegram links")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.telegram_deliveries (\
           id UUID PRIMARY KEY, \
           company TEXT NOT NULL REFERENCES restless_authority.telegram_links(company) ON DELETE CASCADE, \
           delivery_key TEXT NOT NULL, item_id TEXT NOT NULL, binding JSONB, \
           status TEXT NOT NULL CHECK (status IN ('sending','sent')), message_id BIGINT, \
           decided_at TIMESTAMPTZ, decision TEXT, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           UNIQUE (company, delivery_key)\
         )",
    )
    .execute(pool)
    .await
    .context("create telegram deliveries")?;
    Ok(())
}

/// One bot, addressed through its token. The token sits inside the URL, so
/// every transport error is stripped of its URL before it is reported.
#[derive(Clone)]
pub struct Bot {
    http: reqwest::Client,
    endpoint: String,
}

impl std::fmt::Debug for Bot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Bot([REDACTED])")
    }
}

/// Whether a failed call can have reached the chat. Telegram has no
/// idempotency key, so only a refusal or a connect failure proves "not sent".
#[derive(Debug)]
pub enum CallError {
    NotSent(String),
    Unknown(String),
}

impl std::fmt::Display for CallError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSent(detail) => write!(formatter, "Telegram refused the call: {detail}"),
            Self::Unknown(detail) => write!(formatter, "Telegram call outcome unknown: {detail}"),
        }
    }
}

impl std::error::Error for CallError {}

impl Bot {
    pub fn new(token: &str) -> Result<Self> {
        let base = std::env::var(API_BASE_ENV).unwrap_or_else(|_| DEFAULT_API_BASE.into());
        Self::with_base(&base, token)
    }

    pub fn with_base(base: &str, token: &str) -> Result<Self> {
        let token = token.trim();
        if token.is_empty()
            || token.len() > 128
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_'))
        {
            bail!("the referenced value is not a Telegram bot token");
        }
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(40))
                .build()?,
            endpoint: format!("{}/bot{token}", base.trim_end_matches('/')),
        })
    }

    async fn call(
        &self,
        method: &str,
        body: serde_json::Value,
    ) -> std::result::Result<serde_json::Value, CallError> {
        let response = self
            .http
            .post(format!("{}/{method}", self.endpoint))
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                let connect = error.is_connect();
                let detail = error.without_url().to_string();
                if connect {
                    CallError::NotSent(detail)
                } else {
                    CallError::Unknown(detail)
                }
            })?;
        let status = response.status();
        let reply: serde_json::Value = response
            .json()
            .await
            .map_err(|error| CallError::Unknown(error.without_url().to_string()))?;
        if !status.is_success() || reply["ok"] != true {
            let description = reply["description"].as_str().unwrap_or("no description");
            return Err(CallError::NotSent(format!("{status}: {description}")));
        }
        Ok(reply["result"].clone())
    }

    /// Live probe: the token is valid and names this bot.
    pub async fn username(&self) -> Result<String> {
        let me = self.call("getMe", serde_json::json!({})).await?;
        me["username"]
            .as_str()
            .map(str::to_string)
            .context("Telegram getMe returned no username")
    }

    async fn updates(&self, offset: i64, timeout_secs: u64) -> Result<Vec<serde_json::Value>> {
        let result = self
            .call(
                "getUpdates",
                serde_json::json!({
                    "offset": offset,
                    "timeout": timeout_secs,
                    "allowed_updates": ["message", "callback_query"],
                }),
            )
            .await?;
        Ok(result.as_array().cloned().unwrap_or_default())
    }

    async fn send(
        &self,
        chat_id: i64,
        text: &str,
        buttons: Option<Uuid>,
    ) -> std::result::Result<i64, CallError> {
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "link_preview_options": {"is_disabled": true},
        });
        if let Some(delivery) = buttons {
            let id = delivery.simple();
            body["reply_markup"] = serde_json::json!({"inline_keyboard": [[
                {"text": "Approve", "callback_data": format!("y:{id}")},
                {"text": "Decline", "callback_data": format!("n:{id}")},
            ]]});
        }
        let sent = self.call("sendMessage", body).await?;
        sent["message_id"]
            .as_i64()
            .ok_or_else(|| CallError::Unknown("sendMessage returned no message id".into()))
    }

    async fn answer_callback(&self, callback_id: &str, text: &str) {
        let text: String = text.chars().take(190).collect();
        if let Err(error) = self
            .call(
                "answerCallbackQuery",
                serde_json::json!({"callback_query_id": callback_id, "text": text}),
            )
            .await
        {
            tracing::debug!("Telegram callback answer failed: {error}");
        }
    }

    async fn clear_buttons(&self, chat_id: i64, message_id: i64) {
        if let Err(error) = self
            .call(
                "editMessageReplyMarkup",
                serde_json::json!({
                    "chat_id": chat_id,
                    "message_id": message_id,
                    "reply_markup": {"inline_keyboard": []},
                }),
            )
            .await
        {
            tracing::debug!("Telegram button removal failed: {error}");
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Link {
    pub company: String,
    pub token_reference: String,
    pub bot_username: String,
    #[serde(skip)]
    pub cockpit_origin: String,
    pub started_by: String,
    pub pairing_code: Option<String>,
    pub code_expires_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub chat_id: Option<i64>,
    pub paired_at: Option<DateTime<Utc>>,
}

const LINK_COLUMNS: &str = "company,token_reference,bot_username,cockpit_origin,started_by,\
     pairing_code,code_expires_at,chat_id,paired_at";

fn link_from(row: &sqlx::postgres::PgRow) -> Link {
    Link {
        company: row.get("company"),
        token_reference: row.get("token_reference"),
        bot_username: row.get("bot_username"),
        cockpit_origin: row.get("cockpit_origin"),
        started_by: row.get("started_by"),
        pairing_code: row.get("pairing_code"),
        code_expires_at: row.get("code_expires_at"),
        chat_id: row.get("chat_id"),
        paired_at: row.get("paired_at"),
    }
}

/// Validate the cockpit origin that deep links point back to.
pub fn validate_origin(origin: &str) -> Result<()> {
    let url = url::Url::parse(origin).context("cockpit origin is not a URL")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
    {
        bail!("cockpit origin must be a bare http(s) origin");
    }
    Ok(())
}

/// Start (or restart) pairing for one company: replaces any previous link and
/// its deliveries, and issues a fresh one-time code. An Authority-owner act;
/// the caller has checked the principal and live-probed the bot.
pub async fn start_pairing(
    pool: &PgPool,
    company: &str,
    token_reference: &str,
    bot_username: &str,
    cockpit_origin: &str,
    started_by: &str,
) -> Result<Link> {
    validate_origin(cockpit_origin)?;
    let code = Uuid::new_v4().simple().to_string()[..10].to_uppercase();
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM restless_authority.telegram_links WHERE company=$1")
        .bind(company)
        .execute(&mut *tx)
        .await?;
    let row = sqlx::query(&format!(
        "INSERT INTO restless_authority.telegram_links \
           (company,token_reference,bot_username,cockpit_origin,started_by,pairing_code,code_expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6, now() + make_interval(mins => $7)) RETURNING {LINK_COLUMNS}"
    ))
    .bind(company)
    .bind(token_reference)
    .bind(bot_username)
    .bind(cockpit_origin.trim_end_matches('/'))
    .bind(started_by)
    .bind(&code)
    .bind(PAIRING_TTL_MINUTES as i32)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(link_from(&row))
}

pub async fn link(pool: &PgPool, company: &str) -> Result<Option<Link>> {
    Ok(sqlx::query(&format!(
        "SELECT {LINK_COLUMNS} FROM restless_authority.telegram_links WHERE company=$1"
    ))
    .bind(company)
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(link_from))
}

pub async fn links(pool: &PgPool) -> Result<Vec<Link>> {
    Ok(sqlx::query(&format!(
        "SELECT {LINK_COLUMNS} FROM restless_authority.telegram_links ORDER BY company"
    ))
    .fetch_all(pool)
    .await?
    .iter()
    .map(link_from)
    .collect())
}

/// Forget the chat and every delivery; old buttons then answer nothing.
pub async fn unpair(pool: &PgPool, company: &str) -> Result<bool> {
    Ok(
        sqlx::query("DELETE FROM restless_authority.telegram_links WHERE company=$1")
            .bind(company)
            .execute(pool)
            .await?
            .rows_affected()
            > 0,
    )
}

/// What a button decides: one exact Authority question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Binding {
    Party {
        record: i64,
        party: String,
    },
    ToolCall {
        record: i64,
        call_key: String,
        command_digest: String,
    },
    EmailMandate {
        proposal: Uuid,
    },
}

/// One message ready for a paired chat.
#[derive(Debug, Clone)]
pub struct Outgoing {
    pub key: String,
    pub item_id: String,
    pub text: String,
    pub binding: Option<Binding>,
}

/// Compose an Attention item for Telegram. A new source reference (a newer
/// Authority record, a later handoff) is a new delivery; the same one never
/// is. Items still being briefed wait for their brief.
pub async fn outgoing(
    authority: &AuthorityStore,
    company: &str,
    company_display: &str,
    cockpit_origin: &str,
    item: &AttentionItem,
) -> Result<Option<Outgoing>> {
    if item.preparing {
        return Ok(None);
    }
    let mut digest = Sha256::new();
    for value in [
        item.id.as_str(),
        item.source.kind.as_str(),
        item.source.reference.as_str(),
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    let binding = match (item.source.plane, item.source.kind.as_str()) {
        ("authority", "approval_required") => {
            let record: i64 = item.source.reference.parse()?;
            match (&item.source.party, &item.source.call_key) {
                (Some(party), _) => Some(Binding::Party {
                    record,
                    party: party.clone(),
                }),
                (None, Some(call_key)) => {
                    let body = authority
                        .find_body(company, "approval_required", "call_key", call_key)
                        .await?
                        .context("prepared tool call disappeared")?;
                    let command_digest = body["command_digest"]
                        .as_str()
                        .context("prepared tool call has no command digest")?
                        .to_string();
                    Some(Binding::ToolCall {
                        record,
                        call_key: call_key.clone(),
                        command_digest,
                    })
                }
                (None, None) => None,
            }
        }
        ("authority", "email_mandate_proposal") => Some(Binding::EmailMandate {
            proposal: item.source.reference.parse()?,
        }),
        _ => None,
    };
    let query = if item.category == "review" {
        "review"
    } else {
        "item"
    };
    let link = format!(
        "{}/{}?{query}={}",
        cockpit_origin.trim_end_matches('/'),
        encode(company),
        encode(&item.id)
    );
    let happened: String = item.what_happened.chars().take(MAX_TEXT_CHARS).collect();
    Ok(Some(Outgoing {
        key: format!("{:x}", digest.finalize()),
        item_id: item.id.clone(),
        text: format!("{company_display}\n{}\n\n{happened}\n\n{link}", item.title)
            .chars()
            .take(4000)
            .collect(),
        binding,
    }))
}

fn encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Send what this chat has not been sent. Returns how many were sent.
pub async fn deliver(pool: &PgPool, bot: &Bot, link: &Link, items: Vec<Outgoing>) -> Result<usize> {
    let Some(chat_id) = link.chat_id else {
        return Ok(0);
    };
    let mut sent = 0;
    for item in items {
        if sent >= MAX_SENDS_PER_PASS {
            break;
        }
        let id = Uuid::new_v4();
        // Claim before send: a crash after this point can lose the Telegram
        // copy (the cockpit item remains), but can never send it twice.
        let claimed = sqlx::query(
            "INSERT INTO restless_authority.telegram_deliveries \
               (id,company,delivery_key,item_id,binding,status) VALUES ($1,$2,$3,$4,$5,'sending') \
             ON CONFLICT (company,delivery_key) DO NOTHING",
        )
        .bind(id)
        .bind(&link.company)
        .bind(&item.key)
        .bind(&item.item_id)
        .bind(
            item.binding
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?,
        )
        .execute(pool)
        .await?
        .rows_affected()
            == 1;
        if !claimed {
            continue;
        }
        match bot
            .send(chat_id, &item.text, item.binding.is_some().then_some(id))
            .await
        {
            Ok(message_id) => {
                sqlx::query(
                    "UPDATE restless_authority.telegram_deliveries \
                     SET status='sent', message_id=$2 WHERE id=$1",
                )
                .bind(id)
                .bind(message_id)
                .execute(pool)
                .await?;
                sent += 1;
            }
            Err(CallError::NotSent(detail)) => {
                sqlx::query("DELETE FROM restless_authority.telegram_deliveries WHERE id=$1")
                    .bind(id)
                    .execute(pool)
                    .await?;
                bail!("Telegram did not accept an Attention item: {detail}");
            }
            Err(error @ CallError::Unknown(_)) => {
                tracing::warn!(company = %link.company, item = %item.item_id, "{error}; not resending");
            }
        }
    }
    Ok(sent)
}

/// What the plane needs from outside this module to decide: the company's
/// current Authority owner and its OrgIntel handle (to announce decisions).
#[async_trait::async_trait]
pub trait Companies: Send + Sync {
    async fn authority_owner(&self, company: &str) -> Result<String>;
    async fn orgintel(&self, company: &str) -> Option<restless_orgintel::OrgIntel>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Decided(String),
    AlreadyDecided,
    NotPaired,
    NotOwner,
    Failed(String),
}

impl Answer {
    fn text(&self) -> String {
        match self {
            Self::Decided(message) => {
                let mut chars = message.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                    None => "Done.".into(),
                }
            }
            Self::AlreadyDecided => "Already decided.".into(),
            Self::NotPaired => "This chat is not paired for that company.".into(),
            Self::NotOwner => "Only the company's current Authority owner can decide this.".into(),
            Self::Failed(detail) => format!("Could not decide: {detail}"),
        }
    }
}

/// Decide one button press. Only the paired private chat, while its pairer
/// still holds Authority, can decide; each delivery decides at most once.
pub async fn decide(
    authority: &AuthorityStore,
    root: &Path,
    companies: &dyn Companies,
    delivery: Uuid,
    approve: bool,
    chat_id: i64,
    from_id: i64,
) -> Answer {
    match decide_inner(
        authority, root, companies, delivery, approve, chat_id, from_id,
    )
    .await
    {
        Ok(answer) => answer,
        Err(error) => Answer::Failed(format!("{error:#}")),
    }
}

async fn decide_inner(
    authority: &AuthorityStore,
    root: &Path,
    companies: &dyn Companies,
    delivery: Uuid,
    approve: bool,
    chat_id: i64,
    from_id: i64,
) -> Result<Answer> {
    let pool = authority.pool();
    let Some(row) = sqlx::query(
        "SELECT d.company, d.binding, l.chat_id, l.started_by \
         FROM restless_authority.telegram_deliveries d \
         JOIN restless_authority.telegram_links l USING (company) WHERE d.id=$1",
    )
    .bind(delivery)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(Answer::AlreadyDecided);
    };
    let company: String = row.get("company");
    let paired: Option<i64> = row.get("chat_id");
    if paired != Some(chat_id) || from_id != chat_id {
        return Ok(Answer::NotPaired);
    }
    let started_by: String = row.get("started_by");
    let principal = companies.authority_owner(&company).await?;
    if principal != started_by {
        return Ok(Answer::NotOwner);
    }
    let Some(binding) = row
        .get::<Option<serde_json::Value>, _>("binding")
        .map(serde_json::from_value::<Binding>)
        .transpose()?
    else {
        return Ok(Answer::AlreadyDecided);
    };
    let claimed = sqlx::query(
        "UPDATE restless_authority.telegram_deliveries SET decided_at=now(), decision=$2 \
         WHERE id=$1 AND decided_at IS NULL",
    )
    .bind(delivery)
    .bind(if approve { "approve" } else { "decline" })
    .execute(pool)
    .await?
    .rows_affected()
        == 1;
    if !claimed || !still_pending(authority, &company, &binding).await? {
        return Ok(Answer::AlreadyDecided);
    }
    let org = companies.orgintel(&company).await;
    let outcome = match &binding {
        Binding::Party { party, .. } if approve => {
            crate::approval::grant(root, &company, party, authority, org.as_ref(), &principal).await
        }
        Binding::Party { party, .. } => {
            crate::approval::decline(root, &company, party, authority, org.as_ref(), &principal)
                .await
        }
        Binding::ToolCall { call_key, .. } => {
            crate::effect::decide_tool_call(
                authority,
                org.as_ref(),
                &company,
                call_key,
                approve,
                &principal,
            )
            .await
        }
        Binding::EmailMandate { proposal } => authority
            .decide_email_mandate_proposal(&company, &principal, *proposal, approve, None)
            .await
            .map(|_| {
                if approve {
                    "approved the email mandate"
                } else {
                    "declined the email mandate"
                }
                .to_string()
            }),
    };
    match outcome {
        Ok(message) => Ok(Answer::Decided(message)),
        Err(error) => {
            // Nothing was decided; let the owner press again.
            sqlx::query(
                "UPDATE restless_authority.telegram_deliveries \
                 SET decided_at=NULL, decision=NULL WHERE id=$1",
            )
            .bind(delivery)
            .execute(pool)
            .await?;
            Ok(Answer::Failed(format!("{error:#}")))
        }
    }
}

/// The button's exact question is still open: not answered in the cockpit,
/// and (for a tool call) still the same prepared command.
async fn still_pending(
    authority: &AuthorityStore,
    company: &str,
    binding: &Binding,
) -> Result<bool> {
    Ok(match binding {
        Binding::Party { record, party } => {
            let party = party.trim().to_lowercase();
            if crate::approval::approved_parties(authority, company)
                .await?
                .contains(&party)
            {
                return Ok(false);
            }
            let mut answered = false;
            for kind in ["approval_granted", "approval_declined"] {
                answered |= authority
                    .records_of_kind(company, kind)
                    .await?
                    .iter()
                    .any(|event| {
                        event.id > *record
                            && event.body["party"]
                                .as_str()
                                .map(|p| p.trim().to_lowercase())
                                == Some(party.clone())
                    });
            }
            !answered
        }
        Binding::ToolCall {
            call_key,
            command_digest,
            ..
        } => {
            let current = authority
                .find_body(company, "approval_required", "call_key", call_key)
                .await?;
            current
                .as_ref()
                .and_then(|body| body["command_digest"].as_str())
                == Some(command_digest.as_str())
                && crate::effect::tool_call_decision(authority, company, call_key, command_digest)
                    .await?
                    .is_none()
        }
        Binding::EmailMandate { proposal } => authority
            .pending_email_mandate_proposals(company)
            .await?
            .iter()
            .any(|record| record.body["id"].as_str() == Some(proposal.to_string().as_str())),
    })
}

/// Pair the chat that sent a live code for a link on this bot.
async fn pair(
    pool: &PgPool,
    token_reference: &str,
    chat_id: i64,
    text: &str,
) -> Result<Option<String>> {
    let code = text.trim();
    let code = code
        .strip_prefix("/start")
        .unwrap_or(code)
        .trim()
        .to_uppercase();
    if code.len() != 10 || !code.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(None);
    }
    Ok(sqlx::query_scalar(
        "UPDATE restless_authority.telegram_links \
         SET chat_id=$3, paired_at=now(), pairing_code=NULL, code_expires_at=NULL \
         WHERE token_reference=$1 AND pairing_code=$2 AND code_expires_at > now() \
         RETURNING company",
    )
    .bind(token_reference)
    .bind(code)
    .bind(chat_id)
    .fetch_optional(pool)
    .await?)
}

/// One long-poll round for one bot: pairing codes and button presses.
/// `offset` is the next update id; Telegram redelivers anything not yet
/// acknowledged, and the delivery claim makes a redelivered press harmless.
pub async fn poll_once(
    authority: &AuthorityStore,
    root: &Path,
    companies: &dyn Companies,
    bot: &Bot,
    token_reference: &str,
    offset: &mut i64,
    timeout_secs: u64,
) -> Result<()> {
    for update in bot.updates(*offset, timeout_secs).await? {
        if let Some(id) = update["update_id"].as_i64() {
            *offset = (*offset).max(id + 1);
        }
        if let Some(message) = update.get("message") {
            let (Some(chat_id), Some(text)) =
                (message["chat"]["id"].as_i64(), message["text"].as_str())
            else {
                continue;
            };
            if message["chat"]["type"] != "private" {
                continue;
            }
            let reply = match pair(authority.pool(), token_reference, chat_id, text).await? {
                Some(company) => format!("Paired. Attention for {company} will arrive here."),
                None => "Send the pairing code shown in your cockpit.".to_string(),
            };
            if let Err(error) = bot.send(chat_id, &reply, None).await {
                tracing::debug!("Telegram pairing reply failed: {error}");
            }
        } else if let Some(callback) = update.get("callback_query") {
            let callback_id = callback["id"].as_str().unwrap_or_default();
            let chat_id = callback["message"]["chat"]["id"]
                .as_i64()
                .unwrap_or_default();
            let from_id = callback["from"]["id"].as_i64().unwrap_or_default();
            let data = callback["data"].as_str().unwrap_or_default();
            let parsed = data
                .split_once(':')
                .and_then(|(verb, id)| Some((verb, Uuid::try_parse(id).ok()?)));
            let answer = match parsed {
                Some(("y", delivery)) => {
                    decide(authority, root, companies, delivery, true, chat_id, from_id).await
                }
                Some(("n", delivery)) => {
                    decide(
                        authority, root, companies, delivery, false, chat_id, from_id,
                    )
                    .await
                }
                _ => Answer::AlreadyDecided,
            };
            bot.answer_callback(callback_id, &answer.text()).await;
            if matches!(answer, Answer::Decided(_) | Answer::AlreadyDecided) {
                if let Some(message_id) = callback["message"]["message_id"].as_i64() {
                    bot.clear_buttons(chat_id, message_id).await;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "telegram_tests.rs"]
mod tests;
