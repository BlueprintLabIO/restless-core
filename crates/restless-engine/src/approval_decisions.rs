//! The Undo window on an Inbox approval belongs to Authority, not a browser timer.
//!
//! An Approve or Decline from the Inbox is saved here first, as a pending
//! decision with its own deadline (`undo_until`, five seconds after it was
//! saved, by the database clock). Nothing is granted, declined or announced
//! until then: the approval request stays unanswered, so the gate that asked
//! keeps refusing and no agent is woken. Undo before the deadline cancels the
//! row. After it, the owner gateway's decision worker claims the row and makes
//! the ordinary Authority decision, rechecking the person's access and the
//! displayed draft first. The decision record names the pending row, so a
//! replay after a crash finds the answer it already gave instead of giving a
//! second one. This is not a generic request replay service.
use std::path::Path;

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use restless_orgintel::HumanDecisionPrincipal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::approval::{names_party, normalize_party, OwnerNote};
use crate::authority::{AuthorityRecord, AuthorityStore};

/// What the person answered: exactly one waiting request. Each kind keeps its own
/// decision path; this only holds the answer until its Undo window ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApprovalTarget {
    /// One party's first contact.
    Party { party: String },
    /// One prepared tool call.
    ToolCall { call_key: String },
    /// A proposed email mandate and its exact limits.
    EmailMandate { proposal: Uuid },
    /// One more payee for a payment allowance.
    Payee { proposal: i64 },
    /// A notice about a use of one of the owner's accounts: Fine (grant) or Freeze (decline).
    AccountNotice { account: String, notice: String },
    /// A company asking for one of the owner's accounts, approved as asked or denied.
    AccountRequest { request: String },
    /// A prepared publication: Authorise puts the exact build online; Decline stops it.
    Publication { publication: String },
    /// One exact email an agent asked to send (S109 H2): Send lets it go once; Decline stops it.
    EmailSend { request: Uuid },
}

impl ApprovalTarget {
    fn key(&self) -> String {
        match self {
            Self::Party { party } => format!("party:{}", normalize_party(party)),
            Self::ToolCall { call_key } => format!("call:{call_key}"),
            Self::EmailMandate { proposal } => format!("mandate:{proposal}"),
            Self::Payee { proposal } => format!("payee:{proposal}"),
            Self::AccountNotice { notice, .. } => format!("account-notice:{notice}"),
            Self::AccountRequest { request } => format!("account-request:{request}"),
            Self::Publication { publication } => format!("publication:{publication}"),
            Self::EmailSend { request } => format!("email-send:{request}"),
        }
    }

    fn normalized(&self) -> Self {
        match self {
            Self::Party { party } => Self::Party {
                party: normalize_party(party),
            },
            Self::ToolCall { call_key } => Self::ToolCall {
                call_key: call_key.trim().to_string(),
            },
            Self::AccountNotice { account, notice } => Self::AccountNotice {
                account: account.trim().to_string(),
                notice: notice.trim().to_string(),
            },
            Self::AccountRequest { request } => Self::AccountRequest {
                request: request.trim().to_string(),
            },
            Self::Publication { publication } => Self::Publication {
                publication: publication.trim().to_string(),
            },
            other => other.clone(),
        }
    }

    /// The Inbox item this answer comes from, exactly as the Inbox names it.
    pub fn item_id(&self) -> Option<String> {
        match self {
            Self::Party { .. } => None,
            Self::ToolCall { call_key } => Some(format!("authority:tool-call:{call_key}")),
            Self::EmailMandate { proposal } => {
                Some(format!("authority:email_mandate_proposal:{proposal}"))
            }
            Self::Payee { proposal } => Some(format!("authority:payee:{proposal}")),
            Self::AccountNotice { notice, .. } => {
                Some(format!("authority:account_notice:{notice}"))
            }
            Self::AccountRequest { request } => {
                Some(format!("authority:account_request:{request}"))
            }
            Self::Publication { publication } => {
                Some(format!("authority:publication:{publication}"))
            }
            Self::EmailSend { request } => Some(format!("authority:email_send:{request}")),
        }
    }

    fn empty(&self) -> bool {
        match self {
            Self::Party { party } => party.is_empty(),
            Self::ToolCall { call_key } => call_key.is_empty(),
            Self::EmailMandate { .. } | Self::Payee { .. } | Self::EmailSend { .. } => false,
            Self::AccountNotice { account, notice } => account.is_empty() || notice.is_empty(),
            Self::AccountRequest { request } => request.is_empty(),
            Self::Publication { publication } => publication.is_empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// `always`: with a tool call, let that tool act without asking from now on.
    Grant {
        #[serde(default)]
        always: bool,
    },
    Decline {
        #[serde(default)]
        note: Option<String>,
        #[serde(default)]
        changes_requested: bool,
    },
}

impl ApprovalDecision {
    fn note(&self) -> OwnerNote<'_> {
        match self {
            Self::Grant { .. } => OwnerNote::default(),
            Self::Decline {
                note,
                changes_requested,
            } => OwnerNote {
                text: note.as_deref(),
                changes_requested: *changes_requested,
            },
        }
    }

    /// The receipt's kind, which the Inbox notice reads.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Grant { always: false } => "approve",
            Self::Grant { always: true } => "approve_always",
            Self::Decline {
                changes_requested: true,
                ..
            } => "request_changes",
            Self::Decline { .. } => "decline",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingApprovalDecision {
    pub id: Uuid,
    pub company: String,
    pub item_id: String,
    pub principal: HumanDecisionPrincipal,
    pub target: ApprovalTarget,
    pub decision: ApprovalDecision,
    /// The digest of the draft the person was shown. A changed draft asks again.
    pub expected_digest: String,
    /// When the decision takes effect, unless it was undone before then.
    pub undo_until: DateTime<Utc>,
    pub state: String,
    pub problem: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub seen_at: Option<DateTime<Utc>>,
}

impl PendingApprovalDecision {
    /// The receipt's kind, which the Inbox notice reads.
    pub fn receipt_kind(&self) -> &'static str {
        match (&self.target, &self.decision) {
            (ApprovalTarget::AccountNotice { .. }, ApprovalDecision::Grant { .. }) => "fine",
            (ApprovalTarget::AccountNotice { .. }, ApprovalDecision::Decline { .. }) => "freeze",
            (ApprovalTarget::AccountRequest { .. }, ApprovalDecision::Decline { .. }) => "deny",
            (ApprovalTarget::Publication { .. }, ApprovalDecision::Grant { .. }) => "authorize",
            (ApprovalTarget::EmailSend { .. }, ApprovalDecision::Grant { .. }) => "send",
            (_, decision) => decision.kind(),
        }
    }
}

/// What a decision may touch once its window ends. Publications need the manager
/// that starts and stops their provider.
pub struct Effects<'a> {
    pub root: &'a Path,
    pub authority: &'a AuthorityStore,
    pub org: Option<&'a restless_orgintel::OrgIntel>,
    pub publication: Option<&'a crate::publication::PublicationManager>,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    company: String,
    item_id: String,
    principal: serde_json::Value,
    target: serde_json::Value,
    decision: serde_json::Value,
    expected_digest: String,
    undo_until: DateTime<Utc>,
    state: String,
    problem: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    seen_at: Option<DateTime<Utc>>,
}

impl TryFrom<Row> for PendingApprovalDecision {
    type Error = anyhow::Error;
    fn try_from(row: Row) -> Result<Self> {
        Ok(Self {
            id: row.id,
            company: row.company,
            item_id: row.item_id,
            principal: serde_json::from_value(row.principal).context("decision principal")?,
            target: serde_json::from_value(row.target).context("decision target")?,
            decision: serde_json::from_value(row.decision).context("approval decision")?,
            expected_digest: row.expected_digest,
            undo_until: row.undo_until,
            state: row.state,
            problem: row.problem,
            created_at: row.created_at,
            completed_at: row.completed_at,
            seen_at: row.seen_at,
        })
    }
}

const TABLE: &str = "restless_authority.owner_approval_decisions";
const COLUMNS: &str = "id,company,item_id,principal,target,decision,expected_digest,undo_until,\
                       state,problem,created_at,completed_at,seen_at";

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    for (name, sql) in [
        (
            "pending approval decisions",
            "CREATE TABLE IF NOT EXISTS restless_authority.owner_approval_decisions (\
               id UUID PRIMARY KEY, \
               company TEXT NOT NULL, \
               item_id TEXT NOT NULL CHECK (octet_length(item_id) BETWEEN 1 AND 512), \
               principal JSONB NOT NULL, \
               target JSONB NOT NULL, \
               target_key TEXT NOT NULL, \
               decision JSONB NOT NULL, \
               expected_digest TEXT NOT NULL, \
               undo_until TIMESTAMPTZ NOT NULL DEFAULT (clock_timestamp() + INTERVAL '5 seconds'), \
               state TEXT NOT NULL DEFAULT 'pending' \
                 CHECK (state IN ('pending','canceled','applied','failed')), \
               problem TEXT, \
               created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(), \
               completed_at TIMESTAMPTZ, \
               seen_at TIMESTAMPTZ\
             )",
        ),
        (
            "one pending approval decision per request",
            "CREATE UNIQUE INDEX IF NOT EXISTS one_pending_approval_decision \
             ON restless_authority.owner_approval_decisions (company, target_key) \
             WHERE state = 'pending'",
        ),
        (
            "due approval decisions",
            "CREATE INDEX IF NOT EXISTS due_approval_decisions \
             ON restless_authority.owner_approval_decisions (undo_until, id) \
             WHERE state = 'pending'",
        ),
        (
            "approval decision receipts by person",
            "CREATE INDEX IF NOT EXISTS approval_decisions_by_actor \
             ON restless_authority.owner_approval_decisions \
             (company, (principal->>'actor_id'), created_at)",
        ),
    ] {
        sqlx::query(sql)
            .execute(pool)
            .await
            .with_context(|| format!("create {name}"))?;
    }
    Ok(())
}

fn digest_of(request: &AuthorityRecord) -> String {
    request
        .body
        .get("command_digest")
        .or_else(|| request.body.get("draft_digest"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The request still waiting for this answer: the newest one, unanswered.
async fn waiting_request(
    authority: &AuthorityStore,
    company: &str,
    target: &ApprovalTarget,
) -> Result<Option<AuthorityRecord>> {
    match target {
        ApprovalTarget::ToolCall { call_key } => {
            let Some(request) = authority
                .find_record(company, "approval_required", "call_key", call_key)
                .await?
            else {
                return Ok(None);
            };
            Ok(
                crate::effect::held_call_waiting(authority, company, call_key, &request)
                    .await?
                    .then_some(request),
            )
        }
        ApprovalTarget::Party { party } => {
            let Some(newest) = authority
                .records_of_kind(company, "approval_required")
                .await?
                .into_iter()
                .filter(|request| request.body.get("call_key").is_none())
                .filter(|request| names_party(&request.body, party))
                .max_by_key(|request| request.id)
            else {
                return Ok(None);
            };
            if crate::approval::is_approved(authority, company, party).await? {
                return Ok(None);
            }
            let declined = authority
                .records_of_kind(company, "approval_declined")
                .await?
                .into_iter()
                .filter(|decline| names_party(&decline.body, party))
                .map(|decline| decline.id)
                .max();
            if declined.is_some_and(|declined| declined > newest.id) {
                return Ok(None);
            }
            Ok(Some(newest))
        }
        _ => Ok(None),
    }
}

const PUBLICATION_SETTLED: [&str; 4] = [
    "publication_authorized",
    "publication_resource_grant",
    "publication_failed",
    "publication_stopped",
];

/// Whether the request is still waiting, and the version of it the person was shown.
/// `None` once it was answered, withdrawn or expired.
async fn waiting_digest(
    authority: &AuthorityStore,
    company: &str,
    target: &ApprovalTarget,
) -> Result<Option<String>> {
    let pool = authority.pool();
    Ok(match target {
        ApprovalTarget::Party { .. } | ApprovalTarget::ToolCall { .. } => {
            waiting_request(authority, company, target)
                .await?
                .map(|request| digest_of(&request))
        }
        ApprovalTarget::EmailMandate { proposal } => {
            let proposal = proposal.to_string();
            authority
                .pending_email_mandate_proposals(company)
                .await?
                .into_iter()
                .filter(|record| record.body["id"].as_str() == Some(proposal.as_str()))
                .map(|record| record.id)
                .max()
                .map(|id| format!("record:{id}"))
        }
        ApprovalTarget::Payee { proposal } => {
            crate::finance::pending_payee_proposals(authority, company)
                .await?
                .iter()
                .any(|(record, _)| record.id == *proposal)
                .then(|| format!("record:{proposal}"))
        }
        ApprovalTarget::AccountNotice { account, notice } => {
            let row: Option<(String, i64)> = sqlx::query_as(
                "SELECT state, use_id FROM restless_authority.owner_account_notices \
                 WHERE id=$1 AND account_id=$2 AND company=$3",
            )
            .bind(notice)
            .bind(account)
            .bind(company)
            .fetch_optional(pool)
            .await?;
            row.filter(|(state, _)| state == "open")
                .map(|(_, used)| format!("use:{used}"))
        }
        ApprovalTarget::AccountRequest { request } => {
            let row: Option<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
                "SELECT state, account_id, kind, abilities::text \
                 FROM restless_authority.owner_account_requests WHERE id=$1 AND company=$2",
            )
            .bind(request)
            .bind(company)
            .fetch_optional(pool)
            .await?;
            row.filter(|(state, ..)| state == "open")
                .map(|(_, account, kind, abilities)| {
                    format!(
                        "account:{}|kind:{}|abilities:{abilities}",
                        account.unwrap_or_default(),
                        kind.unwrap_or_default()
                    )
                })
        }
        ApprovalTarget::EmailSend { request } => {
            crate::email_approval::waiting_request(authority, company, *request)
                .await?
                .map(|(stored, _)| format!("record:{}", stored.id))
        }
        ApprovalTarget::Publication { publication } => {
            for kind in PUBLICATION_SETTLED {
                if authority
                    .find_body(company, kind, "publication_id", publication)
                    .await?
                    .is_some()
                {
                    return Ok(None);
                }
            }
            let newest = authority
                .records_of_kind(company, "publication_request")
                .await?
                .into_iter()
                .filter(|record| {
                    record.body["publication_id"].as_str() == Some(publication.as_str())
                })
                .max_by_key(|record| record.id);
            newest
                .filter(|record| {
                    record.body["request"]["expires_at"]
                        .as_str()
                        .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
                        .is_none_or(|at| at.with_timezone(&Utc) > Utc::now())
                })
                .map(|record| format!("record:{}", record.id))
        }
    })
}

/// Whether this exact answer was already given by this person: a replay after a
/// crash between the decision and settling its row finds it instead of giving it
/// twice. Any other answer leaves the request no longer waiting, which the
/// recheck refuses.
async fn already_answered(
    authority: &AuthorityStore,
    company: &str,
    saved: &PendingApprovalDecision,
) -> Result<bool> {
    let pool = authority.pool();
    let grant = matches!(saved.decision, ApprovalDecision::Grant { .. });
    let actor = saved.principal.actor_id.as_str();
    Ok(match &saved.target {
        ApprovalTarget::Party { .. } | ApprovalTarget::ToolCall { .. } => false,
        ApprovalTarget::EmailMandate { proposal } => {
            let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
                "SELECT actor_id, body->>'decision' FROM restless_authority.records \
                 WHERE company=$1 AND kind='email_mandate_proposal_decision' \
                 AND body->>'proposal_id'=$2 ORDER BY id LIMIT 1",
            )
            .bind(company)
            .bind(proposal.to_string())
            .fetch_optional(pool)
            .await?;
            row.is_some_and(|(by, decision)| {
                by.as_deref() == Some(actor)
                    && decision.as_deref() == Some(if grant { "approve" } else { "decline" })
            })
        }
        ApprovalTarget::Payee { proposal } => {
            let row: Option<(Option<String>, Option<bool>)> = sqlx::query_as(
                "SELECT actor_id, (body->>'approve')::boolean FROM restless_authority.records \
                 WHERE company=$1 AND kind=$2 AND body->>'proposal'=$3 ORDER BY id LIMIT 1",
            )
            .bind(company)
            .bind(crate::finance::PAYEE_DECIDED_KIND)
            .bind(proposal.to_string())
            .fetch_optional(pool)
            .await?;
            row.is_some_and(|(by, approve)| by.as_deref() == Some(actor) && approve == Some(grant))
        }
        ApprovalTarget::AccountNotice { account, notice } => {
            let row: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT state, decided_by FROM restless_authority.owner_account_notices \
                 WHERE id=$1 AND account_id=$2 AND company=$3",
            )
            .bind(notice)
            .bind(account)
            .bind(company)
            .fetch_optional(pool)
            .await?;
            row.is_some_and(|(state, by)| {
                by.as_deref() == Some(actor) && state == if grant { "fine" } else { "frozen" }
            })
        }
        ApprovalTarget::AccountRequest { request } => {
            let state: Option<String> = sqlx::query_scalar(
                "SELECT state FROM restless_authority.owner_account_requests \
                 WHERE id=$1 AND company=$2",
            )
            .bind(request)
            .bind(company)
            .fetch_optional(pool)
            .await?;
            state.as_deref() == Some(if grant { "approved" } else { "denied" })
        }
        ApprovalTarget::EmailSend { request } => {
            crate::email_approval::already_answered(authority, company, *request, actor, grant)
                .await?
        }
        ApprovalTarget::Publication { publication } => authority
            .find_body(
                company,
                if grant {
                    "publication_authorized"
                } else {
                    "publication_stopped"
                },
                "publication_id",
                publication,
            )
            .await?
            .is_some(),
    })
}

fn valid_item_id(item_id: &str, target: &ApprovalTarget) -> bool {
    match target {
        ApprovalTarget::Party { party } => {
            item_id.starts_with("authority:approval:") && item_id.ends_with(&format!(":{party}"))
        }
        other => other.item_id().is_some_and(|expected| expected == item_id),
    }
}

async fn read(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    lock: bool,
) -> Result<Option<PendingApprovalDecision>> {
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM {TABLE} WHERE id=$1{}",
        if lock { " FOR UPDATE" } else { "" }
    ))
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(PendingApprovalDecision::try_from).transpose()
}

/// Save one exact answer before the Inbox card leaves. A replay of the same
/// request returns its original deadline; another client cannot extend it.
pub async fn queue(
    authority: &AuthorityStore,
    id: Uuid,
    company: &str,
    item_id: &str,
    principal: &HumanDecisionPrincipal,
    target: &ApprovalTarget,
    decision: &ApprovalDecision,
) -> Result<PendingApprovalDecision> {
    let target = target.normalized();
    match (&target, decision) {
        (ApprovalTarget::Party { party }, _) if party.is_empty() => {
            bail!("approval party cannot be empty")
        }
        (ApprovalTarget::ToolCall { call_key }, _) if call_key.is_empty() => {
            bail!("no prepared tool call is waiting under that key")
        }
        (target, _) if target.empty() => bail!("Open the current request before deciding."),
        (ApprovalTarget::ToolCall { .. }, _) => {}
        (_, ApprovalDecision::Grant { always: true }) => {
            bail!("only a tool call can be allowed from now on")
        }
        (ApprovalTarget::Party { .. }, _) => {}
        (
            ApprovalTarget::EmailMandate { .. } | ApprovalTarget::Payee { .. },
            ApprovalDecision::Decline {
                changes_requested: true,
                ..
            },
        ) => bail!("This is answered yes or no. Say what should change in a note with Decline."),
        (ApprovalTarget::EmailMandate { .. } | ApprovalTarget::Payee { .. }, _) => {}
        // An email is declined with a note, or sent back for changes like a prepared call.
        (ApprovalTarget::EmailSend { .. }, _) => {}
        (
            _,
            ApprovalDecision::Decline {
                note,
                changes_requested,
            },
        ) if note.is_some() || *changes_requested => {
            bail!("This answer carries no note.")
        }
        _ => {}
    }
    // The note is checked now, so a refused note never waits five seconds to fail.
    decision.note().record_into(&mut serde_json::json!({}))?;
    if !valid_item_id(item_id, &target) {
        bail!("Open the current request before deciding.");
    }
    let mut tx = authority.pool().begin().await?;
    if let Some(saved) = read(&mut tx, id, false).await? {
        if saved.company != company
            || saved.principal != *principal
            || saved.target != target
            || saved.decision != *decision
        {
            bail!("This decision ID already names a different request.");
        }
        tx.commit().await?;
        return Ok(saved);
    }
    let Some(digest) = waiting_digest(authority, company, &target).await? else {
        bail!("This request is no longer waiting.");
    };
    if let (ApprovalTarget::AccountRequest { request }, ApprovalDecision::Grant { .. }) =
        (&target, decision)
    {
        // Approving as asked needs the account it names; otherwise the owner chooses one.
        let named: Option<String> = sqlx::query_scalar(
            "SELECT account_id FROM restless_authority.owner_account_requests WHERE id=$1",
        )
        .bind(request)
        .fetch_one(&mut *tx)
        .await?;
        if named.is_none() {
            bail!("Choose which account to grant: approve it with limits.");
        }
    }
    let inserted: Option<Row> = sqlx::query_as(&format!(
        "INSERT INTO {TABLE} (id,company,item_id,principal,target,target_key,decision,expected_digest) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(company)
    .bind(item_id)
    .bind(serde_json::to_value(principal)?)
    .bind(serde_json::to_value(&target)?)
    .bind(target.key())
    .bind(serde_json::to_value(decision)?)
    .bind(digest)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(row) = inserted {
        tx.commit().await?;
        return row.try_into();
    }
    // Lost a race: the same ID, or another pending answer to this request.
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM {TABLE} WHERE id=$1 \
         OR (company=$2 AND target_key=$3 AND state='pending') ORDER BY created_at LIMIT 1"
    ))
    .bind(id)
    .bind(company)
    .bind(target.key())
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let saved = PendingApprovalDecision::try_from(row.context("pending approval decision")?)?;
    if saved.principal != *principal || saved.target != target || saved.decision != *decision {
        bail!("Another decision is already waiting for Undo.");
    }
    Ok(saved)
}

/// Undo: only the person who decided, only before the original deadline.
pub async fn cancel(
    authority: &AuthorityStore,
    company: &str,
    id: Uuid,
    principal: &HumanDecisionPrincipal,
) -> Result<PendingApprovalDecision> {
    let mut tx = authority.pool().begin().await?;
    let saved = read(&mut tx, id, true)
        .await?
        .filter(|saved| saved.company == company)
        .context("This decision was not found.")?;
    if saved.principal != *principal {
        bail!("This decision belongs to someone else or an older membership.");
    }
    if saved.state == "canceled" {
        tx.commit().await?;
        return Ok(saved);
    }
    let row: Option<Row> = sqlx::query_as(&format!(
        "UPDATE {TABLE} SET state='canceled',completed_at=clock_timestamp() \
         WHERE id=$1 AND state='pending' AND undo_until>clock_timestamp() RETURNING {COLUMNS}"
    ))
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let saved = row.context("The Undo window has ended.")?.try_into()?;
    tx.commit().await?;
    Ok(saved)
}

/// This person's waiting decisions and unread results in one company.
pub async fn for_actor(
    authority: &AuthorityStore,
    company: &str,
    actor_id: &str,
) -> Result<Vec<PendingApprovalDecision>> {
    let rows: Vec<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM {TABLE} WHERE company=$1 AND principal->>'actor_id'=$2 \
         AND (state='pending' OR seen_at IS NULL) ORDER BY created_at DESC LIMIT 100"
    ))
    .bind(company)
    .bind(actor_id)
    .fetch_all(authority.pool())
    .await?;
    rows.into_iter().map(TryInto::try_into).collect()
}

/// Acknowledging a result changes no permission.
pub async fn acknowledge(
    authority: &AuthorityStore,
    company: &str,
    id: Uuid,
    principal: &HumanDecisionPrincipal,
) -> Result<()> {
    let mut tx = authority.pool().begin().await?;
    let saved = read(&mut tx, id, true)
        .await?
        .filter(|saved| saved.company == company)
        .context("This decision was not found.")?;
    if !saved.principal.same_person(principal) || saved.state == "pending" {
        bail!("This result is not yours to acknowledge.");
    }
    sqlx::query(&format!(
        "UPDATE {TABLE} SET seen_at=COALESCE(seen_at,clock_timestamp()) WHERE id=$1"
    ))
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// End a pending decision's Undo window now. Exec's Inbox actions wait out their own window
/// first (the owner's), so the Authority answer they lead to is due at once.
pub async fn make_due(authority: &AuthorityStore, id: Uuid) -> Result<()> {
    sqlx::query(&format!(
        "UPDATE {TABLE} SET undo_until=LEAST(undo_until, clock_timestamp()) \
         WHERE id=$1 AND state='pending'"
    ))
    .bind(id)
    .execute(authority.pool())
    .await?;
    Ok(())
}

/// A decision's state and refusal, if it exists.
pub async fn outcome(
    authority: &AuthorityStore,
    id: Uuid,
) -> Result<Option<(String, Option<String>)>> {
    Ok(
        sqlx::query_as(&format!("SELECT state, problem FROM {TABLE} WHERE id=$1"))
            .bind(id)
            .fetch_optional(authority.pool())
            .await?,
    )
}

/// Delay to the next deadline in any company, by the database clock.
pub async fn next_delay(authority: &AuthorityStore) -> Result<Option<std::time::Duration>> {
    let seconds: Option<f64> = sqlx::query_scalar(&format!(
        "SELECT GREATEST(0, EXTRACT(EPOCH FROM (MIN(undo_until) - clock_timestamp())))::double precision \
         FROM {TABLE} WHERE state='pending' HAVING COUNT(*) > 0"
    ))
    .fetch_optional(authority.pool())
    .await?;
    Ok(seconds.map(std::time::Duration::from_secs_f64))
}

/// Decisions whose Undo window has ended, oldest deadline first.
pub async fn due(authority: &AuthorityStore) -> Result<Vec<Uuid>> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT id FROM {TABLE} WHERE state='pending' AND undo_until<=clock_timestamp() \
         ORDER BY undo_until,id LIMIT 50"
    ))
    .fetch_all(authority.pool())
    .await?)
}

/// One worker's hold on a due decision. The row stays locked until it is
/// settled, so a second worker skips it and a late Undo waits, then finds the
/// window over. A crash releases the lock with the row still pending.
pub struct Claim {
    tx: Transaction<'static, Postgres>,
    saved: PendingApprovalDecision,
}

pub async fn claim(authority: &AuthorityStore, id: Uuid) -> Result<Option<Claim>> {
    let mut tx = authority.pool().begin().await?;
    let row: Option<Row> = sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM {TABLE} WHERE id=$1 AND state='pending' \
         AND undo_until<=clock_timestamp() FOR UPDATE SKIP LOCKED"
    ))
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        tx.rollback().await?;
        return Ok(None);
    };
    Ok(Some(Claim {
        tx,
        saved: row.try_into()?,
    }))
}

impl Claim {
    pub fn decision(&self) -> &PendingApprovalDecision {
        &self.saved
    }

    /// Make the decision, unless `refusal` says the person may no longer make
    /// it. Either way the row is settled with its outcome.
    pub async fn settle(
        mut self,
        effects: &Effects<'_>,
        refusal: Option<String>,
    ) -> Result<&'static str> {
        let outcome = match refusal {
            Some(problem) => Err(problem),
            None => apply(effects, &self.saved)
                .await
                .map_err(|error| format!("{error:#}")),
        };
        let state = match outcome {
            Ok(()) => {
                sqlx::query(&format!(
                    "UPDATE {TABLE} SET state='applied',completed_at=clock_timestamp() WHERE id=$1"
                ))
                .bind(self.saved.id)
                .execute(&mut *self.tx)
                .await?;
                "applied"
            }
            Err(problem) => {
                let problem: String = problem.chars().take(2000).collect();
                sqlx::query(&format!(
                    "UPDATE {TABLE} SET state='failed',problem=$2,completed_at=clock_timestamp() \
                     WHERE id=$1"
                ))
                .bind(self.saved.id)
                .bind(problem)
                .execute(&mut *self.tx)
                .await?;
                "failed"
            }
        };
        self.tx.commit().await?;
        Ok(state)
    }
}

async fn already_decided(
    authority: &AuthorityStore,
    company: &str,
    kinds: &[&str],
    id: Uuid,
) -> Result<bool> {
    for kind in kinds {
        if authority
            .find_record(company, kind, "pending_decision_id", &id.to_string())
            .await?
            .is_some()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The ordinary Authority decision, now that its Undo window has ended.
async fn apply(effects: &Effects<'_>, saved: &PendingApprovalDecision) -> Result<()> {
    let Effects {
        root,
        authority,
        org,
        publication,
    } = *effects;
    let company = saved.company.as_str();
    let actor = saved.principal.actor_id.as_str();
    let grant = matches!(saved.decision, ApprovalDecision::Grant { .. });
    let recheck = || async {
        let Some(digest) = waiting_digest(authority, company, &saved.target).await? else {
            bail!("This request was already answered or is no longer waiting.");
        };
        if digest != saved.expected_digest {
            match saved.target {
                ApprovalTarget::Party { .. } | ApprovalTarget::ToolCall { .. } => {
                    bail!("The draft changed after you decided. Review the new one.")
                }
                _ => bail!("The request changed after you decided. Review the new one."),
            }
        }
        Ok::<_, anyhow::Error>(())
    };
    let note = saved.decision.note();
    match &saved.target {
        ApprovalTarget::EmailMandate { proposal } => {
            if !already_answered(authority, company, saved).await? {
                recheck().await?;
                authority
                    .decide_email_mandate_proposal(company, actor, *proposal, grant, note.text)
                    .await?;
            }
            crate::approval::announce_decisions(company, authority, org).await;
            return Ok(());
        }
        ApprovalTarget::Payee { proposal } => {
            if !already_answered(authority, company, saved).await? {
                recheck().await?;
                crate::finance::answer_payee(
                    authority, org, company, actor, *proposal, grant, note.text,
                )
                .await?;
            }
            return Ok(());
        }
        ApprovalTarget::AccountNotice { account, notice } => {
            if !already_answered(authority, company, saved).await? {
                recheck().await?;
                crate::account_broker::decide_notice(
                    authority.pool(),
                    account,
                    notice,
                    if grant { "fine" } else { "freeze" },
                    actor,
                )
                .await?
                .context("That notice no longer exists.")?;
            }
            return Ok(());
        }
        ApprovalTarget::AccountRequest { request } => {
            if !already_answered(authority, company, saved).await? {
                recheck().await?;
                let settled = crate::account_broker::settle_request(
                    authority.pool(),
                    request,
                    if grant { "approved" } else { "denied" },
                    None,
                    actor,
                )
                .await?
                .context("That request no longer exists.")?;
                if let Some(org) = org {
                    crate::account_broker::tell_requester(org, &settled).await;
                }
            }
            return Ok(());
        }
        ApprovalTarget::EmailSend { request } => {
            if !already_answered(authority, company, saved).await? {
                recheck().await?;
                crate::email_approval::decide(
                    authority,
                    org,
                    company,
                    *request,
                    grant,
                    actor,
                    note,
                    Some(saved.id),
                )
                .await?;
            }
            crate::approval::announce_decisions(company, authority, org).await;
            return Ok(());
        }
        ApprovalTarget::Publication { publication: id } => {
            if !already_answered(authority, company, saved).await? {
                let manager = publication.context("Publications are unavailable right now.")?;
                let org = org.context("The company is unavailable right now.")?;
                recheck().await?;
                if grant {
                    manager.authorize(org, company, id).await?;
                } else {
                    manager
                        .stop(org, company, id, "Declined by the owner from the Inbox")
                        .await?;
                }
            }
            return Ok(());
        }
        ApprovalTarget::Party { .. } | ApprovalTarget::ToolCall { .. } => {}
    }
    match &saved.target {
        ApprovalTarget::ToolCall { call_key } => {
            let approve = matches!(saved.decision, ApprovalDecision::Grant { .. });
            if !already_decided(
                authority,
                company,
                &crate::approval::TOOL_CALL_DECISION_KINDS,
                saved.id,
            )
            .await?
            {
                recheck().await?;
                crate::effect::decide_tool_call_as(
                    authority,
                    org,
                    company,
                    call_key,
                    approve,
                    actor,
                    saved.decision.note(),
                    Some(saved.id),
                )
                .await?;
            }
            if saved.decision == (ApprovalDecision::Grant { always: true }) {
                let request = authority
                    .find_record(company, "approval_required", "call_key", call_key)
                    .await?
                    .context("the prepared call's request is gone")?;
                let named = |field: &str| {
                    request
                        .body
                        .get(field)
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                };
                if let (Some(connection), Some(tool)) = (named("connection"), named("tool")) {
                    // Both halves are idempotent, so a replay finishes what a crash interrupted.
                    crate::connections::promote_tool(
                        authority.pool(),
                        authority,
                        company,
                        &connection,
                        &tool,
                        actor,
                    )
                    .await
                    .with_context(|| {
                        format!("the call was allowed, but {tool} still asks first")
                    })?;
                    crate::effect::allow_held_calls(authority, org, company, &connection, &tool, actor)
                        .await
                        .with_context(|| {
                            format!("{tool} acts without asking now, but its other waiting calls still wait")
                        })?;
                }
            }
        }
        ApprovalTarget::Party { party } => match &saved.decision {
            ApprovalDecision::Grant { .. } => {
                if already_decided(authority, company, &["approval_granted"], saved.id).await?
                    || crate::approval::is_approved(authority, company, party).await?
                {
                    return Ok(());
                }
                recheck().await?;
                crate::approval::grant_as(
                    root,
                    company,
                    party,
                    authority,
                    org,
                    actor,
                    Some(saved.id),
                )
                .await?;
            }
            ApprovalDecision::Decline { .. } => {
                if already_decided(authority, company, &["approval_declined"], saved.id).await? {
                    return Ok(());
                }
                recheck().await?;
                crate::approval::decline_as(
                    root,
                    company,
                    party,
                    authority,
                    org,
                    actor,
                    saved.decision.note(),
                    Some(saved.id),
                )
                .await?;
            }
        },
        _ => unreachable!("answered above"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::CompanyConfig;

    fn owner() -> HumanDecisionPrincipal {
        HumanDecisionPrincipal {
            actor_id: "owner".into(),
            owner_account: None,
            membership: None,
        }
    }

    struct Fixture {
        url: String,
        company: String,
        root: std::path::PathBuf,
        authority: AuthorityStore,
    }

    impl Fixture {
        async fn new() -> Option<Self> {
            let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
                eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping approval Undo scenario");
                return None;
            };
            let company = format!("approve_undo_{}_test", Uuid::new_v4().simple());
            let root =
                std::env::temp_dir().join(format!("restless-approval-undo-{}", Uuid::new_v4()));
            std::fs::create_dir_all(root.join("companies")).unwrap();
            CompanyConfig::save(&root, &config(&company)).unwrap();
            let authority = AuthorityStore::connect(&url).await.unwrap();
            Some(Self {
                url,
                company,
                root,
                authority,
            })
        }

        async fn ask_call(&self, key: &str, digest: &str) {
            self.authority
                .emit(
                    &self.company,
                    "approval_required",
                    Some("staff"),
                    serde_json::json!({
                        "call_key": key, "command_digest": digest, "tool": "send_email",
                        "connection": "gmail", "effect_class": "email_send",
                    }),
                )
                .await
                .unwrap();
        }

        async fn ask_party(&self, party: &str, digest: &str) {
            self.authority
                .emit(
                    &self.company,
                    "approval_required",
                    Some("staff"),
                    serde_json::json!({
                        "party": party, "draft_digest": digest, "effect_class": "email_send",
                    }),
                )
                .await
                .unwrap();
        }

        /// The deadline passes without waiting five real seconds.
        async fn expire(&self, id: Uuid) {
            sqlx::query(&format!(
                "UPDATE {TABLE} SET undo_until=clock_timestamp()-INTERVAL '1 second' WHERE id=$1"
            ))
            .bind(id)
            .execute(self.authority.pool())
            .await
            .unwrap();
        }

        async fn count(&self, kind: &str) -> usize {
            self.authority
                .records_of_kind(&self.company, kind)
                .await
                .unwrap()
                .len()
        }

        async fn settle(
            &self,
            id: Uuid,
            org: Option<&restless_orgintel::OrgIntel>,
        ) -> Option<&'static str> {
            let claim = claim(&self.authority, id).await.unwrap()?;
            let effects = Effects {
                root: &self.root,
                authority: &self.authority,
                org,
                publication: None,
            };
            Some(claim.settle(&effects, None).await.unwrap())
        }

        async fn state(&self, id: Uuid) -> String {
            sqlx::query_scalar(&format!("SELECT state FROM {TABLE} WHERE id=$1"))
                .bind(id)
                .fetch_one(self.authority.pool())
                .await
                .unwrap()
        }

        async fn done(self) {
            self.authority
                .delete_test_company(&self.company)
                .await
                .unwrap();
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn call(key: &str) -> (String, ApprovalTarget) {
        (
            format!("authority:tool-call:{key}"),
            ApprovalTarget::ToolCall {
                call_key: key.into(),
            },
        )
    }

    const GRANT: ApprovalDecision = ApprovalDecision::Grant { always: false };

    /// Nothing is granted or announced inside the window; after it, exactly one
    /// decision is recorded and the company is told once.
    #[tokio::test]
    async fn an_approval_takes_effect_only_after_its_window() {
        let Some(fx) = Fixture::new().await else {
            return;
        };
        let org = restless_orgintel::OrgIntel::ensure(&fx.url, &fx.company)
            .await
            .unwrap();
        fx.ask_call("k1", "d1").await;
        let (item, target) = call("k1");
        let id = Uuid::new_v4();
        let saved = queue(
            &fx.authority,
            id,
            &fx.company,
            &item,
            &owner(),
            &target,
            &GRANT,
        )
        .await
        .unwrap();
        assert_eq!(saved.state, "pending");
        let window = saved.undo_until - saved.created_at;
        assert!(window >= chrono::Duration::milliseconds(4900), "{window:?}");

        // Inside the window: not due, not claimable, nothing recorded, nobody woken.
        assert!(!due(&fx.authority).await.unwrap().contains(&id));
        assert!(fx.settle(id, Some(&org)).await.is_none());
        assert_eq!(fx.count("tool_call_approved").await, 0);
        assert_eq!(org.inbox(Some("exec")).await.unwrap().len(), 0);
        let request = fx
            .authority
            .find_record(&fx.company, "approval_required", "call_key", "k1")
            .await
            .unwrap()
            .unwrap();
        assert!(
            crate::effect::held_call_waiting(&fx.authority, &fx.company, "k1", &request)
                .await
                .unwrap()
        );
        assert!(next_delay(&fx.authority).await.unwrap().is_some());

        fx.expire(id).await;
        assert!(due(&fx.authority).await.unwrap().contains(&id));
        assert_eq!(fx.settle(id, Some(&org)).await, Some("applied"));
        assert_eq!(fx.count("tool_call_approved").await, 1);
        let record = fx
            .authority
            .find_body(
                &fx.company,
                "tool_call_approved",
                "pending_decision_id",
                &id.to_string(),
            )
            .await
            .unwrap()
            .expect("the decision names its pending row");
        assert_eq!(record["principal"], "owner");
        assert_eq!(org.inbox(Some("exec")).await.unwrap().len(), 1);
        // Settled once: no second claim, no second record.
        assert!(fx.settle(id, Some(&org)).await.is_none());
        assert_eq!(fx.count("tool_call_approved").await, 1);
        org.drop_schema().await.ok();
        fx.done().await;
    }

    /// Undo inside the window cancels; the deadline then applies nothing, and
    /// Undo after the deadline is refused.
    #[tokio::test]
    async fn undo_cancels_the_pending_approval() {
        let Some(fx) = Fixture::new().await else {
            return;
        };
        fx.ask_call("k2", "d2").await;
        let (item, target) = call("k2");
        let id = Uuid::new_v4();
        queue(
            &fx.authority,
            id,
            &fx.company,
            &item,
            &owner(),
            &target,
            &GRANT,
        )
        .await
        .unwrap();
        let someone = HumanDecisionPrincipal {
            actor_id: "member-1".into(),
            ..owner()
        };
        assert!(cancel(&fx.authority, &fx.company, id, &someone)
            .await
            .is_err());
        let canceled = cancel(&fx.authority, &fx.company, id, &owner())
            .await
            .unwrap();
        assert_eq!(canceled.state, "canceled");
        // A repeated Undo is the same answer.
        assert_eq!(
            cancel(&fx.authority, &fx.company, id, &owner())
                .await
                .unwrap()
                .state,
            "canceled"
        );
        fx.expire(id).await;
        assert!(fx.settle(id, None).await.is_none());
        assert_eq!(fx.count("tool_call_approved").await, 0);

        // The request is still waiting, so a fresh answer is possible; its Undo
        // after the deadline is refused and it applies.
        let again = Uuid::new_v4();
        queue(
            &fx.authority,
            again,
            &fx.company,
            &item,
            &owner(),
            &target,
            &GRANT,
        )
        .await
        .unwrap();
        fx.expire(again).await;
        assert!(cancel(&fx.authority, &fx.company, again, &owner())
            .await
            .is_err());
        assert_eq!(fx.settle(again, None).await, Some("applied"));
        assert_eq!(fx.count("tool_call_approved").await, 1);
        fx.done().await;
    }

    /// A restarted daemon finds the original deadline in PostgreSQL. A crash
    /// after the decision was recorded but before the row settled replays to
    /// the same single decision; two workers never both apply.
    #[tokio::test]
    async fn restart_applies_due_decisions_exactly_once() {
        let Some(fx) = Fixture::new().await else {
            return;
        };
        fx.ask_call("k3", "d3").await;
        let (item, target) = call("k3");
        let id = Uuid::new_v4();
        let saved = queue(
            &fx.authority,
            id,
            &fx.company,
            &item,
            &owner(),
            &target,
            &GRANT,
        )
        .await
        .unwrap();
        fx.expire(id).await;

        // "Restart": a new store and pool, nothing carried over in memory.
        let restarted = AuthorityStore::connect(&fx.url).await.unwrap();
        assert!(due(&restarted).await.unwrap().contains(&id));
        let first = claim(&restarted, id).await.unwrap().expect("claimable");
        assert_eq!(first.decision().created_at, saved.created_at);
        // A second worker skips the held row.
        assert!(claim(&fx.authority, id).await.unwrap().is_none());
        // The decision is recorded, then the worker dies before settling.
        crate::effect::decide_tool_call_as(
            &restarted,
            None,
            &fx.company,
            "k3",
            true,
            "owner",
            OwnerNote::default(),
            Some(id),
        )
        .await
        .unwrap();
        drop(first);
        restarted.pool().close().await;
        assert_eq!(fx.state(id).await, "pending");

        // The next worker replays to the decision already given.
        assert_eq!(fx.settle(id, None).await, Some("applied"));
        assert_eq!(fx.count("tool_call_approved").await, 1);
        assert!(fx.settle(id, None).await.is_none());
        fx.done().await;
    }

    /// Replays keep one row and one deadline; a competing answer is refused;
    /// a changed draft is not decided on the old one's say-so.
    #[tokio::test]
    async fn one_answer_per_request_and_the_displayed_draft() {
        let Some(fx) = Fixture::new().await else {
            return;
        };
        fx.ask_call("k4", "d4").await;
        let (item, target) = call("k4");
        let id = Uuid::new_v4();
        let saved = queue(
            &fx.authority,
            id,
            &fx.company,
            &item,
            &owner(),
            &target,
            &GRANT,
        )
        .await
        .unwrap();
        let replay = queue(
            &fx.authority,
            id,
            &fx.company,
            &item,
            &owner(),
            &target,
            &GRANT,
        )
        .await
        .unwrap();
        assert_eq!(replay.undo_until, saved.undo_until);
        let other_tab = queue(
            &fx.authority,
            Uuid::new_v4(),
            &fx.company,
            &item,
            &owner(),
            &target,
            &GRANT,
        )
        .await
        .unwrap();
        assert_eq!(
            other_tab.id, id,
            "an equivalent click joins the waiting answer"
        );
        assert_eq!(other_tab.undo_until, saved.undo_until);
        let decline = ApprovalDecision::Decline {
            note: None,
            changes_requested: false,
        };
        assert!(queue(
            &fx.authority,
            Uuid::new_v4(),
            &fx.company,
            &item,
            &owner(),
            &target,
            &decline
        )
        .await
        .is_err());
        assert!(queue(
            &fx.authority,
            id,
            &fx.company,
            &item,
            &owner(),
            &target,
            &decline
        )
        .await
        .is_err());
        // Request changes needs words, refused at once rather than after the window.
        fx.ask_call("k5", "d5").await;
        let (item5, target5) = call("k5");
        let empty = ApprovalDecision::Decline {
            note: None,
            changes_requested: true,
        };
        assert!(queue(
            &fx.authority,
            Uuid::new_v4(),
            &fx.company,
            &item5,
            &owner(),
            &target5,
            &empty
        )
        .await
        .is_err());

        // The company changes the draft inside the window.
        fx.ask_call("k4", "d4-changed").await;
        fx.expire(id).await;
        assert_eq!(fx.settle(id, None).await, Some("failed"));
        assert_eq!(fx.count("tool_call_approved").await, 0);
        let failed = for_actor(&fx.authority, &fx.company, "owner")
            .await
            .unwrap();
        assert!(failed[0].problem.as_deref().unwrap().contains("changed"));
        acknowledge(&fx.authority, &fx.company, id, &owner())
            .await
            .unwrap();
        assert!(for_actor(&fx.authority, &fx.company, "owner")
            .await
            .unwrap()
            .is_empty());
        fx.done().await;
    }

    /// First contact: a decline with a note waits too, and records the note once.
    #[tokio::test]
    async fn a_first_contact_decline_waits_for_its_window() {
        let Some(fx) = Fixture::new().await else {
            return;
        };
        fx.ask_party("Lead@Acme.com", "p1").await;
        let target = ApprovalTarget::Party {
            party: "lead@acme.com".into(),
        };
        let item = "authority:approval:email_send:lead@acme.com";
        let decline = ApprovalDecision::Decline {
            note: Some("Not this week".into()),
            changes_requested: false,
        };
        let id = Uuid::new_v4();
        queue(
            &fx.authority,
            id,
            &fx.company,
            item,
            &owner(),
            &target,
            &decline,
        )
        .await
        .unwrap();
        assert!(queue(
            &fx.authority,
            Uuid::new_v4(),
            &fx.company,
            "wrong:item",
            &owner(),
            &target,
            &decline
        )
        .await
        .is_err());
        assert_eq!(fx.count("approval_declined").await, 0);
        fx.expire(id).await;
        assert_eq!(fx.settle(id, None).await, Some("applied"));
        let declines = fx
            .authority
            .records_of_kind(&fx.company, "approval_declined")
            .await
            .unwrap();
        assert_eq!(declines.len(), 1);
        assert_eq!(declines[0].body["owner_note"], "Not this week");
        // Answered: a grant for the same draft is not waiting any more.
        assert!(queue(
            &fx.authority,
            Uuid::new_v4(),
            &fx.company,
            item,
            &owner(),
            &target,
            &GRANT
        )
        .await
        .is_err());

        // A grant for another party applies as a standing approval, only after its window.
        fx.ask_party("ops@acme.com", "p2").await;
        let grant_id = Uuid::new_v4();
        queue(
            &fx.authority,
            grant_id,
            &fx.company,
            "authority:approval:email_send:ops@acme.com",
            &owner(),
            &ApprovalTarget::Party {
                party: "ops@acme.com".into(),
            },
            &GRANT,
        )
        .await
        .unwrap();
        assert!(
            !crate::approval::is_approved(&fx.authority, &fx.company, "ops@acme.com")
                .await
                .unwrap()
        );
        fx.expire(grant_id).await;
        assert_eq!(fx.settle(grant_id, None).await, Some("applied"));
        assert!(
            crate::approval::is_approved(&fx.authority, &fx.company, "ops@acme.com")
                .await
                .unwrap()
        );
        assert_eq!(fx.count("approval_granted").await, 1);
        fx.done().await;
    }

    impl Fixture {
        async fn answer(
            &self,
            target: &ApprovalTarget,
            decision: &ApprovalDecision,
        ) -> Result<PendingApprovalDecision> {
            queue(
                &self.authority,
                Uuid::new_v4(),
                &self.company,
                &target.item_id().unwrap(),
                &owner(),
                target,
                decision,
            )
            .await
        }

        /// A worker claims the due row, the decision is made, and the worker
        /// dies before settling: the row stays pending for the next worker.
        async fn crash_after(&self, id: Uuid, decide: impl std::future::Future<Output = ()>) {
            let restarted = AuthorityStore::connect(&self.url).await.unwrap();
            let held = claim(&restarted, id).await.unwrap().expect("claimable");
            assert!(claim(&self.authority, id).await.unwrap().is_none());
            decide.await;
            drop(held);
            restarted.pool().close().await;
            assert_eq!(self.state(id).await, "pending");
        }
    }

    const DECLINE: ApprovalDecision = ApprovalDecision::Decline {
        note: None,
        changes_requested: false,
    };

    /// An email mandate is granted only after its window; Undo leaves the proposal
    /// waiting; a replay after a crash grants it once.
    #[tokio::test]
    async fn a_mandate_answer_waits_for_its_window() {
        let Some(fx) = Fixture::new().await else {
            return;
        };
        let proposal = |per_day| crate::mandate::NewEmailMandate {
            purpose: "Invite past clients to the open day".into(),
            audience_guidance: "People who bought in the last two years".into(),
            sender: "hello@example.com".into(),
            sender_name: None,
            max_per_day: per_day,
            max_total: 100,
            timezone: "UTC".into(),
            expires_at: Utc::now() + chrono::Duration::days(30),
        };
        let first = fx
            .authority
            .propose_email_mandate(&fx.company, "exec", proposal(5), None, None)
            .await
            .unwrap();
        let target = ApprovalTarget::EmailMandate { proposal: first };
        // Yes or no: a request for changes is refused at once, not after five seconds.
        assert!(fx
            .answer(
                &target,
                &ApprovalDecision::Decline {
                    note: Some("smaller".into()),
                    changes_requested: true
                }
            )
            .await
            .is_err());
        let undone = fx.answer(&target, &GRANT).await.unwrap();
        cancel(&fx.authority, &fx.company, undone.id, &owner())
            .await
            .unwrap();
        fx.expire(undone.id).await;
        assert!(fx.settle(undone.id, None).await.is_none());
        assert_eq!(
            fx.authority
                .pending_email_mandate_proposals(&fx.company)
                .await
                .unwrap()
                .len(),
            1,
            "undone: still waiting for an answer"
        );

        let saved = fx.answer(&target, &GRANT).await.unwrap();
        assert_eq!(saved.receipt_kind(), "approve");
        assert!(fx.settle(saved.id, None).await.is_none(), "not due yet");
        assert!(fx
            .authority
            .list_email_mandates(&fx.company)
            .await
            .unwrap()
            .is_empty());
        fx.expire(saved.id).await;
        fx.crash_after(saved.id, async {
            fx.authority
                .decide_email_mandate_proposal(&fx.company, "owner", first, true, None)
                .await
                .unwrap();
        })
        .await;
        assert_eq!(fx.settle(saved.id, None).await, Some("applied"));
        assert_eq!(
            fx.authority
                .list_email_mandates(&fx.company)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(fx.count("email_mandate_proposal_decision").await, 1);
        assert!(fx.settle(saved.id, None).await.is_none());

        // A decline with a note is recorded once, with the note.
        let second = fx
            .authority
            .propose_email_mandate(&fx.company, "exec", proposal(9), None, None)
            .await
            .unwrap();
        let declined = fx
            .answer(
                &ApprovalTarget::EmailMandate { proposal: second },
                &ApprovalDecision::Decline {
                    note: Some("Not before the launch".into()),
                    changes_requested: false,
                },
            )
            .await
            .unwrap();
        assert_eq!(fx.count("email_mandate_proposal_decision").await, 1);
        fx.expire(declined.id).await;
        assert_eq!(fx.settle(declined.id, None).await, Some("applied"));
        let decisions = fx
            .authority
            .records_of_kind(&fx.company, "email_mandate_proposal_decision")
            .await
            .unwrap();
        assert_eq!(decisions.len(), 2);
        assert!(decisions
            .iter()
            .any(|record| record.body["owner_note"] == "Not before the launch"));
        fx.done().await;
    }

    /// A payee joins the allowance only after the window; Undo leaves it waiting;
    /// a crash after the answer replays to the one answer.
    #[tokio::test]
    async fn a_payee_answer_waits_for_its_window() {
        let Some(fx) = Fixture::new().await else {
            return;
        };
        crate::finance::set_envelope(
            &fx.authority,
            &fx.company,
            crate::finance::MoneyEnvelopeInput {
                source_account_ref: "wallet-aud".into(),
                currency: "AUD".into(),
                beneficiary_refs: vec!["beneficiary-1".into()],
                per_payment_limit_minor: 1_000,
                aggregate_limit_minor: 5_000,
                frozen: false,
            },
            "owner",
        )
        .await
        .unwrap();
        let propose = |beneficiary: &str| crate::finance::PayeeProposalInput {
            currency: "aud".into(),
            beneficiary_ref: beneficiary.into(),
            reason: "The printer for the open day.".into(),
            work_id: None,
        };
        let payees = || async {
            crate::finance::envelopes(&fx.authority, &fx.company)
                .await
                .unwrap()
                .remove(0)
                .limits
                .beneficiary_refs
        };
        let asked = crate::finance::propose_payee(
            &fx.authority,
            &fx.company,
            "exec",
            propose("beneficiary-2"),
        )
        .await
        .unwrap();
        let target = ApprovalTarget::Payee { proposal: asked };
        let undone = fx.answer(&target, &GRANT).await.unwrap();
        cancel(&fx.authority, &fx.company, undone.id, &owner())
            .await
            .unwrap();
        fx.expire(undone.id).await;
        assert!(fx.settle(undone.id, None).await.is_none());
        assert_eq!(payees().await, vec!["beneficiary-1".to_string()]);

        let saved = fx.answer(&target, &GRANT).await.unwrap();
        assert!(fx.settle(saved.id, None).await.is_none(), "not due yet");
        assert_eq!(fx.count(crate::finance::PAYEE_DECIDED_KIND).await, 0);
        fx.expire(saved.id).await;
        fx.crash_after(saved.id, async {
            crate::finance::decide_payee(&fx.authority, &fx.company, "owner", asked, true, None)
                .await
                .unwrap();
        })
        .await;
        assert_eq!(fx.settle(saved.id, None).await, Some("applied"));
        assert_eq!(fx.count(crate::finance::PAYEE_DECIDED_KIND).await, 1);
        assert_eq!(
            payees().await,
            vec!["beneficiary-1".to_string(), "beneficiary-2".to_string()]
        );

        // Declined with a note, after the window, once.
        let other = crate::finance::propose_payee(
            &fx.authority,
            &fx.company,
            "exec",
            propose("beneficiary-3"),
        )
        .await
        .unwrap();
        let declined = fx
            .answer(
                &ApprovalTarget::Payee { proposal: other },
                &ApprovalDecision::Decline {
                    note: Some("Use the usual printer".into()),
                    changes_requested: false,
                },
            )
            .await
            .unwrap();
        fx.expire(declined.id).await;
        assert_eq!(fx.settle(declined.id, None).await, Some("applied"));
        assert_eq!(fx.count(crate::finance::PAYEE_DECIDED_KIND).await, 2);
        assert_eq!(payees().await.len(), 2);
        fx.done().await;
    }

    /// A Freeze waits out the window before the grant is frozen; an account request
    /// is granted as asked only after it; replays and Undo hold for both.
    #[tokio::test]
    async fn account_answers_wait_for_their_window() {
        let Some(fx) = Fixture::new().await else {
            return;
        };
        let pool = fx.authority.pool();
        let account = Uuid::new_v4().simple().to_string();
        sqlx::query(
            "INSERT INTO restless_authority.owner_accounts (id, kind, label) \
             VALUES ($1, 'github', 'GitHub')",
        )
        .bind(&account)
        .execute(pool)
        .await
        .unwrap();
        let grant = crate::accounts::create_grant(
            pool,
            &account,
            crate::accounts::GrantInput {
                company: Some(fx.company.clone()),
                ..Default::default()
            },
            "owner",
        )
        .await
        .unwrap();
        let notice = Uuid::new_v4().simple().to_string();
        sqlx::query(
            "INSERT INTO restless_authority.owner_account_notices \
             (id, account_id, grant_id, company, use_id, kind) VALUES ($1,$2,$3,$4,1,'first_use')",
        )
        .bind(&notice)
        .bind(&account)
        .bind(&grant.id)
        .bind(&fx.company)
        .execute(pool)
        .await
        .unwrap();
        let frozen = || async {
            crate::accounts::company_grants(pool, &fx.company)
                .await
                .unwrap()[0]
                .0
                .frozen
        };
        let target = ApprovalTarget::AccountNotice {
            account: account.clone(),
            notice: notice.clone(),
        };
        // Another company's Inbox cannot answer this notice.
        assert!(queue(
            &fx.authority,
            Uuid::new_v4(),
            "someone_else_test",
            &target.item_id().unwrap(),
            &owner(),
            &target,
            &DECLINE
        )
        .await
        .is_err());
        let freeze = fx.answer(&target, &DECLINE).await.unwrap();
        assert_eq!(freeze.receipt_kind(), "freeze");
        assert!(fx.settle(freeze.id, None).await.is_none(), "not due yet");
        assert!(!frozen().await, "nothing is frozen inside the window");
        cancel(&fx.authority, &fx.company, freeze.id, &owner())
            .await
            .unwrap();
        let again = fx.answer(&target, &DECLINE).await.unwrap();
        fx.expire(again.id).await;
        assert_eq!(fx.settle(again.id, None).await, Some("applied"));
        assert!(frozen().await);
        assert!(fx.settle(again.id, None).await.is_none());
        // Answered: a Fine for the same notice is no longer waiting.
        assert!(fx.answer(&target, &GRANT).await.is_err());

        let other = format!("{}_other", fx.company);
        let asked = crate::accounts::Abilities {
            act: "allow".into(),
            ..Default::default()
        };
        let request = crate::account_broker::create_request(
            pool,
            &fx.company,
            Some(&account),
            None,
            Some(asked.clone()),
            "To open release PRs.",
            "baloo",
        )
        .await
        .unwrap();
        let target = ApprovalTarget::AccountRequest {
            request: request.id.clone(),
        };
        let approve = fx.answer(&target, &GRANT).await.unwrap();
        assert!(fx.settle(approve.id, None).await.is_none(), "not due yet");
        assert_eq!(
            crate::account_broker::open_requests(pool, &fx.company)
                .await
                .unwrap()
                .len(),
            1
        );
        fx.expire(approve.id).await;
        fx.crash_after(approve.id, async {
            crate::account_broker::settle_request(pool, &request.id, "approved", None, "owner")
                .await
                .unwrap();
        })
        .await;
        assert_eq!(fx.settle(approve.id, None).await, Some("applied"));
        let granted = crate::accounts::company_grants(pool, &fx.company)
            .await
            .unwrap();
        assert_eq!(granted[0].0.abilities, asked);

        // A request that names no account is chosen with limits, not approved here.
        let unnamed = crate::account_broker::create_request(
            pool,
            &fx.company,
            None,
            Some("github"),
            None,
            "Any GitHub",
            "baloo",
        )
        .await
        .unwrap();
        let target = ApprovalTarget::AccountRequest {
            request: unnamed.id.clone(),
        };
        assert!(fx.answer(&target, &GRANT).await.is_err());
        let deny = fx.answer(&target, &DECLINE).await.unwrap();
        assert_eq!(deny.receipt_kind(), "deny");
        fx.expire(deny.id).await;
        assert_eq!(fx.settle(deny.id, None).await, Some("applied"));
        assert!(crate::account_broker::open_requests(pool, &fx.company)
            .await
            .unwrap()
            .is_empty());

        crate::accounts::delete(pool, &account).await.unwrap();
        for table in ["owner_account_notices", "owner_account_requests"] {
            sqlx::query(&format!(
                "DELETE FROM restless_authority.{table} WHERE company = ANY($1)"
            ))
            .bind(vec![fx.company.clone(), other.clone()])
            .execute(pool)
            .await
            .unwrap();
        }
        fx.done().await;
    }

    /// Nothing is put online or stopped inside the window; a replay after the
    /// provider was told settles to that one answer.
    #[tokio::test]
    async fn a_publication_answer_waits_for_its_window() {
        let Some(fx) = Fixture::new().await else {
            return;
        };
        let ask = |id: &str, expires: DateTime<Utc>| {
            fx.authority.emit(
                &fx.company,
                "publication_request",
                Some("game-builder"),
                serde_json::json!({
                    "publication_id": id,
                    "request": {"expires_at": expires.to_rfc3339()},
                }),
            )
        };
        ask("pub-1", Utc::now() + chrono::Duration::hours(1))
            .await
            .unwrap();
        ask("pub-old", Utc::now() - chrono::Duration::minutes(1))
            .await
            .unwrap();
        assert!(fx
            .answer(
                &ApprovalTarget::Publication {
                    publication: "pub-old".into()
                },
                &GRANT
            )
            .await
            .is_err());
        let target = ApprovalTarget::Publication {
            publication: "pub-1".into(),
        };
        assert!(fx
            .answer(
                &target,
                &ApprovalDecision::Decline {
                    note: Some("no".into()),
                    changes_requested: false
                }
            )
            .await
            .is_err());
        let authorize = fx.answer(&target, &GRANT).await.unwrap();
        assert_eq!(authorize.receipt_kind(), "authorize");
        assert!(fx.settle(authorize.id, None).await.is_none(), "not due yet");
        cancel(&fx.authority, &fx.company, authorize.id, &owner())
            .await
            .unwrap();
        assert_eq!(fx.count("publication_authorized").await, 0);

        // Without its manager the decision cannot be made: it fails visibly.
        let failed = fx.answer(&target, &DECLINE).await.unwrap();
        fx.expire(failed.id).await;
        assert_eq!(fx.settle(failed.id, None).await, Some("failed"));
        assert_eq!(fx.count("publication_stopped").await, 0);

        let decline = fx.answer(&target, &DECLINE).await.unwrap();
        fx.expire(decline.id).await;
        fx.crash_after(decline.id, async {
            fx.authority
                .emit(
                    &fx.company,
                    "publication_stopped",
                    Some("owner"),
                    serde_json::json!({"publication_id": "pub-1"}),
                )
                .await
                .unwrap();
        })
        .await;
        assert_eq!(fx.settle(decline.id, None).await, Some("applied"));
        assert_eq!(fx.count("publication_stopped").await, 1);
        fx.done().await;
    }

    /// S109 H2: an email without a mandate waits in the Inbox. Approval takes effect only after
    /// the window and Undo; then exactly that email may be reserved once. A changed draft asks
    /// again, and a decline is final for that draft.
    #[tokio::test]
    async fn an_email_send_waits_for_the_owner_and_goes_out_once() {
        use crate::email_approval::{approval_for, OwnerApproval};
        let Some(fx) = Fixture::new().await else {
            return;
        };
        let draft = |text: &str, key: &str| {
            let input: crate::email::EmailSendRequest = serde_json::from_value(serde_json::json!({
                "from": "hello@mail.example.com", "to": "jo@example.net",
                "subject": "Your quote", "text": text, "effect_key": key,
            }))
            .unwrap();
            let prepared = crate::email::prepare_email(input.clone(), Vec::new()).unwrap();
            (input, prepared)
        };
        let ask = |text: &'static str, key: &'static str| {
            let (input, prepared) = draft(text, key);
            let authority = &fx.authority;
            let company = fx.company.clone();
            async move {
                approval_for(authority, &company, "exec", &prepared, &input, None)
                    .await
                    .unwrap()
            }
        };
        let OwnerApproval::Waiting { request, new: true } = ask("Hi Jo", "quote-jo").await else {
            panic!("the first send asks the owner");
        };
        // Asking again is the same card, not another one.
        assert_eq!(
            ask("Hi Jo", "quote-jo").await,
            OwnerApproval::Waiting {
                request,
                new: false
            }
        );
        let waiting = crate::email_approval::waiting(&fx.authority, &fx.company)
            .await
            .unwrap();
        assert_eq!(waiting.len(), 1);
        assert!(waiting[0]
            .1
            .readable()
            .contains("Subject: Your quote\n\nHi Jo"));

        // Nothing can be reserved before the owner approves.
        let (_, prepared) = draft("Hi Jo", "quote-jo");
        let reserve = |prepared: crate::email::PreparedEmail, request: Uuid| {
            let authority = &fx.authority;
            let company = fx.company.clone();
            async move {
                authority
                    .reserve_owner_approved_email_send(
                        &company,
                        request,
                        prepared.sender(),
                        prepared.sender_name(),
                        prepared.recipient(),
                        prepared.payload_sha256(),
                        prepared.effect_key(),
                    )
                    .await
            }
        };
        assert!(reserve(prepared, request).await.is_err());

        // Send, then Undo: still waiting. Send again: applied after the window.
        let target = ApprovalTarget::EmailSend { request };
        let first = fx.answer(&target, &GRANT).await.unwrap();
        assert_eq!(first.receipt_kind(), "send");
        assert!(fx.settle(first.id, None).await.is_none(), "not due yet");
        cancel(&fx.authority, &fx.company, first.id, &owner())
            .await
            .unwrap();
        assert!(matches!(
            ask("Hi Jo", "quote-jo").await,
            OwnerApproval::Waiting { .. }
        ));
        let send = fx.answer(&target, &GRANT).await.unwrap();
        fx.expire(send.id).await;
        assert_eq!(fx.settle(send.id, None).await, Some("applied"));
        assert_eq!(
            ask("Hi Jo", "quote-jo").await,
            OwnerApproval::Approved { request }
        );
        assert!(crate::email_approval::waiting(&fx.authority, &fx.company)
            .await
            .unwrap()
            .is_empty());

        // A different email under the approved key is not what the owner approved.
        let (_, changed) = draft("Hi Jo, a cheaper quote", "quote-jo");
        assert!(reserve(changed, request).await.is_err());
        let (_, prepared) = draft("Hi Jo", "quote-jo");
        reserve(prepared, request).await.unwrap();
        let (_, again) = draft("Hi Jo", "quote-jo");
        assert!(reserve(again, request).await.is_err(), "sent once");
        assert!(
            approval_for(
                &fx.authority,
                &fx.company,
                "exec",
                &draft("Hi Jo", "quote-jo").1,
                &draft("Hi Jo", "quote-jo").0,
                None
            )
            .await
            .is_err(),
            "a used effect key never asks again"
        );

        // Changes asked for: that draft never sends; the revised one asks again.
        let OwnerApproval::Waiting { request: other, .. } = ask("Hey", "intro-jo").await else {
            panic!("asks");
        };
        let changes = fx
            .answer(
                &ApprovalTarget::EmailSend { request: other },
                &ApprovalDecision::Decline {
                    note: Some("Say who we are".into()),
                    changes_requested: true,
                },
            )
            .await
            .unwrap();
        fx.expire(changes.id).await;
        assert_eq!(fx.settle(changes.id, None).await, Some("applied"));
        assert_eq!(
            ask("Hey", "intro-jo").await,
            OwnerApproval::Declined {
                request: other,
                note: Some("Say who we are".into()),
                changes: true
            }
        );
        let OwnerApproval::Waiting {
            request: revised,
            new: true,
        } = ask("Hi Jo, I'm Sam from Reno Co.", "intro-jo").await
        else {
            panic!("a revised draft asks again");
        };
        assert_ne!(revised, other);
        fx.done().await;
    }

    fn config(name: &str) -> CompanyConfig {
        CompanyConfig {
            compute_limits: None,
            agent_intelligence: Default::default(),
            native_harnesses: Default::default(),
            display_name: None,
            logo: None,
            name: name.to_string(),
            internal_network: false,
            mission: String::new(),
            spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(30_000_000),
            spend_period: Default::default(),
            spend_warning_percent: None,
            spend_warning_baseline: None,
            home_sites: Vec::new(),
            monthly_runtime_cap_hours: None,
            native_monthly_turn_limit: None,
            native_monthly_token_limit: None,
            auto_sleep_after_minutes: None,
            model: "moonshot/kimi-k3".to_string(),
            coordination_harness: crate::runtime::AgentHarness::RestlessManaged,
            worker_harness: crate::runtime::AgentHarness::RestlessManaged,
            reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.to_string(),
            model_failover: Vec::new(),
            credentials: std::collections::BTreeMap::new(),
            approved_parties: Vec::new(),
            everyday_model: None,
            decision_model: None,
        }
    }
}
