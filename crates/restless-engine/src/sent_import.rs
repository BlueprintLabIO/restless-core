//! Sends made outside Restless, attached to the company so replies arrive with their context.
//!
//! An owner (or Exec) sometimes sends through Resend directly: a script, a one-off campaign. A
//! reply to such an email reached Exec as a stranger's message, with no idea what it answered.
//! `restless email import-sent <resend-id>...` attaches those sends. Each id is read back from
//! Resend with the company's own key (probe, never guess: an id that is not the company's own
//! sent email is refused), and becomes one `email_imported_send` Authority record. Importing the
//! same id again changes nothing.
//!
//! When a reply arrives, [`correspondence`] finds the latest email the company sent that person,
//! imported or sent by Exec, and the projection puts a short, bounded account of it beside the
//! reply ([`crate::inbound`]).
use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

pub const KIND: &str = "email_imported_send";
/// How much of what was sent is kept, and later shown beside a reply.
const EXCERPT_CHARS: usize = 1_500;
/// How many ids one import takes.
pub const MAX_IDS: usize = 50;

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS authority_email_imported_send_once \
         ON restless_authority.records (company, (body->>'provider_email_id')) \
         WHERE kind = 'email_imported_send'",
    )
    .execute(pool)
    .await
    .context("index imported sends")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Imported {
    pub resend_id: String,
    /// `imported`, `already_imported` or `refused`.
    pub outcome: &'static str,
    pub recipient: Option<String>,
    pub subject: Option<String>,
    pub sent_at: Option<DateTime<Utc>>,
    /// Why it was refused, in the owner's words.
    pub detail: Option<String>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 200
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Attach each sent email to the company. One refusal never stops the others.
pub async fn import(
    root: &std::path::Path,
    pool: &PgPool,
    company: &str,
    ids: &[String],
    by: &str,
) -> Result<Vec<Imported>> {
    if ids.is_empty() {
        bail!("Name at least one Resend email id.");
    }
    if ids.len() > MAX_IDS {
        bail!("Import at most {MAX_IDS} emails at a time.");
    }
    let config = crate::runtime::CompanyConfig::load(root, company)?;
    let key = crate::credential::resolve_resend_send(&config)
        .await
        .context("The company has no usable Resend key. Add it in Company › Vault first.")?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let mut results = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id.trim();
        results.push(
            match import_one(&client, &key, pool, company, id, by).await {
                Ok(imported) => imported,
                Err(error) => Imported {
                    resend_id: id.to_string(),
                    outcome: "refused",
                    recipient: None,
                    subject: None,
                    sent_at: None,
                    detail: Some(error.to_string()),
                },
            },
        );
    }
    Ok(results)
}

async fn import_one(
    client: &reqwest::Client,
    key: &str,
    pool: &PgPool,
    company: &str,
    id: &str,
    by: &str,
) -> Result<Imported> {
    if !valid_id(id) {
        bail!("{id:?} is not a Resend email id.");
    }
    let response = client
        .get(format!("{}/emails/{id}", crate::email::resend_api_base()))
        .bearer_auth(key.trim())
        .send()
        .await
        .map_err(|error| {
            anyhow::anyhow!("Resend could not be reached ({})", error.without_url())
        })?;
    let status = response.status().as_u16();
    match status {
        200..=299 => {}
        401 | 403 => bail!("The company's Resend key cannot read sent emails (HTTP {status}). Add a full-access key in Company › Vault."),
        404 => bail!("Resend has no sent email {id} on the company's account."),
        _ => bail!("Resend answered HTTP {status}."),
    }
    let bytes = response.bytes().await.context("read Resend's answer")?;
    if bytes.len() > 4 * 1024 * 1024 {
        bail!("Resend's answer for {id} was too large.");
    }
    let email: Value = serde_json::from_slice(&bytes).context("parse Resend's answer")?;
    let email = email
        .get("data")
        .filter(|data| data.is_object())
        .unwrap_or(&email);
    if email.get("id").and_then(Value::as_str) != Some(id) {
        bail!("Resend answered for a different email than {id}.");
    }
    let recipients: Vec<String> = match email.get("to") {
        Some(Value::String(one)) => vec![one.clone()],
        Some(Value::Array(many)) => many
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    };
    let recipient = recipients
        .first()
        .and_then(|recipient| crate::inbound::bare_address(recipient))
        .context("Resend's record has no recipient.")?;
    let sender = email
        .get("from")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let subject: String = email
        .get("subject")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .chars()
        .take(500)
        .collect();
    let sent_at = email
        .get("created_at")
        .and_then(Value::as_str)
        .and_then(parse_time);
    let text = email
        .get("text")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| email.get("html").and_then(Value::as_str).map(strip_html))
        .unwrap_or_default();
    let excerpt: String = text.trim().chars().take(EXCERPT_CHARS).collect();
    let inserted = sqlx::query_scalar::<_, i64>(
        "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id",
    )
    .bind(company)
    .bind(KIND)
    .bind(by)
    .bind(serde_json::json!({
        "provider": "resend",
        "provider_email_id": id,
        "sender": sender,
        "recipient": recipient,
        "recipients": recipients,
        "subject": subject,
        "sent_at": sent_at,
        "last_event": email.get("last_event"),
        "excerpt": excerpt,
        "imported_by": by,
    }))
    .fetch_optional(pool)
    .await
    .context("record the imported send")?;
    Ok(Imported {
        resend_id: id.to_string(),
        outcome: if inserted.is_some() {
            "imported"
        } else {
            "already_imported"
        },
        recipient: Some(recipient),
        subject: Some(subject),
        sent_at,
        detail: None,
    })
}

fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .or_else(|| DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f%#z").ok())
        .map(|time| time.with_timezone(&Utc))
}

/// Text from HTML, crudely: tags dropped, whitespace folded. Only ever an excerpt for context.
fn strip_html(html: &str) -> String {
    let mut text = String::with_capacity(html.len().min(16_384));
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(character),
            _ => {}
        }
        if text.len() > 16_384 {
            break;
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What the company last sent this person, as a short trusted paragraph for the projection, or
/// `None` when it never wrote to them. Bounded: one email, its subject and an excerpt.
pub async fn correspondence(
    pool: &PgPool,
    company: &str,
    counterpart: &str,
) -> Result<Option<String>> {
    let Some(counterpart) = crate::inbound::bare_address(counterpart) else {
        return Ok(None);
    };
    let latest = sqlx::query_as::<_, (String, Value, DateTime<Utc>)>(
        "SELECT kind, body, created_at FROM restless_authority.records \
         WHERE company=$1 AND kind IN ('email_imported_send','email_send_reserved') \
           AND lower(body->>'recipient')=$2 \
         ORDER BY COALESCE((body->>'sent_at')::timestamptz, created_at) DESC LIMIT 1",
    )
    .bind(company)
    .bind(&counterpart)
    .fetch_optional(pool)
    .await
    .context("read what the company sent this person")?;
    let Some((kind, body, created_at)) = latest else {
        return Ok(None);
    };
    let field = |name: &str| body.get(name).and_then(Value::as_str).unwrap_or_default();
    let when = body
        .get("sent_at")
        .and_then(Value::as_str)
        .and_then(parse_time)
        .unwrap_or(created_at)
        .format("%-d %b %Y %H:%M UTC");
    Ok(Some(if kind == KIND {
        format!(
            "The company emailed {counterpart} on {when}, from {} (sent outside Restless, attached by {}).\nSubject: {}\nWhat it said (excerpt):\n{}",
            field("sender"),
            field("imported_by"),
            field("subject"),
            field("excerpt"),
        )
    } else {
        format!(
            "Exec emailed {counterpart} on {when}, from {} (email effect {}).",
            field("sender"),
            field("effect_key"),
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_bounded_provider_ids() {
        assert!(valid_id("01a1a3ba-2b2c-702e-a44f-5ad19aea6ef4"));
        for bad in ["", "../emails", "a b", &"x".repeat(201)] {
            assert!(!valid_id(bad), "{bad}");
        }
    }

    /// An outside send is attached once, a wrong id is refused with a reason, and a reply from
    /// that person reaches the Exec with what it answers.
    #[tokio::test]
    async fn an_attached_send_gives_its_reply_context() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping the import scenario");
            return;
        };
        let _resend = crate::email::TEST_RESEND_BASE.lock().await;
        use axum::{extract::Path, routing::get, Json, Router};
        let sent_id = "01a1a3ba-2b2c-702e-a44f-5ad19aea6ef4";
        let reply_id = "9d0c7f3e-0000-4000-8000-000000000001";
        let app = Router::new()
            .route(
                "/emails/{id}",
                get(move |Path(id): Path<String>| async move {
                    if id != sent_id {
                        return (axum::http::StatusCode::NOT_FOUND, Json(serde_json::json!({})));
                    }
                    (axum::http::StatusCode::OK, Json(serde_json::json!({
                        "object": "email", "id": sent_id, "from": "Jerry, Reno Studio <hi@reno-test.example>",
                        "to": ["admin@aleon.example"], "subject": "Aleon Electrical's website",
                        "created_at": "2026-11-03 23:24:09.123+00", "last_event": "delivered",
                        "html": "<p>Hi Alex and Vicki,</p><p>You've got what people look for.</p>", "text": null
                    })))
                }),
            )
            .route(
                "/emails/receiving/{id}",
                get(move |Path(id): Path<String>| async move {
                    Json(serde_json::json!({
                        "id": id, "from": "Alex <admin@aleon.example>", "to": ["hi@reno-test.example"],
                        "subject": "Re: Aleon Electrical's website", "text": "How much would it cost?",
                        "headers": {}
                    }))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        // SAFETY: only Resend calls read these.
        unsafe {
            std::env::set_var("RESTLESS_TEST_RESEND_API_BASE", &base);
            std::env::set_var("IMPORT_TEST_RESEND_KEY", "re_import_test");
        }
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let company = format!("si{}_test", &suffix[..12]);
        let root = std::env::temp_dir().join(format!("restless-import-{suffix}"));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let mut config: crate::runtime::CompanyConfig =
            toml::from_str(&format!("name = '{company}'")).unwrap();
        config.credentials.insert(
            crate::credential::RESEND_SEND_BINDING.into(),
            "env:IMPORT_TEST_RESEND_KEY".into(),
        );
        crate::runtime::CompanyConfig::save(&root, &config).unwrap();
        let authority = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .unwrap();
        let pool = authority.pool().clone();

        let ids = vec![
            sent_id.to_string(),
            "00000000-0000-0000-0000-000000000000".to_string(),
        ];
        let first = import(&root, &pool, &company, &ids, "owner").await.unwrap();
        assert_eq!(first[0].outcome, "imported");
        assert_eq!(first[0].recipient.as_deref(), Some("admin@aleon.example"));
        assert_eq!(first[1].outcome, "refused");
        assert!(first[1]
            .detail
            .as_deref()
            .unwrap()
            .contains("no sent email"));
        let again = import(&root, &pool, &company, &ids[..1], "owner")
            .await
            .unwrap();
        assert_eq!(again[0].outcome, "already_imported");
        assert_eq!(
            authority
                .records_of_kind(&company, KIND)
                .await
                .unwrap()
                .len(),
            1
        );

        let context = correspondence(&pool, &company, "ALEX <Admin@Aleon.example>")
            .await
            .unwrap()
            .expect("the company wrote to Alex");
        assert!(context.contains("Aleon Electrical's website"));
        assert!(context.contains("You've got what people look for."));
        assert!(context.contains("3 Nov 2026"));
        assert!(correspondence(&pool, &company, "stranger@example.com")
            .await
            .unwrap()
            .is_none());

        // The reply, through the same receiver the webhook and the poll use.
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
            in_flight: std::sync::Arc::new(std::sync::Mutex::new(
                crate::schedule::WakeClaims::default(),
            )),
            schedule_wake: std::sync::Arc::new(tokio::sync::Notify::new()),
        };
        crate::inbound::accept_resend(
            &daemon,
            crate::ingress::InboundEvent {
                provider_event_id: "msg_reply".into(),
                company: company.clone(),
                body: serde_json::json!({"type": "email.received", "data": {
                    "email_id": reply_id, "from": "Alex <admin@aleon.example>",
                    "to": ["hi@reno-test.example"], "subject": "Re: Aleon Electrical's website"
                }}),
            },
            crate::inbound::Via::Webhook,
        )
        .await
        .unwrap();
        let org = daemon.orgintel.get(&company).await.unwrap();
        let inbox = org.inbox(Some("exec")).await.unwrap();
        assert_eq!(inbox.len(), 1, "the reply reaches the Exec once");
        let body = &inbox[0].body;
        assert!(body.contains("WHAT THE COMPANY SENT THIS PERSON"));
        assert!(body.contains("Aleon Electrical's website"));
        assert!(body.contains("How much would it cost?"));
        // Trusted context comes before, and outside, the sender's untrusted words.
        assert!(
            body.find("WHAT THE COMPANY SENT").unwrap()
                < body.find("BEGIN UNTRUSTED SENDER CONTENT").unwrap()
        );

        org.close().await;
        crate::cell::destroy_database(&root, &database_url, &company)
            .await
            .unwrap();
        authority.delete_test_company(&company).await.unwrap();
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn html_becomes_a_plain_excerpt() {
        assert_eq!(
            strip_html("<p>Hi Alex,</p>\n<ul><li>a local family team</li></ul>"),
            "Hi Alex, a local family team"
        );
    }
}
