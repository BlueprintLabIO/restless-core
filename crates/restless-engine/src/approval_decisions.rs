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

/// What the person answered. Exactly one prepared call, or one party's first contact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApprovalTarget {
    Party { party: String },
    ToolCall { call_key: String },
}

impl ApprovalTarget {
    fn key(&self) -> String {
        match self {
            Self::Party { party } => format!("party:{}", normalize_party(party)),
            Self::ToolCall { call_key } => format!("call:{call_key}"),
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
    }
}

fn valid_item_id(item_id: &str, target: &ApprovalTarget) -> bool {
    match target {
        ApprovalTarget::ToolCall { call_key } => {
            item_id == format!("authority:tool-call:{call_key}")
        }
        ApprovalTarget::Party { party } => {
            item_id.starts_with("authority:approval:") && item_id.ends_with(&format!(":{party}"))
        }
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
        (ApprovalTarget::Party { .. }, ApprovalDecision::Grant { always: true }) => {
            bail!("only a tool call can be allowed from now on")
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
    let Some(request) = waiting_request(authority, company, &target).await? else {
        bail!("This request is no longer waiting.");
    };
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
    .bind(digest_of(&request))
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
        root: &Path,
        authority: &AuthorityStore,
        org: Option<&restless_orgintel::OrgIntel>,
        refusal: Option<String>,
    ) -> Result<&'static str> {
        let outcome = match refusal {
            Some(problem) => Err(problem),
            None => apply(root, authority, org, &self.saved)
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
async fn apply(
    root: &Path,
    authority: &AuthorityStore,
    org: Option<&restless_orgintel::OrgIntel>,
    saved: &PendingApprovalDecision,
) -> Result<()> {
    let company = saved.company.as_str();
    let actor = saved.principal.actor_id.as_str();
    let recheck = || async {
        let Some(request) = waiting_request(authority, company, &saved.target).await? else {
            bail!("This request was already answered or is no longer waiting.");
        };
        if digest_of(&request) != saved.expected_digest {
            bail!("The draft changed after you decided. Review the new one.");
        }
        Ok::<_, anyhow::Error>(request)
    };
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
            Some(
                claim
                    .settle(&self.root, &self.authority, org, None)
                    .await
                    .unwrap(),
            )
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
