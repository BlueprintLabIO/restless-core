//! Email as one integration with one honest status (S109 H2, phase 1 status).
//!
//! The status is read from what Restless already holds, never guessed and never a live provider
//! call on read: whether the company has a Resend key (under any of its names), what Resend said
//! about the latest email or test the company sent, the sender it sends from, how many emails
//! wait for the owner, and the mandate's daily cap. Exec states facts about email only from this
//! (`restless capabilities`), and the owner sees the same words on Company › Integrations.
//!
//! States: `not_connected` (no key), `key_only` (a key, nothing sent yet, so nothing is known to
//! work), `ready` (Resend accepted the latest email), `failing` (Resend refused the latest email,
//! or its outcome is unknown). Domain checks wait on the owner's setup-key decision (open
//! question 1 in the H2 design) and are not claimed here.
use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::authority::AuthorityStore;
use crate::credential;
use crate::runtime::CompanyConfig;

/// An owner's test email, recorded beside the company's own sends.
pub const TEST_SEND_KIND: &str = "email_test_send";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LastSend {
    pub at: DateTime<Utc>,
    /// `accepted`, `refused` or `unknown`.
    pub outcome: &'static str,
    /// What Resend said, in the owner's words.
    pub detail: Option<String>,
    pub test: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EmailStatus {
    pub state: &'static str,
    /// The Vault name the key is under; `None` without one. Shown to the owner and admins only.
    pub key: Option<String>,
    pub sender: Option<String>,
    pub last_send: Option<LastSend>,
    /// Emails an agent wrote that wait for the owner's approval in the Inbox.
    pub waiting_for_owner: usize,
    pub mandate_daily: Option<u32>,
    pub sent_today: Option<u32>,
    /// One line: what to do next, and where.
    pub action: Option<String>,
}

/// What Resend's answer means for the owner.
fn refused_detail(detail: Option<&str>) -> String {
    let status = detail
        .and_then(|detail| detail.strip_prefix("Resend HTTP "))
        .and_then(|code| code.trim().parse::<u16>().ok());
    match status {
        Some(401 | 403) => format!(
            "Resend refused the key (HTTP {}). It may have been deleted, rotated or limited to another domain.",
            status.unwrap_or_default()
        ),
        Some(422) => "Resend refused the sender (HTTP 422). Its domain may not be verified in Resend yet.".into(),
        Some(429) => "Resend's sending limit was reached (HTTP 429).".into(),
        Some(code) => format!("Resend did not accept the email (HTTP {code})."),
        None => detail
            .map(str::to_string)
            .unwrap_or_else(|| "Resend did not accept the email.".into()),
    }
}

fn outcome_of(body: &Value) -> Option<&'static str> {
    match body.get("outcome").and_then(Value::as_str)? {
        "confirmed_sent" => Some("accepted"),
        "confirmed_not_sent" => Some("refused"),
        "unknown" => Some("unknown"),
        _ => None,
    }
}

/// The latest thing Resend said about an email this company sent, a test included.
async fn last_send(authority: &AuthorityStore, company: &str) -> Result<Option<LastSend>> {
    let mut latest: Option<(i64, LastSend)> = None;
    for (kind, test) in [("email_send_status", false), (TEST_SEND_KIND, true)] {
        if let Some(record) = authority.records_of_kind(company, kind).await?.pop() {
            let Some(outcome) = outcome_of(&record.body) else {
                continue;
            };
            let detail = record
                .body
                .get("provider_detail")
                .and_then(Value::as_str)
                .map(str::to_string);
            let send = LastSend {
                at: record.created_at,
                outcome,
                detail: match outcome {
                    "refused" => Some(refused_detail(detail.as_deref())),
                    "unknown" => Some(
                        "Nobody knows whether Resend accepted the last email. Reconcile it before sending more."
                            .into(),
                    ),
                    _ => None,
                },
                test,
            };
            if latest.as_ref().is_none_or(|(id, _)| record.id > *id) {
                latest = Some((record.id, send));
            }
        }
    }
    Ok(latest.map(|(_, send)| send))
}

/// The address the company sends from: its mandate's sender, else the latest one it used.
async fn sender(authority: &AuthorityStore, company: &str) -> Result<Option<String>> {
    if let Some(mandate) = authority
        .list_email_mandates(company)
        .await?
        .into_iter()
        .find(|mandate| mandate.expires_at > Utc::now())
    {
        return Ok(Some(mandate.sender));
    }
    for kind in [
        "email_send_reserved",
        crate::email_approval::REQUEST_KIND,
        TEST_SEND_KIND,
    ] {
        if let Some(record) = authority.records_of_kind(company, kind).await?.pop() {
            if let Some(sender) = record.body.get("sender").and_then(Value::as_str) {
                return Ok(Some(sender.to_string()));
            }
        }
    }
    Ok(None)
}

/// The key's Vault name, by the same rule sending uses (`credential::resolve_resend_send`).
pub fn key_name(config: &CompanyConfig, unbound: &[&str]) -> Option<String> {
    if config
        .credentials
        .contains_key(credential::RESEND_SEND_BINDING)
    {
        return Some(credential::RESEND_SEND_BINDING.to_string());
    }
    let bound = config.credentials.keys().map(String::as_str);
    bound
        .chain(unbound.iter().copied())
        .find(|name| credential::is_resend_send_name(name))
        .map(str::to_string)
}

/// The status, given the key's name as the caller found it.
pub async fn status(
    authority: &AuthorityStore,
    company: &str,
    key: Option<String>,
) -> Result<EmailStatus> {
    let last_send = last_send(authority, company).await?;
    let sender = sender(authority, company).await?;
    let waiting_for_owner = crate::email_approval::waiting(authority, company)
        .await
        .context("read emails waiting for the owner")?
        .len();
    let mandate = authority
        .list_email_mandates(company)
        .await?
        .into_iter()
        .find(|mandate| mandate.expires_at > Utc::now());
    let (mandate_daily, sent_today) = match &mandate {
        Some(mandate) => {
            let usage = authority.email_mandate_usage(company, mandate.id).await?;
            (Some(usage.limit_per_day), Some(usage.used_today))
        }
        None => (None, None),
    };
    let (state, action) = match (&key, &last_send) {
        (None, _) => (
            "not_connected",
            Some("Add the company's Resend key in Company › Integrations › Email".to_string()),
        ),
        (Some(_), Some(send)) if send.outcome != "accepted" => (
            "failing",
            Some(format!(
                "{} See Company › Integrations › Email",
                send.detail.clone().unwrap_or_default()
            )),
        ),
        (Some(_), Some(_)) => ("ready", None),
        (Some(_), None) => (
            "key_only",
            Some("Nothing has been sent yet, so it is not known to work. Send a test from Company › Integrations › Email".to_string()),
        ),
    };
    Ok(EmailStatus {
        state,
        key,
        sender,
        last_send,
        waiting_for_owner,
        mandate_daily,
        sent_today,
        action,
    })
}

/// The owner's test: one plain email from `from` to `to`, sent at once with the company's key.
/// It is the owner's own action, so it needs no mandate and no approval; what Resend answers is
/// recorded and becomes the status.
pub async fn send_test(
    root: &std::path::Path,
    authority: &AuthorityStore,
    company: &str,
    from: &str,
    to: &str,
    by: &str,
) -> Result<LastSend> {
    let config = CompanyConfig::load(root, company)?;
    let key = credential::resolve(&config, credential::RESEND_SEND_BINDING)
        .await
        .context("The company has no usable Resend key. Add it in the Vault first.")?;
    let input: crate::email::EmailSendRequest = serde_json::from_value(serde_json::json!({
        "from": from,
        "to": to,
        "subject": "A test email from Restless",
        "text": "This is a test from Company › Integrations › Email. If it reached your inbox, the company can send email.",
        "effect_key": format!("integration-test-{}", uuid::Uuid::new_v4()),
    }))?;
    let prepared = crate::email::prepare_email(input, Vec::new())?;
    let (outcome, provider_ref, detail) = match prepared.send(&key).await? {
        crate::email::EmailSendOutcome::Accepted { provider_id } => {
            ("confirmed_sent", Some(provider_id), None)
        }
        crate::email::EmailSendOutcome::Rejected { status } => (
            "confirmed_not_sent",
            None,
            Some(format!("Resend HTTP {status}")),
        ),
        crate::email::EmailSendOutcome::Unknown { reason } => ("unknown", None, Some(reason)),
    };
    authority
        .emit(
            company,
            TEST_SEND_KIND,
            Some(by),
            serde_json::json!({
                "sender": prepared.sender(),
                "recipient": prepared.recipient(),
                "outcome": outcome,
                "provider_ref": provider_ref,
                "provider_detail": detail,
            }),
        )
        .await?;
    last_send(authority, company)
        .await?
        .context("the test's outcome was not recorded")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resend_refusals_read_as_what_to_fix() {
        assert!(refused_detail(Some("Resend HTTP 401"))
            .starts_with("Resend refused the key (HTTP 401)"));
        assert!(refused_detail(Some("Resend HTTP 422")).contains("not be verified"));
        assert_eq!(
            refused_detail(Some("Resend HTTP 500")),
            "Resend did not accept the email (HTTP 500)."
        );
        assert_eq!(
            refused_detail(Some("Request preparation failed before contacting Resend")),
            "Request preparation failed before contacting Resend"
        );
    }

    /// The status says only what was observed: no key, a key nobody has tried, a refusal (and
    /// why), and a send Resend accepted.
    #[tokio::test]
    async fn the_status_is_what_resend_last_said() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping email status scenario");
            return;
        };
        let authority = AuthorityStore::connect(&url).await.unwrap();
        let company = format!("email_status_{}_test", uuid::Uuid::new_v4().simple());
        let key = || Some(credential::RESEND_SEND_BINDING.to_string());
        let none = status(&authority, &company, None).await.unwrap();
        assert_eq!(none.state, "not_connected");
        assert!(none.action.unwrap().contains("Resend key"));
        let untried = status(&authority, &company, key()).await.unwrap();
        assert_eq!(untried.state, "key_only");
        assert_eq!(untried.last_send, None);
        assert_eq!(untried.mandate_daily, None);
        let test = |outcome: &str, detail: Option<&str>| {
            serde_json::json!({
                "sender": "hello@mail.example.com", "recipient": "owner@example.com",
                "outcome": outcome, "provider_detail": detail,
            })
        };
        authority
            .emit(
                &company,
                TEST_SEND_KIND,
                Some("owner"),
                test("confirmed_not_sent", Some("Resend HTTP 401")),
            )
            .await
            .unwrap();
        let refused = status(&authority, &company, key()).await.unwrap();
        assert_eq!(refused.state, "failing");
        assert!(refused
            .action
            .unwrap()
            .starts_with("Resend refused the key (HTTP 401)"));
        assert_eq!(refused.sender.as_deref(), Some("hello@mail.example.com"));
        assert!(refused.last_send.unwrap().test);
        authority
            .emit(
                &company,
                TEST_SEND_KIND,
                Some("owner"),
                test("confirmed_sent", None),
            )
            .await
            .unwrap();
        let ready = status(&authority, &company, key()).await.unwrap();
        assert_eq!(ready.state, "ready");
        assert_eq!(ready.action, None);
        assert_eq!(ready.last_send.unwrap().outcome, "accepted");
        authority.delete_test_company(&company).await.unwrap();
    }
}
