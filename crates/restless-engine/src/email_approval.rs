//! The owner approves one exact email (S109 H2).
//!
//! Without a mandate permit, `restless email send` does not fail: the host records the exact
//! prepared email as a request and the owner sees it in the Inbox (From, To, Subject, the text).
//! Send, Decline and Ask for changes wait out Authority's Undo window like every other Inbox
//! approval (`approval_decisions`). Once approved, the requester runs the same send again and the
//! host sends exactly that email once: the sender, recipient, digest and effect key must all match
//! the approved request. A changed draft is a new request and asks again.
use anyhow::{bail, Context as _, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::authority::{AuthorityRecord, AuthorityStore};
use crate::email::{EmailSendRequest, PreparedEmail};

pub const REQUEST_KIND: &str = "email_send_request";
pub const APPROVED_KIND: &str = "email_send_approved";
pub const DECLINED_KIND: &str = "email_send_declined";
pub const DECISION_KINDS: [&str; 2] = [APPROVED_KIND, DECLINED_KIND];

/// What the owner reads is bounded; the digest still covers the whole email.
const PREVIEW_CHARS: usize = 20_000;

/// One exact email waiting for the owner, as it is stored and shown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailSendRequestRecord {
    pub id: Uuid,
    pub effect_key: String,
    pub payload_sha256: String,
    pub sender: String,
    #[serde(default)]
    pub sender_name: Option<String>,
    pub recipient: String,
    pub subject: String,
    /// The text the recipient reads (the text part, or the HTML part without its tags).
    pub body: String,
    #[serde(default)]
    pub body_truncated: bool,
    #[serde(default)]
    pub attachments: Vec<String>,
    #[serde(default)]
    pub in_reply_to: Option<String>,
    #[serde(default)]
    pub work_id: Option<Uuid>,
}

impl EmailSendRequestRecord {
    /// The email as the owner reads it on the card: headers, a blank line, then the text.
    pub fn readable(&self) -> String {
        let from = match &self.sender_name {
            Some(name) => format!("{name} <{}>", self.sender),
            None => self.sender.clone(),
        };
        let mut text = format!(
            "From: {from}\nTo: {}\nSubject: {}\n",
            self.recipient, self.subject
        );
        if !self.attachments.is_empty() {
            text.push_str(&format!("Attached: {}\n", self.attachments.join(", ")));
        }
        if self.in_reply_to.is_some() {
            text.push_str("In reply to: their last email\n");
        }
        text.push('\n');
        text.push_str(self.body.trim_end());
        if self.body_truncated {
            text.push_str("\n\n[The rest of the email is not shown here.]");
        }
        text
    }
}

/// Where an email stands with the owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerApproval {
    /// Approved: send it now, once.
    Approved { request: Uuid },
    /// In the owner's Inbox. `new` when this call put it there.
    Waiting { request: Uuid, new: bool },
    /// Declined, or changes were asked for; this exact draft is never sent.
    Declined {
        request: Uuid,
        note: Option<String>,
        changes: bool,
    },
}

fn tagless(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    text.split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_string()
}

fn record_for(
    id: Uuid,
    prepared: &PreparedEmail,
    input: &EmailSendRequest,
    work_id: Option<Uuid>,
) -> EmailSendRequestRecord {
    let body = input
        .text
        .clone()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| input.html.as_deref().map(tagless).unwrap_or_default());
    let body_truncated = body.chars().count() > PREVIEW_CHARS;
    EmailSendRequestRecord {
        id,
        effect_key: prepared.effect_key().to_string(),
        payload_sha256: prepared.payload_sha256().to_string(),
        sender: prepared.sender().to_string(),
        sender_name: prepared.sender_name().map(str::to_string),
        recipient: prepared.recipient().to_string(),
        subject: input.subject.trim().to_string(),
        body: body.chars().take(PREVIEW_CHARS).collect(),
        body_truncated,
        attachments: input
            .attachments
            .iter()
            .map(|attachment| attachment.filename.clone())
            .collect(),
        in_reply_to: input.in_reply_to.clone(),
        work_id,
    }
}

fn parse(record: &AuthorityRecord) -> Option<EmailSendRequestRecord> {
    serde_json::from_value(record.body.clone()).ok()
}

fn same_email(record: &EmailSendRequestRecord, prepared: &PreparedEmail) -> bool {
    record.effect_key == prepared.effect_key()
        && record.payload_sha256 == prepared.payload_sha256()
        && record.sender == prepared.sender()
        && record.sender_name.as_deref() == prepared.sender_name()
        && record.recipient == prepared.recipient()
}

/// The owner's answer to one request, if any: (approved, note, changes requested).
async fn decision(
    authority: &AuthorityStore,
    company: &str,
    request: Uuid,
) -> Result<Option<(bool, Option<String>, bool)>> {
    let id = request.to_string();
    for (kind, approved) in [(DECLINED_KIND, false), (APPROVED_KIND, true)] {
        if let Some(body) = authority
            .find_body(company, kind, "request_id", &id)
            .await?
        {
            let note = body
                .get("owner_note")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            let changes = body
                .get("changes_requested")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            return Ok(Some((approved, note, changes)));
        }
    }
    Ok(None)
}

/// Whether an email under this effect key was already reserved for sending.
async fn consumed(authority: &AuthorityStore, company: &str, effect_key: &str) -> Result<bool> {
    Ok(authority
        .find_body(company, "email_send_reserved", "effect_key", effect_key)
        .await?
        .is_some())
}

/// Where this exact email stands with the owner, asking them when nobody has yet.
pub async fn approval_for(
    authority: &AuthorityStore,
    company: &str,
    actor: &str,
    prepared: &PreparedEmail,
    input: &EmailSendRequest,
    work_id: Option<Uuid>,
) -> Result<OwnerApproval> {
    if consumed(authority, company, prepared.effect_key()).await? {
        bail!(
            "an email with effect key {} was already sent or attempted; do not send it again. Reconcile with `restless email observe` if its outcome is unknown",
            prepared.effect_key()
        );
    }
    let newest = authority
        .records_of_kind(company, REQUEST_KIND)
        .await?
        .iter()
        .filter_map(parse)
        .filter(|record| record.effect_key == prepared.effect_key())
        .next_back();
    if let Some(record) = newest.filter(|record| same_email(record, prepared)) {
        return Ok(match decision(authority, company, record.id).await? {
            Some((true, _, _)) => OwnerApproval::Approved { request: record.id },
            Some((false, note, changes)) => OwnerApproval::Declined {
                request: record.id,
                note,
                changes,
            },
            None => OwnerApproval::Waiting {
                request: record.id,
                new: false,
            },
        });
    }
    // A new email, or a changed draft under the same key: the owner sees this version instead.
    let record = record_for(Uuid::new_v4(), prepared, input, work_id);
    authority
        .emit(
            company,
            REQUEST_KIND,
            Some(actor),
            serde_json::to_value(&record)?,
        )
        .await?;
    Ok(OwnerApproval::Waiting {
        request: record.id,
        new: true,
    })
}

/// The request record, if it is the one still waiting for the owner: the newest draft under its
/// effect key, unanswered and not yet sent.
pub async fn waiting_request(
    authority: &AuthorityStore,
    company: &str,
    request: Uuid,
) -> Result<Option<(AuthorityRecord, EmailSendRequestRecord)>> {
    let Some(found) = newest_drafts(authority, company)
        .await?
        .into_iter()
        .find(|(_, record)| record.id == request)
    else {
        return Ok(None);
    };
    if decision(authority, company, request).await?.is_some()
        || consumed(authority, company, &found.1.effect_key).await?
    {
        return Ok(None);
    }
    Ok(Some(found))
}

/// The newest draft under each effect key: an older draft under the same key is superseded.
async fn newest_drafts(
    authority: &AuthorityStore,
    company: &str,
) -> Result<Vec<(AuthorityRecord, EmailSendRequestRecord)>> {
    let mut newest: std::collections::BTreeMap<String, (AuthorityRecord, EmailSendRequestRecord)> =
        std::collections::BTreeMap::new();
    for stored in authority.records_of_kind(company, REQUEST_KIND).await? {
        if let Some(record) = parse(&stored) {
            newest.insert(record.effect_key.clone(), (stored, record));
        }
    }
    let mut drafts: Vec<_> = newest.into_values().collect();
    drafts.sort_by_key(|(stored, _)| stored.id);
    Ok(drafts)
}

/// Every email waiting for the owner, oldest first.
pub async fn waiting(
    authority: &AuthorityStore,
    company: &str,
) -> Result<Vec<(AuthorityRecord, EmailSendRequestRecord)>> {
    let mut waiting = Vec::new();
    for (stored, record) in newest_drafts(authority, company).await? {
        if decision(authority, company, record.id).await?.is_none()
            && !consumed(authority, company, &record.effect_key).await?
        {
            waiting.push((stored, record));
        }
    }
    Ok(waiting)
}

/// The owner's answer, once its Undo window has ended. `pending` names that Inbox decision.
#[allow(clippy::too_many_arguments)]
pub async fn decide(
    authority: &AuthorityStore,
    org: Option<&restless_orgintel::OrgIntel>,
    company: &str,
    request: Uuid,
    approve: bool,
    actor: &str,
    note: crate::approval::OwnerNote<'_>,
    pending: Option<Uuid>,
) -> Result<()> {
    if approve && note.changes_requested {
        bail!("asking for changes declines this email; it cannot also send it");
    }
    let (_, record) = waiting_request(authority, company, request)
        .await?
        .context("This email is no longer waiting.")?;
    let mut body = serde_json::json!({
        "request_id": request,
        "effect_key": record.effect_key,
        "payload_sha256": record.payload_sha256,
        "sender": record.sender,
        "recipient": record.recipient,
        "subject": record.subject,
        "principal": actor,
        "decided_at": Utc::now(),
    });
    note.record_into(&mut body)?;
    if let Some(pending) = pending {
        body["pending_decision_id"] = pending.to_string().into();
    }
    authority
        .emit(
            company,
            if approve {
                APPROVED_KIND
            } else {
                DECLINED_KIND
            },
            Some(actor),
            body,
        )
        .await?;
    crate::approval::announce_decisions(company, authority, org).await;
    Ok(())
}

/// Whether this person already gave this exact answer (a replay after a crash).
pub async fn already_answered(
    authority: &AuthorityStore,
    company: &str,
    request: Uuid,
    actor: &str,
    approve: bool,
) -> Result<bool> {
    Ok(authority
        .find_record(
            company,
            if approve {
                APPROVED_KIND
            } else {
                DECLINED_KIND
            },
            "request_id",
            &request.to_string(),
        )
        .await?
        .is_some_and(|record| record.actor_id.as_deref() == Some(actor)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_owner_reads_the_email_not_its_markup() {
        assert_eq!(
            tagless("<p>Hi Jo,</p>\n<p>Your <b>quote</b> &amp; plan.</p>"),
            "Hi Jo,\nYour quote & plan."
        );
        let record = EmailSendRequestRecord {
            id: Uuid::nil(),
            effect_key: "k".into(),
            payload_sha256: "0".repeat(64),
            sender: "hello@mail.renoco.com.au".into(),
            sender_name: Some("Reno Co".into()),
            recipient: "jo@example.com".into(),
            subject: "Your quote".into(),
            body: "Hi Jo".into(),
            body_truncated: false,
            attachments: vec!["quote.pdf".into()],
            in_reply_to: None,
            work_id: None,
        };
        assert_eq!(
            record.readable(),
            "From: Reno Co <hello@mail.renoco.com.au>\nTo: jo@example.com\nSubject: Your quote\nAttached: quote.pdf\n\nHi Jo"
        );
    }
}
