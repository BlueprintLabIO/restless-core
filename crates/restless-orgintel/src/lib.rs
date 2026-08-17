//! OrgIntel core (ARCHITECTURE.md §4.4): recoverable coordination state for
//! one company — actors, goals, Work/Attempts, messages, artifact refs,
//! decisions, events. Explicitly outside the constitutional trust boundary
//! (§4.9): nothing here is a ledger, a custody machine, or kernel truth.
//!
//! One schema per company; the company name is the schema name. One
//! [`OrgIntel`] handle = one company, with `search_path` pinned on every
//! pooled connection so no query can cross companies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

/// The Work lifecycle. Migration 0006 renames the former primitive in place;
/// there is no second task or workflow truth beneath it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "work_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkStatus {
    Proposed,
    Active,
    Blocked,
    Completed,
    Abandoned,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ts_rs::TS,
)]
#[sqlx(type_name = "work_edge_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkEdgeKind {
    Requires,
    Revises,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ts_rs::TS)]
#[sqlx(type_name = "work_attempt_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkAttemptState {
    Running,
    Produced,
    ChangesRequested,
    Blocked,
    Failed,
    Abandoned,
    Superseded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ts_rs::TS)]
#[sqlx(type_name = "artifact_ref_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRefState {
    Available,
    Stale,
    Missing,
    Superseded,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ts_rs::TS)]
#[sqlx(type_name = "owner_handoff_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OwnerHandoffCategory {
    Identity,
    Captcha,
    Mfa,
    LegalAttestation,
    PaymentConfirmation,
    OwnerJudgement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ts_rs::TS)]
#[sqlx(type_name = "owner_handoff_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OwnerHandoffState {
    Pending,
    Resolved,
    Declined,
    Withdrawn,
}

/// An explicit owner decision on a prepared outcome. Ordinary Work-linked
/// conversation never implies either variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerReviewDecision {
    Accepted,
    ChangesRequested,
}

/// The exact runtime workspace inherited by every Attempt of a Work node.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct WorkspaceSpec {
    pub repo: Option<String>,
    pub base_ref: Option<String>,
    pub integration_branch: Option<String>,
    pub worktree: Option<String>,
}

/// Input for one Work node. The node is deliberately a stable outcome
/// contract around flexible model/runtime execution.
pub struct NewWork<'a> {
    pub owner_id: &'a str,
    pub title: &'a str,
    pub outcome: &'a str,
    pub goal_id: Option<Uuid>,
    pub priority: i16,
    pub expected_artifact: &'a str,
    pub workspace: WorkspaceSpec,
    pub attempt_limit: Option<i32>,
}

#[derive(Debug, thiserror::Error)]
pub enum OrgIntelError {
    #[error("invalid company schema name {0:?}")]
    BadSchemaName(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("invalid Work graph: {0}")]
    InvalidWork(String),
}

pub type Result<T> = std::result::Result<T, OrgIntelError>;

/// Company schema names are SQL identifiers injected into DDL — validated so
/// `SET search_path` can never carry injection. Deliberately stricter than
/// company names elsewhere: lowercase, starts with a letter.
fn valid_schema_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name.bytes().enumerate().all(|(index, byte)| {
            if index == 0 {
                matches!(byte, b'a'..=b'z' | b'_')
            } else {
                matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'_')
            }
        })
}

fn valid_runtime_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_git_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.starts_with('-')
        && !value.starts_with('/')
        && !value.ends_with(['/', '.'])
        && !value.contains("..")
        && !value.contains("//")
        && !value.contains("@{")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
}

fn valid_company_cwd(value: &str) -> bool {
    value == "/company"
        || (value.starts_with("/company/") && !value.split('/').any(|part| part == ".."))
}

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// One company's coordination state. Cheap to clone (a pool inside).
#[derive(Clone)]
pub struct OrgIntel {
    pool: PgPool,
    schema: String,
}

impl OrgIntel {
    /// Connectivity check without side effects (daemon boot probe).
    pub async fn probe(database_url: &str) -> Result<()> {
        let options: PgConnectOptions = database_url.parse().map_err(OrgIntelError::Db)?;
        let mut connection = sqlx::ConnectOptions::connect(&options).await?;
        sqlx::query("SELECT 1").execute(&mut connection).await?;
        Ok(())
    }

    /// Table names in this company's schema (observability probe).
    pub async fn table_names(&self) -> Result<Vec<String>> {
        let rows =
            sqlx::query("SELECT tablename FROM pg_tables WHERE schemaname = $1 ORDER BY tablename")
                .bind(&self.schema)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    /// Drop this company's schema and everything in it. Company teardown —
    /// the one destructive operation here, and it is never called implicitly.
    /// Cheap check that this handle's schema still has its tables. A cached
    /// handle survives the schema being dropped underneath it — by an operator,
    /// a scenario reset, or a restore — and then fails every query with
    /// `relation "actors" does not exist`. Reconcile after failure rather than
    /// assuming the world held still (docs/specs/cross-layer-contract.md §18.5).
    pub async fn is_live(&self) -> bool {
        sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass(format('%I.actors', $1))::text")
            .bind(&self.schema)
            .fetch_one(&self.pool)
            .await
            .ok()
            .flatten()
            .is_some()
    }

    pub async fn drop_schema(&self) -> Result<()> {
        sqlx::query(&format!("DROP SCHEMA IF EXISTS {} CASCADE", self.schema))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    /// Ensure the company's schema exists and is migrated, then return a
    /// handle whose connections are pinned to it.
    pub async fn ensure(database_url: &str, company: &str) -> Result<Self> {
        if !valid_schema_name(company) {
            return Err(OrgIntelError::BadSchemaName(company.to_string()));
        }
        // Migrate over a single-connection pool with search_path pinned:
        // CREATE SCHEMA, then the migration set, both inside the schema.
        // (A bare PgConnection here trips a known sqlx Acquire/Send issue
        // when the caller's future is spawned; the pool form does not.)
        let schema = company.to_string();
        let pinned = schema.clone();
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .after_connect(move |connection, _meta| {
                let schema = pinned.clone();
                Box::pin(async move {
                    sqlx::query(&format!("SET search_path TO {schema}"))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(database_url)
            .await?;
        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {company}"))
            .execute(&admin)
            .await?;
        MIGRATOR.run(&admin).await?;
        admin.close().await;

        let pinned = schema.clone();
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _meta| {
                let schema = pinned.clone();
                Box::pin(async move {
                    sqlx::query(&format!("SET search_path TO {schema}"))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(database_url)
            .await?;
        Ok(Self { pool, schema })
    }

    pub fn schema(&self) -> &str {
        &self.schema
    }

    // ---- actors ----

    pub async fn add_actor(&self, id: &str, kind: &str, display: &str) -> Result<()> {
        self.add_actor_with_model(id, kind, display, None).await
    }

    /// S04-T9. The same insert, carrying what this actor thinks with.
    ///
    /// `ON CONFLICT DO NOTHING` on the row, but the model is refreshed: an
    /// actor persists across wakes (`orgintel §2.1`) while the model it is
    /// given can change between them, and the owner asking "which model wrote
    /// this" wants the one that ran, not the one it was first created with.
    pub async fn add_actor_with_model(
        &self,
        id: &str,
        kind: &str,
        display: &str,
        model: Option<&str>,
    ) -> Result<()> {
        if id.trim().is_empty() || kind.trim().is_empty() || display.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "an actor needs a stable id, role and display name".into(),
            ));
        }
        let changed = sqlx::query(
            "INSERT INTO actors (id, kind, display, model) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (id) DO UPDATE SET model = COALESCE(EXCLUDED.model, actors.model) \
             WHERE actors.retired_at IS NULL",
        )
        .bind(id)
        .bind(kind)
        .bind(display)
        .bind(model)
        .execute(&self.pool)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(format!(
                "actor {id:?} is retired; restore must be an explicit organisational decision"
            )));
        }
        Ok(())
    }

    /// Create one durable specialist identity. Runtime wakes use
    /// `add_actor_with_model` to refresh a known active actor's session model;
    /// this is the explicit organisational path for adding a person.
    pub async fn create_actor(
        &self,
        id: &str,
        kind: &str,
        display: &str,
        model: Option<&str>,
        created_by: &str,
        reason: &str,
    ) -> Result<()> {
        let id = id.trim();
        let kind = kind.trim();
        let display = display.trim();
        if id.trim().is_empty() || kind.trim().is_empty() || display.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "an actor needs a stable id, role and display name".into(),
            ));
        }
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "creating a specialist needs the difference this actor buys".into(),
            ));
        }
        if matches!(id, "owner" | "exec" | "world" | "daemon")
            || matches!(kind, "owner" | "exec" | "world" | "daemon")
        {
            return Err(OrgIntelError::InvalidWork(
                "standing company/system actors are bootstrapped, not commissioned as specialists"
                    .into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let may_create: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors a WHERE a.id=$1 AND a.retired_at IS NULL \
             AND (a.id IN ('owner','exec') OR EXISTS(SELECT 1 FROM teams t \
             WHERE t.lead_actor_id=a.id AND t.disbanded_at IS NULL)))",
        )
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_create {
            return Err(OrgIntelError::InvalidWork(
                "only the owner, Exec, or an appointed live team lead may create a specialist"
                    .into(),
            ));
        }

        let changed = sqlx::query(
            "INSERT INTO actors (id, kind, display, model) VALUES ($1,$2,$3,$4) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(kind)
        .bind(display)
        .bind(model)
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(format!(
                "actor {id:?} already exists; assign Work to that durable identity or choose a genuinely new specialist"
            )));
        }
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('actor_created',$1,$2)")
            .bind(created_by)
            .bind(serde_json::json!({
                "actor_id": id,
                "role": kind,
                "display": display,
                "model": model,
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Every actor the company has, for `restless people`.
    pub async fn list_actors(&self) -> Result<Vec<ActorRow>> {
        Ok(sqlx::query_as::<_, ActorRow>(
            "SELECT id, kind, display, model, team_id, retired_at, retired_by, \
                    retirement_reason, created_at FROM actors \
             WHERE retired_at IS NULL ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Active and retired actors for an explicit historical People read.
    pub async fn list_actors_including_retired(&self) -> Result<Vec<ActorRow>> {
        Ok(sqlx::query_as::<_, ActorRow>(
            "SELECT id, kind, display, model, team_id, retired_at, retired_by, \
                    retirement_reason, created_at FROM actors ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Resolve a Work owner without mutating the actor roster.
    pub async fn active_actor(&self, actor_id: &str) -> Result<Option<ActorRow>> {
        Ok(sqlx::query_as::<_, ActorRow>(
            "SELECT id, kind, display, model, team_id, retired_at, retired_by, \
                    retirement_reason, created_at FROM actors \
             WHERE id=$1 AND retired_at IS NULL",
        )
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Retire a durable actor without erasing attribution. Historical Work,
    /// messages and effects continue to name the same actor; only future
    /// assignment and owner surfaces stop treating it as available.
    pub async fn retire_actor(&self, actor_id: &str, retired_by: &str, reason: &str) -> Result<()> {
        if matches!(actor_id, "owner" | "exec" | "world" | "daemon") {
            return Err(OrgIntelError::InvalidWork(format!(
                "{actor_id} is a standing company/system actor and cannot be retired"
            )));
        }
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "retiring an actor needs a reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let may_retire: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(retired_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_retire {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may retire a durable actor".into(),
            ));
        }
        let row = sqlx::query(
            "SELECT a.team_id, EXISTS(SELECT 1 FROM teams t WHERE t.lead_actor_id=a.id \
             AND t.disbanded_at IS NULL) AS leads_team FROM actors a \
             WHERE a.id=$1 AND a.retired_at IS NULL FOR UPDATE",
        )
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!("active actor {actor_id:?} does not exist"))
        })?;
        if row.get::<bool, _>("leads_team") {
            return Err(OrgIntelError::InvalidWork(
                "appoint another lead or disband the team before retiring its lead".into(),
            ));
        }
        let owns_open_work: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work WHERE owner_id=$1 \
             AND status IN ('proposed','active','blocked'))",
        )
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if owns_open_work {
            return Err(OrgIntelError::InvalidWork(
                "reassign or close the actor's open Work before retiring it".into(),
            ));
        }
        let owes_judgement: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM owner_handoffs WHERE assigned_to=$1 AND state='pending')",
        )
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if owes_judgement {
            return Err(OrgIntelError::InvalidWork(
                "resolve or escalate the actor's pending judgement before retiring it".into(),
            ));
        }
        sqlx::query(
            "UPDATE actors SET retired_at=now(), retired_by=$2, retirement_reason=$3, \
             team_id=NULL WHERE id=$1",
        )
        .bind(actor_id)
        .bind(retired_by)
        .bind(reason.trim())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('actor_retired',$1,$2)")
            .bind(retired_by)
            .bind(serde_json::json!({
                "actor_id": actor_id,
                "previous_team_id": row.get::<Option<Uuid>, _>("team_id"),
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- teams (S06-T4) ----
    //
    // A team is coordination, not authority. It grants no effect permission, no
    // budget, no credential scope and no approval right, and no kernel record
    // carries a team. Its whole job is to give escalation a destination that is
    // not the human, and the owner one actor per team that can answer for it.

    /// Form a team around an accountable lead. The lead joins its own team.
    pub async fn create_team(
        &self,
        name: &str,
        brief: &str,
        lead_actor_id: &str,
        created_by: &str,
    ) -> Result<Uuid> {
        if name.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork("a team needs a name".into()));
        }
        if brief.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "commissioning a team needs its outcome charter".into(),
            ));
        }
        if matches!(lead_actor_id, "owner" | "exec" | "world" | "daemon") {
            return Err(OrgIntelError::InvalidWork(
                "a standing company/system actor cannot be appointed as a team lead".into(),
            ));
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        let commissioner_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;
        if !commissioner_exists {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may commission a team".into(),
            ));
        }
        let lead = sqlx::query(
            "SELECT team_id, EXISTS(SELECT 1 FROM teams WHERE lead_actor_id=$1 \
             AND disbanded_at IS NULL) AS leads_team FROM actors \
             WHERE id=$1 AND retired_at IS NULL FOR UPDATE",
        )
        .bind(lead_actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!(
                "active lead actor {lead_actor_id:?} does not exist"
            ))
        })?;
        if lead.get::<Option<Uuid>, _>("team_id").is_some() || lead.get::<bool, _>("leads_team") {
            return Err(OrgIntelError::InvalidWork(
                "a commissioned lead must be unassigned and must not already lead a team".into(),
            ));
        }
        sqlx::query(
            "INSERT INTO teams (id, name, brief, lead_actor_id, created_by) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id)
        .bind(name.trim())
        .bind(brief.trim())
        .bind(lead_actor_id)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;
        let joined = sqlx::query(
            "UPDATE actors SET team_id=$2 WHERE id=$1 AND retired_at IS NULL AND team_id IS NULL",
        )
        .bind(lead_actor_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        if joined.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(
                "lead assignment changed while the team was being commissioned".into(),
            ));
        }
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('team_commissioned',$1,$2)")
            .bind(created_by)
            .bind(serde_json::json!({
                "team_id": id,
                "name": name.trim(),
                "brief": brief.trim(),
                "lead_actor_id": lead_actor_id,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Live teams, oldest first. Disbanded teams keep their record and are not
    /// returned — they are history, not structure.
    pub async fn list_teams(&self) -> Result<Vec<TeamRow>> {
        Ok(sqlx::query_as::<_, TeamRow>(
            "SELECT id, name, brief, lead_actor_id, created_by, created_at, disbanded_at \
             FROM teams WHERE disbanded_at IS NULL ORDER BY created_at, id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Change a live team's display name or charter. This remains ordinary,
    /// recoverable coordination state; only the owner or Exec may widen the
    /// outcome a lead was commissioned to pursue.
    pub async fn update_team(
        &self,
        team_id: Uuid,
        name: Option<&str>,
        brief: Option<&str>,
        changed_by: &str,
        reason: &str,
    ) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "changing a team needs a reason".into(),
            ));
        }
        let name = name.map(str::trim).filter(|value| !value.is_empty());
        let brief = brief.map(str::trim).filter(|value| !value.is_empty());
        if name.is_none() && brief.is_none() {
            return Err(OrgIntelError::InvalidWork(
                "provide a non-empty name or brief to change".into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let may_override: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_override {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may change a team's name or charter".into(),
            ));
        }
        let before = sqlx::query(
            "SELECT name, brief FROM teams WHERE id=$1 AND disbanded_at IS NULL FOR UPDATE",
        )
        .bind(team_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("no live team with that id".into()))?;
        let old_name: String = before.get("name");
        let old_brief: String = before.get("brief");
        let new_name = name.unwrap_or(&old_name);
        let new_brief = brief.unwrap_or(&old_brief);
        if new_name == old_name && new_brief == old_brief {
            return Err(OrgIntelError::InvalidWork(
                "the proposed team name and charter are unchanged".into(),
            ));
        }

        sqlx::query("UPDATE teams SET name=$2, brief=$3 WHERE id=$1")
            .bind(team_id)
            .bind(new_name)
            .bind(new_brief)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('team_updated',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "team_id": team_id,
                "from_name": old_name,
                "to_name": new_name,
                "from_brief": old_brief,
                "to_brief": new_brief,
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Claim an unassigned direct member, release a direct member, or apply an
    /// owner/Exec override. The caller and the composition reason remain in the
    /// recoverable event stream; team membership itself has one writer here.
    pub async fn set_actor_team(
        &self,
        actor_id: &str,
        team_id: Option<Uuid>,
        changed_by: &str,
        reason: &str,
    ) -> Result<()> {
        if matches!(actor_id, "owner" | "exec" | "world" | "daemon") {
            return Err(OrgIntelError::InvalidWork(
                "standing company/system actors do not belong to a team".into(),
            ));
        }
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "changing a roster needs the difference or repair this member buys".into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let member = sqlx::query(
            "SELECT team_id, EXISTS(SELECT 1 FROM teams WHERE lead_actor_id=$1 \
             AND disbanded_at IS NULL) AS leads_team FROM actors \
             WHERE id=$1 AND retired_at IS NULL FOR UPDATE",
        )
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!("active actor {actor_id:?} does not exist"))
        })?;
        let previous_team: Option<Uuid> = member.get("team_id");
        if member.get::<bool, _>("leads_team") {
            return Err(OrgIntelError::InvalidWork(
                "a live lead cannot be released or poached; replace the lead explicitly first"
                    .into(),
            ));
        }
        if previous_team == team_id {
            return Err(OrgIntelError::InvalidWork(
                "the actor already has that team assignment".into(),
            ));
        }

        if let Some(target) = team_id {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM teams WHERE id=$1 AND disbanded_at IS NULL)",
            )
            .bind(target)
            .fetch_one(&mut *tx)
            .await?;
            if !exists {
                return Err(OrgIntelError::InvalidWork(
                    "no live team with that id".into(),
                ));
            }
        }

        let coordinator = sqlx::query(
            "SELECT a.id, t.id AS led_team FROM actors a LEFT JOIN teams t \
             ON t.lead_actor_id=a.id AND t.disbanded_at IS NULL \
             WHERE a.id=$1 AND a.retired_at IS NULL",
        )
        .bind(changed_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!(
                "active coordinating actor {changed_by:?} does not exist"
            ))
        })?;
        let override_roster = matches!(changed_by, "owner" | "exec");
        if !override_roster {
            let led_team: Option<Uuid> = coordinator.get("led_team");
            let Some(led_team) = led_team else {
                return Err(OrgIntelError::InvalidWork(
                    "only the owner, Exec, or this roster's appointed lead may change it".into(),
                ));
            };
            let is_claim = previous_team.is_none() && team_id == Some(led_team);
            let is_release = previous_team == Some(led_team) && team_id.is_none();
            if !is_claim && !is_release {
                return Err(OrgIntelError::InvalidWork(
                    "a lead may only claim an unassigned actor into its own team or release one of its direct members"
                        .into(),
                ));
            }
            if is_release {
                let owns_open_work: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM work WHERE owner_id=$1 \
                     AND status IN ('proposed','active','blocked'))",
                )
                .bind(actor_id)
                .fetch_one(&mut *tx)
                .await?;
                if owns_open_work {
                    return Err(OrgIntelError::InvalidWork(
                        "reassign or settle the member's open Work before releasing it".into(),
                    ));
                }
            }
        }

        sqlx::query("UPDATE actors SET team_id=$2 WHERE id=$1")
            .bind(actor_id)
            .bind(team_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('team_roster_changed',$1,$2)",
        )
        .bind(changed_by)
        .bind(serde_json::json!({
            "member_actor_id": actor_id,
            "from_team_id": previous_team,
            "to_team_id": team_id,
            "reason": reason.trim(),
            "override": override_roster,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Replace a team's lead. The new lead joins the team it now leads.
    pub async fn set_team_lead(
        &self,
        team_id: Uuid,
        lead_actor_id: &str,
        changed_by: &str,
        reason: &str,
    ) -> Result<()> {
        if matches!(lead_actor_id, "owner" | "exec" | "world" | "daemon") {
            return Err(OrgIntelError::InvalidWork(
                "a standing company/system actor cannot be appointed as a team lead".into(),
            ));
        }
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "replacing a lead needs a reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let may_override: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_override {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may replace an accountable lead".into(),
            ));
        }
        let previous_lead: Option<String> = sqlx::query_scalar(
            "SELECT lead_actor_id FROM teams WHERE id=$1 AND disbanded_at IS NULL FOR UPDATE",
        )
        .bind(team_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(previous_lead) = previous_lead else {
            return Err(OrgIntelError::InvalidWork(
                "no live team with that id".into(),
            ));
        };
        if previous_lead == lead_actor_id {
            return Err(OrgIntelError::InvalidWork(
                "that actor already leads this team".into(),
            ));
        }
        let new_lead = sqlx::query(
            "SELECT team_id, EXISTS(SELECT 1 FROM teams WHERE lead_actor_id=$1 \
             AND disbanded_at IS NULL) AS leads_team FROM actors \
             WHERE id=$1 AND retired_at IS NULL FOR UPDATE",
        )
        .bind(lead_actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!(
                "active lead actor {lead_actor_id:?} does not exist"
            ))
        })?;
        if new_lead.get::<bool, _>("leads_team") {
            return Err(OrgIntelError::InvalidWork(
                "one lead cannot lead another lead or a second team".into(),
            ));
        }
        let previous_team: Option<Uuid> = new_lead.get("team_id");
        if previous_team.is_some() {
            return Err(OrgIntelError::InvalidWork(
                "a replacement lead must be unassigned; release or move the actor explicitly before appointment"
                    .into(),
            ));
        }
        sqlx::query("UPDATE teams SET lead_actor_id=$2 WHERE id=$1")
            .bind(team_id)
            .bind(lead_actor_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE actors SET team_id=$2 WHERE id=$1")
            .bind(lead_actor_id)
            .bind(team_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('team_lead_changed',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "team_id": team_id,
                "from_lead_actor_id": previous_lead,
                "to_lead_actor_id": lead_actor_id,
                "new_lead_previous_team_id": previous_team,
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Disband a team. Members become unassigned rather than orphaned, and any
    /// judgement the team still owed falls through to the Exec with the
    /// fall-through recorded — never silently dropped.
    pub async fn disband_team(&self, team_id: Uuid, changed_by: &str, reason: &str) -> Result<u64> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "disbanding a team needs a reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let may_override: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND id IN ('owner','exec'))",
        )
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;
        if !may_override {
            return Err(OrgIntelError::InvalidWork(
                "only the owner or Exec may disband a team".into(),
            ));
        }
        let changed =
            sqlx::query("UPDATE teams SET disbanded_at=now() WHERE id=$1 AND disbanded_at IS NULL")
                .bind(team_id)
                .execute(&mut *tx)
                .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(
                "no live team with that id".into(),
            ));
        }
        let stranded = sqlx::query(
            "UPDATE owner_handoffs SET assigned_to='exec', escalated_from=assigned_to, \
             escalated_at=now(), resolution=$2 \
             WHERE state='pending' AND assigned_to IN (SELECT id FROM actors WHERE team_id=$1)",
        )
        .bind(team_id)
        .bind(format!("team disbanded: {reason}"))
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE actors SET team_id=NULL WHERE team_id=$1")
            .bind(team_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('team_disbanded',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "team_id": team_id,
                "reason": reason.trim(),
                "judgements_reassigned": stranded.rows_affected(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(stranded.rows_affected())
    }

    /// The lead accountable for this actor, or `None` when it has no team, or
    /// when it *is* the lead — a lead does not escalate to itself.
    pub async fn team_lead_for(&self, actor_id: &str) -> Result<Option<String>> {
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT t.lead_actor_id FROM actors a JOIN teams t ON t.id = a.team_id \
             WHERE a.id = $1 AND t.disbanded_at IS NULL AND t.lead_actor_id <> $1",
        )
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Judgement this actor owes. This is the lead's queue, and it is the same
    /// shape as the owner's — the point of a lead is that the two are the same
    /// job at different altitudes.
    pub async fn handoffs_assigned_to(&self, actor_id: &str) -> Result<Vec<OwnerHandoffRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, attempt_id, requested_by, category, requested_action, \
                    prepared_state, resume_condition, state, resolution, assigned_to, \
                    escalated_from, escalated_at, created_at, resolved_at \
             FROM owner_handoffs WHERE state='pending' AND assigned_to=$1 \
             ORDER BY created_at, id",
        )
        .bind(actor_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Pass a judgement up one altitude because it is outside this actor's
    /// remit. Leads and unassigned specialists go to the Exec; only the Exec
    /// can pass ordinary judgement to the owner.
    pub async fn escalate_handoff(&self, id: Uuid, from_actor: &str, reason: &str) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "escalating to the owner needs a reason: the owner is being asked for time, and \
                 an unexplained handoff is the cost this is meant to remove"
                    .into(),
            ));
        }
        let next = (from_actor != "exec").then_some("exec");
        let changed = sqlx::query(
            "UPDATE owner_handoffs SET assigned_to=$3, escalated_from=$2, escalated_at=now(), \
             resolution=$4 WHERE id=$1 AND state='pending' AND assigned_to=$2",
        )
        .bind(id)
        .bind(from_actor)
        .bind(next)
        .bind(reason.trim())
        .execute(&self.pool)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(
                "no pending handoff is assigned to that actor".into(),
            ));
        }
        Ok(())
    }

    /// Preserve judgement when an assigned coordinator cannot run. Runtime
    /// supervision calls this only after an observed blocked/crashed lead
    /// turn; the existing row moves to Exec rather than being copied or lost.
    pub async fn fallthrough_handoffs_to_exec(
        &self,
        unavailable_actor: &str,
        reason: &str,
    ) -> Result<u64> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "judgement fall-through needs the observed failure reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let changed = sqlx::query(
            "UPDATE owner_handoffs SET assigned_to='exec', escalated_from=$1, \
             escalated_at=now(), resolution=$2 \
             WHERE state='pending' AND assigned_to=$1",
        )
        .bind(unavailable_actor)
        .bind(reason.trim())
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() > 0 {
            sqlx::query(
                "INSERT INTO events (kind, actor_id, body) VALUES ('judgement_fell_through',$1,$2)",
            )
            .bind(unavailable_actor)
            .bind(serde_json::json!({
                "to_actor": "exec",
                "count": changed.rows_affected(),
                "reason": reason.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(changed.rows_affected())
    }

    // ---- goals ----

    pub async fn add_goal(&self, title: &str, body: &str, created_by: &str) -> Result<Uuid> {
        if title.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork("a Goal needs a title".into()));
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        let creator_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;
        if !creator_exists {
            return Err(OrgIntelError::InvalidWork(format!(
                "Goal creator {created_by:?} is not an existing active actor"
            )));
        }
        sqlx::query("INSERT INTO goals (id, title, body, created_by) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(title.trim())
            .bind(body.trim())
            .bind(created_by)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('goal_created',$1,$2)")
            .bind(created_by)
            .bind(serde_json::json!({
                "goal_id": id,
                "title": title.trim(),
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    pub async fn list_goals(&self) -> Result<Vec<GoalRow>> {
        Ok(sqlx::query_as(
            "SELECT id, title, body, created_by, created_at, closed_at FROM goals \
             ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Attach or reassign existing Work to an existing Goal. This is ordinary
    /// recoverable coordination: the event explains the change, but the Work
    /// row remains the single current truth.
    pub async fn set_work_goal(
        &self,
        work_id: Uuid,
        goal_id: Uuid,
        changed_by: &str,
    ) -> Result<Option<Uuid>> {
        let mut tx = self.pool.begin().await?;
        let actor_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(changed_by)
        .fetch_one(&mut *tx)
        .await?;
        if !actor_exists {
            return Err(OrgIntelError::InvalidWork(format!(
                "Goal assignment actor {changed_by:?} is not an existing active actor"
            )));
        }
        let goal_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM goals WHERE id=$1)")
                .bind(goal_id)
                .fetch_one(&mut *tx)
                .await?;
        if !goal_exists {
            return Err(OrgIntelError::InvalidWork(format!(
                "Goal {goal_id} does not exist in this company"
            )));
        }
        let previous: Option<Uuid> =
            sqlx::query_scalar("SELECT goal_id FROM work WHERE id=$1 FOR UPDATE")
                .bind(work_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidWork(format!("Work {work_id} does not exist"))
                })?;
        if previous == Some(goal_id) {
            tx.commit().await?;
            return Ok(previous);
        }
        sqlx::query("UPDATE work SET goal_id=$2, updated_at=now() WHERE id=$1")
            .bind(work_id)
            .bind(goal_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('work_goal_assigned',$1,$2)",
        )
        .bind(changed_by)
        .bind(serde_json::json!({
            "work_id": work_id,
            "goal_id": goal_id,
            "previous_goal_id": previous,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(previous)
    }

    // ---- Work graph ----

    pub async fn add_work(&self, work: NewWork<'_>) -> Result<Uuid> {
        self.add_work_with_edges(work, &[], &[]).await
    }

    /// Add a Work node and the dependencies that define its initial readiness
    /// in one transaction. PostgreSQL delivers the insert notification only
    /// after commit, so the scheduler can never claim the node between its
    /// creation and its initial edges.
    pub async fn add_work_with_edges(
        &self,
        work: NewWork<'_>,
        requires: &[Uuid],
        revises: &[Uuid],
    ) -> Result<Uuid> {
        if work.title.trim().is_empty() || work.outcome.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "Work needs a title and outcome contract".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let owner_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(work.owner_id)
        .fetch_one(&mut *tx)
        .await?;
        if !owner_exists {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work owner {:?} is not an existing active actor; inspect People and commission one durable specialist if none fits",
                work.owner_id
            )));
        }
        if let Some(goal_id) = work.goal_id {
            let goal_exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM goals WHERE id=$1)")
                    .bind(goal_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if !goal_exists {
                return Err(OrgIntelError::InvalidWork(format!(
                    "Goal {goal_id} does not exist in this company"
                )));
            }
        }
        if work
            .workspace
            .repo
            .as_deref()
            .is_some_and(|value| !valid_runtime_slug(value))
            || work
                .workspace
                .worktree
                .as_deref()
                .is_some_and(|value| !valid_runtime_slug(value))
        {
            return Err(OrgIntelError::InvalidWork(
                "repo and worktree must be lowercase runtime slugs".into(),
            ));
        }
        if work
            .workspace
            .base_ref
            .as_deref()
            .is_some_and(|value| !valid_git_ref(value))
            || work
                .workspace
                .integration_branch
                .as_deref()
                .is_some_and(|value| !valid_git_ref(value))
        {
            return Err(OrgIntelError::InvalidWork(
                "base and integration branch must be bounded Git refs".into(),
            ));
        }
        let id = Uuid::new_v4();
        let mut initial_edges = std::collections::HashSet::new();
        for (kind, targets) in [
            (WorkEdgeKind::Requires, requires),
            (WorkEdgeKind::Revises, revises),
        ] {
            for target in targets {
                if !initial_edges.insert((kind, *target)) {
                    continue;
                }
                let exists: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM work WHERE id=$1)")
                        .bind(target)
                        .fetch_one(&mut *tx)
                        .await?;
                if !exists {
                    return Err(OrgIntelError::InvalidWork(format!(
                        "initial {kind:?} target {target} does not exist"
                    )));
                }
            }
        }

        sqlx::query(
            "INSERT INTO work \
             (id, goal_id, owner_id, title, outcome, priority, expected_artifact, \
              repo, base_ref, integration_branch, worktree, attempt_limit) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
        )
        .bind(id)
        .bind(work.goal_id)
        .bind(work.owner_id)
        .bind(work.title)
        .bind(work.outcome)
        .bind(work.priority)
        .bind(work.expected_artifact)
        .bind(work.workspace.repo)
        .bind(work.workspace.base_ref)
        .bind(work.workspace.integration_branch)
        .bind(work.workspace.worktree)
        .bind(work.attempt_limit)
        .execute(&mut *tx)
        .await?;
        for (kind, target) in initial_edges {
            let (from, to) = match kind {
                WorkEdgeKind::Requires => (target, id),
                WorkEdgeKind::Revises => (id, target),
            };
            sqlx::query(
                "INSERT INTO work_edges (from_work_id, to_work_id, kind) VALUES ($1,$2,$3)",
            )
            .bind(from)
            .bind(to)
            .bind(kind)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    pub async fn list_work(&self) -> Result<Vec<WorkRow>> {
        Ok(sqlx::query_as(
            "SELECT id, goal_id, owner_id, title, outcome, status, resolution, priority, \
             expected_artifact, repo, base_ref, integration_branch, worktree, revision, \
             attempt_limit, created_at, updated_at FROM work \
             ORDER BY priority DESC, created_at",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get_work(&self, id: Uuid) -> Result<Option<WorkRow>> {
        Ok(sqlx::query_as(
            "SELECT id, goal_id, owner_id, title, outcome, status, resolution, priority, \
             expected_artifact, repo, base_ref, integration_branch, worktree, revision, \
             attempt_limit, created_at, updated_at FROM work WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Move an unsettled responsibility to another durable actor. This is
    /// ordinary recoverable coordination: owner/Exec may repair any assignment,
    /// while a lead may only move Work between members of the team it leads.
    pub async fn reassign_work(
        &self,
        work_id: Uuid,
        new_owner_id: &str,
        changed_by: &str,
        reason: &str,
    ) -> Result<String> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "reassigning Work needs the outcome or repair this change buys".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT owner_id, status FROM work WHERE id=$1 FOR UPDATE")
            .bind(work_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| OrgIntelError::InvalidWork(format!("Work {work_id} does not exist")))?;
        let previous_owner: String = row.get("owner_id");
        let status: WorkStatus = row.get("status");
        if matches!(status, WorkStatus::Completed | WorkStatus::Abandoned) {
            return Err(OrgIntelError::InvalidWork(
                "settled Work is historical; revise it instead of rewriting its owner".into(),
            ));
        }
        if previous_owner == new_owner_id {
            return Err(OrgIntelError::InvalidWork(
                "Work is already assigned to that actor".into(),
            ));
        }
        let new_owner_team: Option<Uuid> =
            sqlx::query_scalar("SELECT team_id FROM actors WHERE id=$1 AND retired_at IS NULL")
                .bind(new_owner_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidWork(format!(
                        "new Work owner {new_owner_id:?} is not an existing active actor"
                    ))
                })?;
        let coordinating_actor = sqlx::query(
            "SELECT a.id, a.team_id, t.id AS led_team FROM actors a LEFT JOIN teams t \
             ON t.lead_actor_id=a.id AND t.disbanded_at IS NULL \
             WHERE a.id=$1 AND a.retired_at IS NULL",
        )
        .bind(changed_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(format!("coordinating actor {changed_by:?} is not active"))
        })?;
        let override_assignment = matches!(changed_by, "owner" | "exec");
        if !override_assignment {
            let led_team: Option<Uuid> = coordinating_actor.get("led_team");
            let previous_owner_team: Option<Uuid> =
                sqlx::query_scalar("SELECT team_id FROM actors WHERE id=$1 AND retired_at IS NULL")
                    .bind(&previous_owner)
                    .fetch_optional(&mut *tx)
                    .await?
                    .flatten();
            if led_team.is_none() || previous_owner_team != led_team || new_owner_team != led_team {
                return Err(OrgIntelError::InvalidWork(
                    "a lead may only reassign Work between active members of its own team".into(),
                ));
            }
        }
        let attempt_running: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_attempts WHERE work_id=$1 AND state='running')",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        if attempt_running {
            return Err(OrgIntelError::InvalidWork(
                "stop or settle the running Attempt before reassigning its Work".into(),
            ));
        }

        sqlx::query("UPDATE work SET owner_id=$2, updated_at=now() WHERE id=$1")
            .bind(work_id)
            .bind(new_owner_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_reassigned',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "work_id": work_id,
                "from_actor_id": previous_owner,
                "to_actor_id": new_owner_id,
                "reason": reason.trim(),
                "override": override_assignment,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(previous_owner)
    }

    /// Add one graph edge. Only hard prerequisites must be acyclic. Revision
    /// edges deliberately close producer/reviewer loops; each traversal creates
    /// a new Work revision and therefore a new, ordered Attempt generation.
    pub async fn add_work_edge(&self, from: Uuid, to: Uuid, kind: WorkEdgeKind) -> Result<()> {
        if from == to {
            return Err(OrgIntelError::InvalidWork(
                "a Work node cannot depend on itself".into(),
            ));
        }
        if kind == WorkEdgeKind::Requires {
            let closes_cycle = sqlx::query_scalar::<_, bool>(
                "WITH RECURSIVE reachable(id) AS (\
                   SELECT to_work_id FROM work_edges \
                    WHERE from_work_id = $1 AND kind = 'requires' \
                   UNION \
                   SELECT edge.to_work_id FROM work_edges edge \
                    JOIN reachable prior ON edge.from_work_id = prior.id \
                    WHERE edge.kind = 'requires'\
                 ) SELECT EXISTS(SELECT 1 FROM reachable WHERE id = $2)",
            )
            .bind(to)
            .bind(from)
            .fetch_one(&self.pool)
            .await?;
            if closes_cycle {
                return Err(OrgIntelError::InvalidWork(format!(
                    "requires edge {from} -> {to} closes a hard dependency cycle; use a revises edge for feedback"
                )));
            }
        }
        sqlx::query(
            "INSERT INTO work_edges (from_work_id, to_work_id, kind) VALUES ($1,$2,$3) \
             ON CONFLICT DO NOTHING",
        )
        .bind(from)
        .bind(to)
        .bind(kind)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Remove a mistaken dependency during local graph repair. This remains
    /// recoverable OrgIntel coordination, but a lead is bounded to Work owned
    /// by actors in its own team.
    pub async fn remove_work_edge(
        &self,
        from: Uuid,
        to: Uuid,
        kind: WorkEdgeKind,
        changed_by: &str,
        reason: &str,
    ) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "removing a Work edge needs the observed repair reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let may_override = matches!(changed_by, "owner" | "exec");
        if !may_override {
            let led_team: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM teams WHERE lead_actor_id=$1 AND disbanded_at IS NULL",
            )
            .bind(changed_by)
            .fetch_optional(&mut *tx)
            .await?;
            let Some(led_team) = led_team else {
                return Err(OrgIntelError::InvalidWork(
                    "only the owner, Exec, or the accountable team lead may repair graph edges"
                        .into(),
                ));
            };
            let within_team: bool = sqlx::query_scalar(
                "SELECT EXISTS(\
                   SELECT 1 FROM work wf JOIN actors af ON af.id=wf.owner_id, \
                                      work wt JOIN actors at ON at.id=wt.owner_id \
                   WHERE wf.id=$1 AND wt.id=$2 AND af.team_id=$3 AND at.team_id=$3\
                 )",
            )
            .bind(from)
            .bind(to)
            .bind(led_team)
            .fetch_one(&mut *tx)
            .await?;
            if !within_team {
                return Err(OrgIntelError::InvalidWork(
                    "a lead may only repair dependencies between Work owned by its own team".into(),
                ));
            }
        }
        let changed = sqlx::query(
            "DELETE FROM work_edges WHERE from_work_id=$1 AND to_work_id=$2 AND kind=$3",
        )
        .bind(from)
        .bind(to)
        .bind(kind)
        .execute(&mut *tx)
        .await?;
        if changed.rows_affected() != 1 {
            return Err(OrgIntelError::InvalidWork(
                "that Work edge does not exist".into(),
            ));
        }
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_edge_removed',$1,$2)")
            .bind(changed_by)
            .bind(serde_json::json!({
                "from_work_id": from,
                "to_work_id": to,
                "kind": kind,
                "reason": reason.trim(),
                "override": may_override,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_work_edges(&self) -> Result<Vec<WorkEdgeRow>> {
        Ok(sqlx::query_as(
            "SELECT from_work_id, to_work_id, kind, created_at FROM work_edges \
             ORDER BY created_at, from_work_id, to_work_id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Atomically lease the highest-priority ready Work node. Readiness is a
    /// database fact: all hard requirements completed, no pending owner
    /// handoff, no live Attempt, and any explicit attempt limit still has room.
    pub async fn claim_ready_work(&self, trigger: &str) -> Result<Option<ClaimedWork>> {
        self.claim_ready_work_excluding(trigger, &[]).await
    }

    /// The scheduler's claim path. A supervised conversation already owns its
    /// actor outside the Work graph, so exclude those actor ids before taking a
    /// database lease. Skipping is not failure: no Attempt row is created and
    /// the Work remains ready for the next scan.
    pub async fn claim_ready_work_excluding(
        &self,
        trigger: &str,
        excluded_actor_ids: &[String],
    ) -> Result<Option<ClaimedWork>> {
        let mut tx = self.pool.begin().await?;
        let work = sqlx::query_as::<_, WorkRow>(
            "SELECT w.id, w.goal_id, w.owner_id, w.title, w.outcome, w.status, \
                    w.resolution, w.priority, w.expected_artifact, w.repo, w.base_ref, \
                    w.integration_branch, w.worktree, w.revision, w.attempt_limit, \
                    w.created_at, w.updated_at \
             FROM work w \
             WHERE w.status IN ('proposed','active') \
               AND w.owner_id <> ALL($1::text[]) \
               AND NOT EXISTS (SELECT 1 FROM work_attempts a \
                               WHERE a.work_id = w.id AND a.state = 'running') \
               AND NOT EXISTS (SELECT 1 FROM work_attempts a \
                               WHERE a.actor_id = w.owner_id AND a.state = 'running') \
               AND NOT EXISTS (SELECT 1 FROM owner_handoffs h \
                               WHERE h.work_id = w.id AND h.state = 'pending') \
               AND NOT EXISTS (\
                 SELECT 1 FROM work_edges e JOIN work upstream ON upstream.id = e.from_work_id \
                 WHERE e.to_work_id = w.id AND e.kind = 'requires' \
                   AND upstream.status <> 'completed'\
               ) \
               AND (w.attempt_limit IS NULL OR (\
                    SELECT count(*) FROM work_attempts a \
                    WHERE a.work_id = w.id AND a.revision = w.revision\
               ) < w.attempt_limit) \
             ORDER BY w.priority DESC, w.created_at \
             FOR UPDATE OF w SKIP LOCKED LIMIT 1",
        )
        .bind(excluded_actor_ids)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(work) = work else {
            tx.commit().await?;
            return Ok(None);
        };

        let inputs = sqlx::query_as::<_, ArtifactRefRow>(
            "SELECT a.id, a.kind, a.uri, a.note, a.created_by, a.work_id, a.attempt_id, \
                    a.digest, a.source_commit, a.runtime_generation, a.label, a.state, \
                    a.created_at, a.superseded_at \
             FROM work_edges e JOIN artifact_refs a ON a.work_id = e.from_work_id \
             WHERE e.to_work_id = $1 AND e.kind = 'requires' AND a.state = 'available' \
             ORDER BY e.from_work_id, a.created_at, a.id",
        )
        .bind(work.id)
        .fetch_all(&mut *tx)
        .await?;
        let mut fingerprint_source = inputs
            .iter()
            .map(|artifact| {
                format!(
                    "{}:{}:{}",
                    artifact.id,
                    artifact.digest.as_deref().unwrap_or("unknown"),
                    artifact.source_commit.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let feedback = sqlx::query_as::<_, MessageRow>(
            "SELECT id, from_actor, to_actor, body, created_at, read_at FROM (\
               SELECT m.id, m.from_actor, m.to_actor, m.body, m.created_at, m.read_at \
               FROM work_feedback f JOIN messages m ON m.id=f.message_id \
               WHERE f.work_id=$1 ORDER BY m.id DESC LIMIT 100\
             ) recent ORDER BY id",
        )
        .bind(work.id)
        .fetch_all(&mut *tx)
        .await?;
        let feedback_cursor = feedback.last().map(|message| message.id).unwrap_or(0);
        for message in &feedback {
            fingerprint_source.push_str(&format!(
                "\nfeedback:{}:{:x}",
                message.id,
                Sha256::digest(message.body.as_bytes())
            ));
        }
        let input_fingerprint = format!("{:x}", Sha256::digest(fingerprint_source.as_bytes()));
        let attempt_no: i64 = sqlx::query_scalar(
            "SELECT count(*) + 1 FROM work_attempts WHERE work_id = $1 AND revision = $2",
        )
        .bind(work.id)
        .bind(work.revision)
        .fetch_one(&mut *tx)
        .await?;
        let attempt_id = Uuid::new_v4();
        let session_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO work_attempts \
             (id, work_id, revision, attempt_no, actor_id, session_id, trigger, input_fingerprint, feedback_cursor) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(attempt_id)
        .bind(work.id)
        .bind(work.revision)
        .bind(i32::try_from(attempt_no).unwrap_or(i32::MAX))
        .bind(&work.owner_id)
        .bind(&session_id)
        .bind(trigger)
        .bind(&input_fingerprint)
        .bind(feedback_cursor)
        .execute(&mut *tx)
        .await?;
        for artifact in &inputs {
            sqlx::query(
                "INSERT INTO work_attempt_inputs (attempt_id, artifact_ref_id) VALUES ($1,$2)",
            )
            .bind(attempt_id)
            .bind(artifact.id)
            .execute(&mut *tx)
            .await?;
        }
        for message in &feedback {
            sqlx::query(
                "INSERT INTO work_attempt_feedback (attempt_id, message_id) VALUES ($1,$2)",
            )
            .bind(attempt_id)
            .bind(message.id)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE work SET status = 'active', resolution = '' WHERE id = $1")
            .bind(work.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Some(ClaimedWork {
            work,
            attempt_id,
            attempt_no: i32::try_from(attempt_no).unwrap_or(i32::MAX),
            session_id,
            input_fingerprint,
            inputs,
            feedback,
        }))
    }

    pub async fn set_attempt_model(&self, attempt_id: Uuid, model: &str) -> Result<()> {
        sqlx::query("UPDATE work_attempts SET model = $2 WHERE id = $1 AND state = 'running'")
            .bind(attempt_id)
            .bind(model)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_work_attempts(&self, work_id: Option<Uuid>) -> Result<Vec<WorkAttemptRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, revision, attempt_no, actor_id, session_id, state, trigger, \
                    input_fingerprint, feedback_cursor, model, started_at, finished_at, summary \
             FROM work_attempts WHERE ($1::uuid IS NULL OR work_id = $1) \
             ORDER BY started_at, id",
        )
        .bind(work_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Close one running Attempt and deterministically advance the graph.
    /// A result from an old revision is retained as history but can never
    /// complete the current Work revision.
    pub async fn finish_work_attempt(
        &self,
        attempt_id: Uuid,
        result: WorkAttemptState,
        summary: &str,
    ) -> Result<WorkAttemptState> {
        if result == WorkAttemptState::Running {
            return Err(OrgIntelError::InvalidWork(
                "finishing an Attempt requires a terminal result".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT a.work_id, a.actor_id, a.revision AS attempt_revision, a.state, \
                    w.revision AS work_revision \
             FROM work_attempts a JOIN work w ON w.id = a.work_id \
             WHERE a.id = $1 FOR UPDATE OF a, w",
        )
        .bind(attempt_id)
        .fetch_one(&mut *tx)
        .await?;
        let work_id: Uuid = row.get("work_id");
        let attempt_actor: String = row.get("actor_id");
        let attempt_revision: i64 = row.get("attempt_revision");
        let work_revision: i64 = row.get("work_revision");
        let current_state: WorkAttemptState = row.get("state");
        if current_state != WorkAttemptState::Running {
            tx.commit().await?;
            return Ok(current_state);
        }
        let mut effective = if attempt_revision == work_revision {
            result
        } else {
            WorkAttemptState::Superseded
        };
        let mut effective_summary = summary.to_string();
        if effective == WorkAttemptState::ChangesRequested {
            let has_revision_target: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM work_edges WHERE from_work_id=$1 AND kind='revises')",
            )
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
            if !has_revision_target {
                // `changes_requested` has graph power only when this node was
                // explicitly modelled as a reviewer. Evidence and research
                // nodes often contain critical findings for a downstream
                // critic; without a revises edge, those findings are their
                // produced result rather than an invalidation instruction.
                // Convert before validation so expected artifacts and gates
                // remain exactly as strict as any other Produced result.
                effective = WorkAttemptState::Produced;
            }
        }
        if effective == WorkAttemptState::Produced {
            let expected_artifact: String =
                sqlx::query_scalar("SELECT expected_artifact FROM work WHERE id=$1")
                    .bind(work_id)
                    .fetch_one(&mut *tx)
                    .await?;
            let artifact_present: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM artifact_refs \
                 WHERE work_id=$1 AND attempt_id=$2 AND state='available')",
            )
            .bind(work_id)
            .bind(attempt_id)
            .fetch_one(&mut *tx)
            .await?;
            let gates_passed: bool = sqlx::query_scalar(
                "SELECT NOT EXISTS (\
                   SELECT 1 FROM work_gates g \
                   LEFT JOIN work_gate_runs r ON r.gate_id=g.id AND r.attempt_id=$2 \
                   WHERE g.work_id=$1 AND COALESCE(r.passed, false)=false\
                 )",
            )
            .bind(work_id)
            .bind(attempt_id)
            .fetch_one(&mut *tx)
            .await?;
            if !expected_artifact.trim().is_empty() && !artifact_present {
                effective = WorkAttemptState::Failed;
                effective_summary = format!(
                    "declared complete without linking expected artifact: {expected_artifact}"
                );
            } else if !gates_passed {
                effective = WorkAttemptState::Failed;
                effective_summary = "one or more deterministic Work gates did not pass".into();
            }
        }
        sqlx::query(
            "UPDATE work_attempts SET state = $2, summary = $3, finished_at = now() WHERE id = $1",
        )
        .bind(attempt_id)
        .bind(effective)
        .bind(&effective_summary)
        .execute(&mut *tx)
        .await?;

        match effective {
            WorkAttemptState::Produced => {
                sqlx::query("UPDATE work SET status = 'completed', resolution = $2 WHERE id = $1")
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
            }
            WorkAttemptState::ChangesRequested => {
                // The reviewer completed its assignment, then each revises
                // edge invalidates the target and its hard descendants.
                sqlx::query("UPDATE work SET status = 'completed', resolution = $2 WHERE id = $1")
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
                let targets = sqlx::query_scalar::<_, Uuid>(
                    "SELECT to_work_id FROM work_edges \
                     WHERE from_work_id = $1 AND kind = 'revises' ORDER BY to_work_id",
                )
                .bind(work_id)
                .fetch_all(&mut *tx)
                .await?;
                for target in targets {
                    invalidate_from(&mut tx, target, &attempt_actor, &effective_summary).await?;
                }
            }
            WorkAttemptState::Blocked => {
                sqlx::query("UPDATE work SET status = 'blocked', resolution = $2 WHERE id = $1")
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
            }
            WorkAttemptState::Failed => {
                // A blind automatic retry repeats the same mechanism before a
                // lead can repair it. Preserve the workspace and stop this node
                // until its owner/lead/Exec explicitly resumes it with a reason.
                sqlx::query("UPDATE work SET status='blocked', resolution=$2 WHERE id=$1")
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
            }
            WorkAttemptState::Abandoned => {
                sqlx::query("UPDATE work SET status = 'abandoned', resolution = $2 WHERE id = $1")
                    .bind(work_id)
                    .bind(&effective_summary)
                    .execute(&mut *tx)
                    .await?;
            }
            WorkAttemptState::Superseded => {}
            WorkAttemptState::Running => unreachable!(),
        }
        tx.commit().await?;
        Ok(effective)
    }

    /// Resume a repaired Work node. The reason is operational evidence of
    /// what changed; retrying the same vague instruction is deliberately not
    /// an automatic scheduler behaviour.
    pub async fn resume_work(&self, work_id: Uuid, by: &str, reason: &str) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "resuming Work needs the concrete repair or changed mechanism".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT owner_id, status, attempt_limit, revision FROM work WHERE id=$1 FOR UPDATE",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        let owner_id: String = row.get("owner_id");
        let status: WorkStatus = row.get("status");
        if status != WorkStatus::Blocked {
            return Err(OrgIntelError::InvalidWork(
                "only blocked Work can be resumed after repair".into(),
            ));
        }
        let allowed = by == "owner"
            || by == "exec"
            || by == owner_id
            || self.team_lead_for(&owner_id).await?.as_deref() == Some(by);
        if !allowed {
            return Err(OrgIntelError::InvalidWork(format!(
                "{by:?} is not the Work owner, its lead, the Exec, or the owner"
            )));
        }
        let exhausted: bool = sqlx::query_scalar(
            "SELECT attempt_limit IS NOT NULL AND (SELECT count(*) FROM work_attempts \
             WHERE work_id=$1 AND revision=work.revision) >= attempt_limit FROM work WHERE id=$1",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        if exhausted {
            return Err(OrgIntelError::InvalidWork(
                "attempt limit reached; revise or replace the Work instead of resuming it".into(),
            ));
        }
        sqlx::query("UPDATE work SET status='active', resolution=$2 WHERE id=$1")
            .bind(work_id)
            .bind(format!("resumed by {by}: {}", reason.trim()))
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_repaired',$1,$2)")
            .bind(by)
            .bind(serde_json::json!({ "work_id": work_id, "reason": reason.trim() }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Retire superseded coordination state without deleting Work, Attempts,
    /// artifacts, or attribution. A live Attempt must be stopped and observed
    /// first so abandoning Work never becomes an implicit process kill.
    pub async fn abandon_work(&self, work_id: Uuid, by: &str, reason: &str) -> Result<()> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "abandoning Work needs an observed reason".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT owner_id, status FROM work WHERE id=$1 FOR UPDATE")
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
        let owner_id: String = row.get("owner_id");
        let status: WorkStatus = row.get("status");
        if matches!(status, WorkStatus::Completed | WorkStatus::Abandoned) {
            return Err(OrgIntelError::InvalidWork(
                "completed or already-abandoned Work keeps its recorded outcome".into(),
            ));
        }
        let allowed = by == "owner"
            || by == "exec"
            || by == owner_id
            || self.team_lead_for(&owner_id).await?.as_deref() == Some(by);
        if !allowed {
            return Err(OrgIntelError::InvalidWork(format!(
                "{by:?} is not the Work owner, its lead, the Exec, or the owner"
            )));
        }
        let running: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_attempts WHERE work_id=$1 AND state='running')",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        if running {
            return Err(OrgIntelError::InvalidWork(
                "Work has a running Attempt; stop and observe that process before abandoning it"
                    .into(),
            ));
        }
        sqlx::query(
            "UPDATE work SET status='abandoned', resolution=$2, updated_at=now() WHERE id=$1",
        )
        .bind(work_id)
        .bind(format!("abandoned by {by}: {}", reason.trim()))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_abandoned',$1,$2)")
            .bind(by)
            .bind(serde_json::json!({ "work_id": work_id, "reason": reason.trim() }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Artifact references point at Runtime truth; digest/commit make the
    /// exact version an Attempt consumed checkable without importing custody.
    pub async fn link_work_artifact(&self, artifact: NewArtifactRef<'_>) -> Result<Uuid> {
        if artifact.kind.trim().is_empty()
            || artifact.uri.trim().is_empty()
            || artifact.label.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "a Work artifact needs kind, locator and label".into(),
            ));
        }
        if artifact.work_id.is_some() != artifact.attempt_id.is_some() {
            return Err(OrgIntelError::InvalidWork(
                "a Work artifact must name both its Work and producer Attempt".into(),
            ));
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        if let (Some(work_id), Some(attempt_id)) = (artifact.work_id, artifact.attempt_id) {
            let matches: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM work_attempts \
                 WHERE id=$1 AND work_id=$2 AND actor_id=$3)",
            )
            .bind(attempt_id)
            .bind(work_id)
            .bind(artifact.created_by)
            .fetch_one(&mut *tx)
            .await?;
            if !matches {
                return Err(OrgIntelError::InvalidWork(
                    "artifact producer, Attempt and Work do not match".into(),
                ));
            }
        }
        sqlx::query(
            "INSERT INTO artifact_refs \
             (id, kind, uri, note, created_by, work_id, attempt_id, digest, source_commit, \
              runtime_generation, label, state) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'available')",
        )
        .bind(id)
        .bind(artifact.kind)
        .bind(artifact.uri)
        .bind(artifact.note)
        .bind(artifact.created_by)
        .bind(artifact.work_id)
        .bind(artifact.attempt_id)
        .bind(artifact.digest)
        .bind(artifact.source_commit)
        .bind(artifact.runtime_generation)
        .bind(artifact.label)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(id)
    }

    pub async fn list_artifact_refs(&self, work_id: Option<Uuid>) -> Result<Vec<ArtifactRefRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, uri, note, created_by, work_id, attempt_id, digest, source_commit, \
                    runtime_generation, label, state, created_at, superseded_at \
             FROM artifact_refs WHERE ($1::uuid IS NULL OR work_id = $1) \
             ORDER BY created_at, id",
        )
        .bind(work_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn add_work_gate(&self, gate: NewWorkGate<'_>) -> Result<Uuid> {
        if gate.command.is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a Work gate needs an argv command".into(),
            ));
        }
        if !valid_company_cwd(gate.cwd) || gate.command.iter().any(|part| part.contains('\0')) {
            return Err(OrgIntelError::InvalidWork(
                "a Work gate needs NUL-free argv and an absolute cwd under /company".into(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO work_gates (id, work_id, name, cwd, command, created_by) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(id)
        .bind(gate.work_id)
        .bind(gate.name)
        .bind(gate.cwd)
        .bind(
            serde_json::to_value(gate.command)
                .map_err(|error| OrgIntelError::Db(sqlx::Error::Protocol(error.to_string())))?,
        )
        .bind(gate.created_by)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn list_work_gates(&self, work_id: Uuid) -> Result<Vec<WorkGateRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, name, cwd, command, created_by, created_at \
             FROM work_gates WHERE work_id = $1 ORDER BY created_at, id",
        )
        .bind(work_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// One repeatable-read projection for CLI and SPA. It is deliberately a
    /// read model over ordinary OrgIntel rows, never a second writer.
    pub async fn work_graph_snapshot(&self) -> Result<WorkGraphSnapshot> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
            .execute(&mut *tx)
            .await?;
        let work = sqlx::query_as(
            "SELECT id, goal_id, owner_id, title, outcome, status, resolution, priority, \
             expected_artifact, repo, base_ref, integration_branch, worktree, revision, \
             attempt_limit, created_at, updated_at FROM work ORDER BY priority DESC, created_at",
        )
        .fetch_all(&mut *tx)
        .await?;
        let edges = sqlx::query_as(
            "SELECT from_work_id, to_work_id, kind, created_at FROM work_edges \
             ORDER BY created_at, from_work_id, to_work_id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let attempts = sqlx::query_as(
            "SELECT id, work_id, revision, attempt_no, actor_id, session_id, state, trigger, \
                    input_fingerprint, feedback_cursor, model, started_at, finished_at, summary \
             FROM work_attempts ORDER BY started_at, id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let attempt_inputs = sqlx::query_as(
            "SELECT attempt_id, artifact_ref_id FROM work_attempt_inputs \
             ORDER BY attempt_id, artifact_ref_id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let attempt_feedback = sqlx::query_as(
            "SELECT attempt_id, message_id FROM work_attempt_feedback \
             ORDER BY attempt_id, message_id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let artifacts = sqlx::query_as(
            "SELECT id, kind, uri, note, created_by, work_id, attempt_id, digest, source_commit, \
                    runtime_generation, label, state, created_at, superseded_at \
             FROM artifact_refs WHERE work_id IS NOT NULL ORDER BY created_at, id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let gates = sqlx::query_as(
            "SELECT id, work_id, name, cwd, command, created_by, created_at \
             FROM work_gates ORDER BY created_at, id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let gate_runs = sqlx::query_as(
            "SELECT id, gate_id, attempt_id, exit_code, output_digest, output_excerpt, \
                    passed, ran_at FROM work_gate_runs ORDER BY ran_at, id",
        )
        .fetch_all(&mut *tx)
        .await?;
        let handoffs = sqlx::query_as(
            "SELECT id, work_id, attempt_id, requested_by, category, requested_action, \
                    prepared_state, resume_condition, state, resolution, assigned_to, \
                    escalated_from, escalated_at, created_at, resolved_at \
             FROM owner_handoffs ORDER BY created_at, id",
        )
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(WorkGraphSnapshot {
            work,
            edges,
            attempts,
            attempt_inputs,
            attempt_feedback,
            artifacts,
            gates,
            gate_runs,
            handoffs,
        })
    }

    pub async fn record_gate_run(&self, run: NewGateRun<'_>) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let valid: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_gates g JOIN work_attempts a \
             ON a.work_id=g.work_id WHERE g.id=$1 AND a.id=$2)",
        )
        .bind(run.gate_id)
        .bind(run.attempt_id)
        .fetch_one(&self.pool)
        .await?;
        if !valid {
            return Err(OrgIntelError::InvalidWork(
                "gate and Attempt must belong to the same Work".into(),
            ));
        }
        sqlx::query(
            "INSERT INTO work_gate_runs \
             (id, gate_id, attempt_id, exit_code, output_digest, output_excerpt, passed) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) \
             ON CONFLICT (gate_id, attempt_id) DO UPDATE SET \
               exit_code=EXCLUDED.exit_code, output_digest=EXCLUDED.output_digest, \
               output_excerpt=EXCLUDED.output_excerpt, passed=EXCLUDED.passed, ran_at=now()",
        )
        .bind(id)
        .bind(run.gate_id)
        .bind(run.attempt_id)
        .bind(run.exit_code)
        .bind(run.output_digest)
        .bind(run.output_excerpt)
        .bind(run.passed)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn gates_passed(&self, work_id: Uuid, attempt_id: Uuid) -> Result<bool> {
        sqlx::query_scalar(
            "SELECT NOT EXISTS (\
               SELECT 1 FROM work_gates g \
               LEFT JOIN work_gate_runs r ON r.gate_id=g.id AND r.attempt_id=$2 \
               WHERE g.work_id=$1 AND COALESCE(r.passed, false)=false\
             )",
        )
        .bind(work_id)
        .bind(attempt_id)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn request_owner_handoff(&self, handoff: NewOwnerHandoff<'_>) -> Result<Uuid> {
        if handoff.requested_action.trim().is_empty()
            || handoff.prepared_state.trim().is_empty()
            || handoff.resume_condition.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "owner handoff needs an exact action, prepared state and observable resume condition"
                    .into(),
            ));
        }
        let id = Uuid::new_v4();

        // Where this judgement goes (S06-T5). Only `owner_judgement` can be
        // answered by anyone other than the owner: identity, CAPTCHA, MFA, legal
        // attestation and payment confirmation are irreducibly human and stay
        // with the owner no matter how large the company grows. A lead absorbing
        // one of those would be a lead exercising authority it does not have.
        //
        // For judgement, the requester's team lead answers first. A lead or an
        // unassigned specialist asks the Exec. Only an Exec judgement reaches
        // the owner; nobody escalates to themselves.
        let assigned_to = if handoff.category == OwnerHandoffCategory::OwnerJudgement {
            if handoff.requested_by == "exec" {
                None
            } else {
                self.team_lead_for(handoff.requested_by)
                    .await?
                    .or_else(|| Some("exec".to_string()))
            }
        } else {
            None
        };

        // The blocked Work says who it is waiting on, not just that it waits.
        // "awaiting owner handoff" on work a lead owes is how a queue becomes
        // invisible to the person who could clear it.
        let blocked_reason = match assigned_to.as_deref() {
            Some(lead) => format!("awaiting {lead} judgement, handoff {id}"),
            None => format!("awaiting owner handoff {id}"),
        };

        let mut tx = self.pool.begin().await?;
        // Serialize with scheduler claim. A handoff without an Attempt is
        // valid for a prepared legacy/manual outcome, but it must not strand
        // an already-running actor behind a second source of truth.
        sqlx::query("SELECT id FROM work WHERE id=$1 FOR UPDATE")
            .bind(handoff.work_id)
            .fetch_one(&mut *tx)
            .await?;
        let running_attempt = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM work_attempts WHERE work_id=$1 AND state='running' LIMIT 1",
        )
        .bind(handoff.work_id)
        .fetch_optional(&mut *tx)
        .await?;
        if handoff.attempt_id.is_none() && running_attempt.is_some() {
            return Err(OrgIntelError::InvalidWork(
                "a Work with a running Attempt must attach that Attempt to its owner handoff"
                    .into(),
            ));
        }
        sqlx::query(
            "INSERT INTO owner_handoffs \
             (id, work_id, attempt_id, requested_by, category, requested_action, \
              prepared_state, resume_condition, assigned_to) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(id)
        .bind(handoff.work_id)
        .bind(handoff.attempt_id)
        .bind(handoff.requested_by)
        .bind(handoff.category)
        .bind(handoff.requested_action)
        .bind(handoff.prepared_state)
        .bind(handoff.resume_condition)
        .bind(assigned_to.as_deref())
        .execute(&mut *tx)
        .await?;
        if let Some(attempt_id) = handoff.attempt_id {
            let closed = sqlx::query(
                "UPDATE work_attempts SET state='blocked', summary=$3, finished_at=now() \
                 WHERE id=$1 AND work_id=$2 AND state='running'",
            )
            .bind(attempt_id)
            .bind(handoff.work_id)
            .bind(&blocked_reason)
            .execute(&mut *tx)
            .await?;
            if closed.rows_affected() != 1 {
                return Err(OrgIntelError::InvalidWork(
                    "handoff Attempt must be the running Attempt of this Work".into(),
                ));
            }
        }
        sqlx::query("UPDATE work SET status='blocked', resolution=$2 WHERE id=$1")
            .bind(handoff.work_id)
            .bind(&blocked_reason)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Refresh the prepared state of an outstanding handoff when the same Work
    /// has advanced after the owner request was first raised. The handoff stays
    /// pending and keeps its identity; only the exact outcome presented for the
    /// eventual decision changes. This is ordinary, attributable OrgIntel
    /// coordination, not an Authority mutation.
    pub async fn refresh_owner_handoff(
        &self,
        id: Uuid,
        changed_by: &str,
        requested_action: &str,
        prepared_state: &str,
        resume_condition: &str,
    ) -> Result<()> {
        if requested_action.trim().is_empty()
            || prepared_state.trim().is_empty()
            || resume_condition.trim().is_empty()
        {
            return Err(OrgIntelError::InvalidWork(
                "refreshing a handoff needs an exact action, prepared state and observable resume condition"
                    .into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT h.work_id, h.requested_action, h.prepared_state, h.resume_condition, \
                    w.owner_id \
             FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
             WHERE h.id=$1 AND h.state='pending' FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("no outstanding handoff with that id".into()))?;
        let work_id: Uuid = row.get("work_id");
        let work_owner: String = row.get("owner_id");
        if !matches!(changed_by, "owner" | "exec") && changed_by != work_owner {
            return Err(OrgIntelError::InvalidWork(
                "only the Work owner, Exec or owner may refresh its prepared handoff".into(),
            ));
        }
        let previous_action: String = row.get("requested_action");
        let previous_prepared: String = row.get("prepared_state");
        let previous_resume: String = row.get("resume_condition");
        if previous_action == requested_action.trim()
            && previous_prepared == prepared_state.trim()
            && previous_resume == resume_condition.trim()
        {
            return Err(OrgIntelError::InvalidWork(
                "the prepared handoff is unchanged".into(),
            ));
        }

        sqlx::query(
            "UPDATE owner_handoffs SET requested_action=$2, prepared_state=$3, \
                    resume_condition=$4 WHERE id=$1",
        )
        .bind(id)
        .bind(requested_action.trim())
        .bind(prepared_state.trim())
        .bind(resume_condition.trim())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('owner_handoff_refreshed',$1,$2)",
        )
        .bind(changed_by)
        .bind(serde_json::json!({
            "handoff_id": id,
            "work_id": work_id,
            "previous_requested_action": previous_action,
            "requested_action": requested_action.trim(),
            "previous_prepared_state": previous_prepared,
            "prepared_state": prepared_state.trim(),
            "previous_resume_condition": previous_resume,
            "resume_condition": resume_condition.trim(),
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn resolve_owner_handoff(
        &self,
        id: Uuid,
        state: OwnerHandoffState,
        resolution: &str,
    ) -> Result<()> {
        self.resolve_handoff_as(id, "owner", state, resolution)
            .await
    }

    /// Resolve judgement at the altitude that currently owes it, write the
    /// answer back as exact Work input, then release the blocked node. The
    /// return path is therefore observable by the next Attempt rather than
    /// ending as an isolated management answer.
    pub async fn resolve_handoff_as(
        &self,
        id: Uuid,
        resolved_by: &str,
        state: OwnerHandoffState,
        resolution: &str,
    ) -> Result<()> {
        if state == OwnerHandoffState::Pending {
            return Err(OrgIntelError::InvalidWork(
                "a handoff cannot be resolved back to pending".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        if resolution.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "resolving a handoff needs the answer or observed outcome".into(),
            ));
        }
        let row = sqlx::query(
            "SELECT h.work_id, h.assigned_to, w.owner_id FROM owner_handoffs h \
             JOIN work w ON w.id=h.work_id WHERE h.id=$1 AND h.state='pending' FOR UPDATE",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let work_id: Uuid = row.get("work_id");
        let assigned_to: Option<String> = row.get("assigned_to");
        let work_owner: String = row.get("owner_id");
        let resolver_owns = assigned_to.as_deref() == Some(resolved_by)
            || (assigned_to.is_none() && resolved_by == "owner");
        if !resolver_owns {
            return Err(OrgIntelError::InvalidWork(format!(
                "handoff is not currently owed by {resolved_by:?}"
            )));
        }
        sqlx::query(
            "UPDATE owner_handoffs SET state=$2, resolution=$3, resolved_at=now() WHERE id=$1",
        )
        .bind(id)
        .bind(state)
        .bind(resolution.trim())
        .execute(&mut *tx)
        .await?;
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor,to_actor,body) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(resolved_by)
        .bind(&work_owner)
        .bind(resolution.trim())
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO work_feedback (work_id,message_id,linked_by) VALUES ($1,$2,$3)")
            .bind(work_id)
            .bind(message_id)
            .bind(resolved_by)
            .execute(&mut *tx)
            .await?;
        let work_status = if state == OwnerHandoffState::Resolved {
            "active"
        } else {
            "blocked"
        };
        sqlx::query("UPDATE work SET status=$2::work_state, resolution=$3 WHERE id=$1")
            .bind(work_id)
            .bind(work_status)
            .bind(resolution)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Resolve an owner-judgement handoff as a review of the exact prepared
    /// outcome. Accepting closes the Work; requesting changes advances the
    /// Work and its hard descendants to a new revision with the owner's exact
    /// feedback as kickoff context.
    pub async fn decide_owner_review(
        &self,
        id: Uuid,
        decision: OwnerReviewDecision,
        feedback: &str,
    ) -> Result<()> {
        let feedback = feedback.trim();
        if decision == OwnerReviewDecision::ChangesRequested && feedback.is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "requesting changes needs owner feedback for the next Attempt".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT h.work_id, h.category::text AS category, w.owner_id \
             FROM owner_handoffs h JOIN work w ON w.id=h.work_id \
             WHERE h.id=$1 AND h.state='pending' FOR UPDATE",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let work_id: Uuid = row.get("work_id");
        let category: String = row.get("category");
        let owner_id: String = row.get("owner_id");
        if category != "owner_judgement" {
            return Err(OrgIntelError::InvalidWork(
                "only an owner_judgement handoff can receive an outcome review".into(),
            ));
        }
        let has_running_attempt: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_attempts WHERE work_id=$1 AND state='running')",
        )
        .bind(work_id)
        .fetch_one(&mut *tx)
        .await?;
        if has_running_attempt {
            return Err(OrgIntelError::InvalidWork(
                "cannot decide an outcome review while its Work Attempt is still running".into(),
            ));
        }

        sqlx::query(
            "INSERT INTO actors (id, kind, display) VALUES ('owner','owner','The Owner') \
             ON CONFLICT (id) DO NOTHING",
        )
        .execute(&mut *tx)
        .await?;
        if !feedback.is_empty() {
            let existing_message: Option<i64> = sqlx::query_scalar(
                "SELECT m.id FROM messages m JOIN work_feedback f ON f.message_id=m.id \
                 WHERE f.work_id=$1 AND m.from_actor='owner' AND m.to_actor=$2 AND m.body=$3 \
                   AND NOT EXISTS (SELECT 1 FROM work_attempt_feedback af WHERE af.message_id=m.id) \
                 ORDER BY m.id DESC LIMIT 1",
            )
            .bind(work_id)
            .bind(&owner_id)
            .bind(feedback)
            .fetch_optional(&mut *tx)
            .await?;
            if existing_message.is_none() {
                let message_id: i64 = sqlx::query_scalar(
                    "INSERT INTO messages (from_actor,to_actor,body) \
                     VALUES ('owner',$1,$2) RETURNING id",
                )
                .bind(&owner_id)
                .bind(feedback)
                .fetch_one(&mut *tx)
                .await?;
                sqlx::query(
                    "INSERT INTO work_feedback (work_id,message_id,linked_by) \
                     VALUES ($1,$2,'owner')",
                )
                .bind(work_id)
                .bind(message_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        let (handoff_state, resolution) = match decision {
            OwnerReviewDecision::Accepted => (
                OwnerHandoffState::Resolved,
                if feedback.is_empty() {
                    "Owner accepted the prepared outcome".to_string()
                } else {
                    format!("Owner accepted the prepared outcome: {feedback}")
                },
            ),
            OwnerReviewDecision::ChangesRequested => (
                OwnerHandoffState::Declined,
                format!("Owner requested changes: {feedback}"),
            ),
        };
        sqlx::query(
            "UPDATE owner_handoffs SET state=$2,resolution=$3,resolved_at=now() WHERE id=$1",
        )
        .bind(id)
        .bind(handoff_state)
        .bind(&resolution)
        .execute(&mut *tx)
        .await?;
        match decision {
            OwnerReviewDecision::Accepted => {
                sqlx::query(
                    "UPDATE work SET status='completed',resolution=$2,updated_at=now() WHERE id=$1",
                )
                .bind(work_id)
                .bind(&resolution)
                .execute(&mut *tx)
                .await?;
            }
            OwnerReviewDecision::ChangesRequested => {
                invalidate_from(&mut tx, work_id, "owner", feedback).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_owner_handoffs(&self) -> Result<Vec<OwnerHandoffRow>> {
        Ok(sqlx::query_as(
            "SELECT id, work_id, attempt_id, requested_by, category, requested_action, \
                    prepared_state, resume_condition, state, resolution, assigned_to, \
                    escalated_from, escalated_at, created_at, resolved_at \
             FROM owner_handoffs ORDER BY created_at, id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn add_schedule(
        &self,
        actor_id: &str,
        work_id: Option<Uuid>,
        reason: &str,
        fire_at: DateTime<Utc>,
    ) -> Result<Uuid> {
        if reason.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a time dependency needs a reason".into(),
            ));
        }
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO schedules (id, actor_id, work_id, reason, fire_at) VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id)
        .bind(actor_id)
        .bind(work_id)
        .bind(reason)
        .bind(fire_at)
        .execute(&mut *tx)
        .await?;
        if let Some(work_id) = work_id {
            sqlx::query("UPDATE work SET status='blocked', resolution=$2 WHERE id=$1")
                .bind(work_id)
                .bind(format!("waiting for schedule {id}: {reason}"))
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    pub async fn claim_due_schedules(&self) -> Result<Vec<ScheduleRow>> {
        let mut tx = self.pool.begin().await?;
        let rows = sqlx::query_as::<_, ScheduleRow>(
            "SELECT id, actor_id, work_id, reason, fire_at, fired_at, cancelled_at, created_at \
             FROM schedules WHERE fire_at <= now() AND fired_at IS NULL AND cancelled_at IS NULL \
             ORDER BY fire_at FOR UPDATE SKIP LOCKED",
        )
        .fetch_all(&mut *tx)
        .await?;
        for row in &rows {
            sqlx::query("UPDATE schedules SET fired_at=now() WHERE id=$1")
                .bind(row.id)
                .execute(&mut *tx)
                .await?;
            if let Some(work_id) = row.work_id {
                sqlx::query(
                    "UPDATE work SET status='active', resolution='time condition reached' \
                     WHERE id=$1 AND resolution LIKE $2 \
                       AND NOT EXISTS (SELECT 1 FROM schedules s WHERE s.work_id=$1 \
                         AND s.id<>$3 AND s.fired_at IS NULL AND s.cancelled_at IS NULL)",
                )
                .bind(work_id)
                .bind(format!("waiting for schedule {}:%", row.id))
                .bind(row.id)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(rows)
    }

    // ---- messages ----

    /// Send a directed message; `to_actor: None` addresses the owner inbox.
    pub async fn send_message(&self, from: &str, to: Option<&str>, body: &str) -> Result<i64> {
        let row = sqlx::query(
            "INSERT INTO messages (from_actor, to_actor, body) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(from)
        .bind(to)
        .bind(body)
        .fetch_one(&self.pool)
        .await?;
        // Directed-mail NOTIFY comes from the database trigger (0002).
        Ok(row.get(0))
    }

    /// Send ordinary free-form conversation and link it to the Work it changes.
    /// The link is kickoff context, not a conversation lifecycle.
    pub async fn send_work_message(
        &self,
        from: &str,
        to: &str,
        work_id: Uuid,
        body: &str,
    ) -> Result<i64> {
        if body.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "Work feedback message cannot be empty".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let owner: String = sqlx::query_scalar("SELECT owner_id FROM work WHERE id=$1")
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
        if owner != to {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work {work_id} belongs to {owner:?}, not message recipient {to:?}"
            )));
        }
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor, to_actor, body) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(from)
        .bind(to)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO work_feedback (work_id, message_id, linked_by) VALUES ($1,$2,$3)")
            .bind(work_id)
            .bind(id)
            .bind(from)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// Whether a directed message is already deterministic input to an active
    /// Work revision. The scheduler uses this after the transaction commits:
    /// such a message must start (or await) the Work Attempt, not race it with
    /// a second free-form actor session.
    pub async fn message_is_work_attempt_input(&self, message_id: i64) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS(\
               SELECT 1 FROM work_feedback feedback \
               JOIN work ON work.id=feedback.work_id \
               WHERE feedback.message_id=$1 \
                 AND work.status IN ('proposed','active') \
                 AND NOT EXISTS (\
                   SELECT 1 FROM owner_handoffs handoff \
                   WHERE handoff.work_id=work.id AND handoff.state='pending'\
                 )\
             )",
        )
        .bind(message_id)
        .fetch_one(&self.pool)
        .await?)
    }

    /// Reply from the accountable Work owner to the human owner, preserving
    /// the same Work-scoped conversation. The owner inbox remains the existing
    /// `to_actor = NULL` convention; no thread entity is introduced.
    pub async fn send_work_message_to_owner(
        &self,
        from: &str,
        work_id: Uuid,
        body: &str,
    ) -> Result<i64> {
        if body.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "Work feedback message cannot be empty".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let owner: String = sqlx::query_scalar("SELECT owner_id FROM work WHERE id=$1")
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
        if owner != from {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work {work_id} belongs to {owner:?}, not replying actor {from:?}"
            )));
        }
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor, to_actor, body) VALUES ($1,NULL,$2) RETURNING id",
        )
        .bind(from)
        .bind(body)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO work_feedback (work_id, message_id, linked_by) VALUES ($1,$2,$3)")
            .bind(work_id)
            .bind(id)
            .bind(from)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    /// The owner/actor messages linked to one Work item. This keeps the review
    /// conversation focused without inventing a thread entity.
    pub async fn owner_work_conversation(
        &self,
        actor: &str,
        work_id: Uuid,
        limit: i64,
    ) -> Result<Vec<MessageRow>> {
        let owner: String = sqlx::query_scalar("SELECT owner_id FROM work WHERE id=$1")
            .bind(work_id)
            .fetch_one(&self.pool)
            .await?;
        if owner != actor {
            return Err(OrgIntelError::InvalidWork(format!(
                "Work {work_id} belongs to {owner:?}, not conversation actor {actor:?}"
            )));
        }
        Ok(sqlx::query_as(
            "SELECT id,from_actor,to_actor,body,created_at,read_at FROM (\
               SELECT m.id,m.from_actor,m.to_actor,m.body,m.created_at,m.read_at \
               FROM messages m JOIN work_feedback f ON f.message_id=m.id \
               WHERE f.work_id=$1 AND (\
                 (m.from_actor='owner' AND m.to_actor=$2) OR \
                 (m.from_actor=$2 AND m.to_actor IS NULL)\
               ) ORDER BY m.created_at DESC,m.id DESC LIMIT $3\
             ) recent ORDER BY created_at,id",
        )
        .bind(work_id)
        .bind(actor)
        .bind(limit.max(1))
        .fetch_all(&self.pool)
        .await?)
    }

    /// An actor's unread inbox (`None` = the owner's), oldest first.
    pub async fn inbox(&self, actor: Option<&str>) -> Result<Vec<MessageRow>> {
        Ok(sqlx::query_as(
            "SELECT id, from_actor, to_actor, body, created_at, read_at FROM messages \
             WHERE read_at IS NULL AND to_actor IS NOT DISTINCT FROM $1 ORDER BY id",
        )
        .bind(actor)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Ordinary conversation between the owner and one actor, oldest first.
    /// This is a read over the existing message rows, not a handover/thread
    /// entity. `to_actor = NULL` is the established owner-inbox convention.
    pub async fn owner_conversation(&self, actor: &str, limit: i64) -> Result<Vec<MessageRow>> {
        let limit = limit.clamp(1, 200);
        Ok(sqlx::query_as(
            "SELECT id, from_actor, to_actor, body, created_at, read_at FROM (\
               SELECT id, from_actor, to_actor, body, created_at, read_at FROM messages \
               WHERE (from_actor = 'owner' AND to_actor = $1) \
                  OR (from_actor = $1 AND to_actor IS NULL) \
               ORDER BY id DESC LIMIT $2\
             ) recent ORDER BY id",
        )
        .bind(actor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn mark_read(&self, message_id: i64) -> Result<()> {
        sqlx::query("UPDATE messages SET read_at = now() WHERE id = $1")
            .bind(message_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- decisions ----

    pub async fn add_decision(&self, title: &str, body: &str, decided_by: &str) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO decisions (id, title, body, decided_by) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(title)
            .bind(body)
            .bind(decided_by)
            .execute(&self.pool)
            .await?;
        Ok(id)
    }

    // ---- events: operational stream, compactable, not a ledger (§4.4) ----

    pub async fn emit_event(
        &self,
        kind: &str,
        actor: Option<&str>,
        body: serde_json::Value,
    ) -> Result<i64> {
        let row = sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(kind)
        .bind(actor)
        .bind(body)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get(0))
    }

    pub async fn list_events(&self, limit: i64) -> Result<Vec<EventRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, actor_id, body, created_at FROM events \
             ORDER BY id DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Every event of one kind, oldest first. Reconciliation reads the whole
    /// effect history: a partial view would understate what the company
    /// actually did, which is the opposite of the point.
    pub async fn events_of_kind(&self, kind: &str) -> Result<Vec<EventRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, actor_id, body, created_at FROM events \
             WHERE kind = $1 ORDER BY id",
        )
        .bind(kind)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Events newer than a watermark, oldest first — the watch stream's
    /// incremental read (T10).
    pub async fn events_after(&self, watermark: i64) -> Result<Vec<EventRow>> {
        Ok(sqlx::query_as(
            "SELECT id, kind, actor_id, body, created_at FROM events \
             WHERE id > $1 ORDER BY id",
        )
        .bind(watermark)
        .fetch_all(&self.pool)
        .await?)
    }

    /// The body of the most recent event of a kind whose body carries a
    /// given string field value — the effect surface's idempotency replay
    /// lookup (T8).
    pub async fn find_event_body(
        &self,
        kind: &str,
        json_field: &str,
        value: &str,
    ) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query(
            "SELECT body FROM events WHERE kind = $1 AND body->>$2 = $3 ORDER BY id DESC LIMIT 1",
        )
        .bind(kind)
        .bind(json_field)
        .bind(value)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.get(0)))
    }

    // ---- scheduler reads (T6) ----

    /// The channel internal wakeups travel on. One channel per database;
    /// the payload carries the company (schema) name.
    pub const NOTIFY_CHANNEL: &'static str = "restless_orgintel";

    /// When the most recent event of a kind happened (e.g. the last wake).
    pub async fn latest_event_at(&self, kind: &str) -> Result<Option<DateTime<Utc>>> {
        let row =
            sqlx::query("SELECT created_at FROM events WHERE kind = $1 ORDER BY id DESC LIMIT 1")
                .bind(kind)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|row| row.get(0)))
    }
}

/// Increment the target and every hard descendant to a new generation. This
/// is change propagation, not artifact custody: Runtime files remain present;
/// the references and results that consumed an older generation become visibly
/// superseded.
async fn invalidate_from(
    tx: &mut Transaction<'_, Postgres>,
    target: Uuid,
    reviewer: &str,
    reason: &str,
) -> Result<()> {
    let target_owner: String = sqlx::query_scalar("SELECT owner_id FROM work WHERE id=$1")
        .bind(target)
        .fetch_one(&mut **tx)
        .await?;
    let feedback_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM messages m JOIN work_feedback f ON f.message_id=m.id \
         WHERE f.work_id=$1 AND m.from_actor=$2 AND m.to_actor=$3 AND m.body=$4 \
         AND NOT EXISTS (SELECT 1 FROM work_attempt_feedback af WHERE af.message_id=m.id))",
    )
    .bind(target)
    .bind(reviewer)
    .bind(&target_owner)
    .bind(reason)
    .fetch_one(&mut **tx)
    .await?;
    if !feedback_exists {
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (from_actor,to_actor,body) VALUES ($1,$2,$3) RETURNING id",
        )
        .bind(reviewer)
        .bind(&target_owner)
        .bind(reason)
        .fetch_one(&mut **tx)
        .await?;
        sqlx::query("INSERT INTO work_feedback (work_id,message_id,linked_by) VALUES ($1,$2,$3)")
            .bind(target)
            .bind(message_id)
            .bind(reviewer)
            .execute(&mut **tx)
            .await?;
    }
    let lead: Option<String> = sqlx::query_scalar(
        "SELECT t.lead_actor_id FROM actors a JOIN teams t ON t.id=a.team_id \
         WHERE a.id=$1 AND t.disbanded_at IS NULL AND t.lead_actor_id<>a.id",
    )
    .bind(&target_owner)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(lead) = lead {
        sqlx::query("INSERT INTO messages (from_actor,to_actor,body) VALUES ($1,$2,$3)")
            .bind(reviewer)
            .bind(&lead)
            .bind(format!(
                "Review rejected Work {target} owned by {target_owner}: {reason}. Change the failed mechanism, then resume the node."
            ))
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query(
        "WITH RECURSIVE affected(id) AS (\
           VALUES ($1::uuid) \
           UNION \
           SELECT edge.to_work_id FROM work_edges edge \
            JOIN affected prior ON edge.from_work_id = prior.id \
            WHERE edge.kind = 'requires'\
         ) UPDATE artifact_refs SET state='superseded', superseded_at=now() \
           WHERE work_id IN (SELECT id FROM affected) AND state='available'",
    )
    .bind(target)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "WITH RECURSIVE affected(id) AS (\
           VALUES ($1::uuid) \
           UNION \
           SELECT edge.to_work_id FROM work_edges edge \
            JOIN affected prior ON edge.from_work_id = prior.id \
            WHERE edge.kind = 'requires'\
         ) UPDATE work SET revision=revision+1, status='proposed', resolution='' \
           WHERE id IN (SELECT id FROM affected)",
    )
    .bind(target)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE work SET status='blocked', resolution=$2 WHERE id=$1")
        .bind(target)
        .bind(format!("changes requested by {reviewer}: {reason}"))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ActorRow {
    pub id: String,
    /// The actor's durable role — `copywriter`, `critic`, `exec`, `owner`.
    /// S04-T5 stopped flattening every worker to the literal `"staff"`, which
    /// is why AC5 can ask for rows whose kind is not `"staff"`.
    pub kind: String,
    pub display: String,
    /// NULL means inherited or not applicable, never "unknown".
    pub model: Option<String>,
    /// The team this actor belongs to, or NULL for unassigned. Unassigned is a
    /// normal state that surfaces show as such — never a default team (S06-T4).
    pub team_id: Option<Uuid>,
    /// Retirement preserves historical attribution while removing the actor
    /// from future staffing. Active-list reads filter this to NULL.
    pub retired_at: Option<DateTime<Utc>>,
    /// The owner or Exec who made retirement explicit.
    pub retired_by: Option<String>,
    /// Why the actor stopped being available; never inferred from inactivity.
    pub retirement_reason: String,
    pub created_at: DateTime<Utc>,
}

/// A group of actors with one accountable lead (S06-T4).
///
/// Coordination state, not kernel truth: recoverable, overridable, repairable.
/// A team grants no effect permission, no budget, no credential scope and no
/// approval right — a lead cannot approve what its members could not, and the
/// owner's approval boundary is unchanged by any team.
#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct TeamRow {
    pub id: Uuid,
    pub name: String,
    /// Why this team exists and what it is accountable for.
    pub brief: String,
    pub lead_actor_id: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub disbanded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct GoalRow {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkRow {
    pub id: Uuid,
    pub goal_id: Option<Uuid>,
    pub owner_id: String,
    pub title: String,
    pub outcome: String,
    pub status: WorkStatus,
    pub resolution: String,
    pub priority: i16,
    pub expected_artifact: String,
    pub repo: Option<String>,
    pub base_ref: Option<String>,
    pub integration_branch: Option<String>,
    pub worktree: Option<String>,
    pub revision: i64,
    pub attempt_limit: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkEdgeRow {
    pub from_work_id: Uuid,
    pub to_work_id: Uuid,
    pub kind: WorkEdgeKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkAttemptRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub revision: i64,
    pub attempt_no: i32,
    pub actor_id: String,
    pub session_id: String,
    pub state: WorkAttemptState,
    pub trigger: String,
    pub input_fingerprint: String,
    pub feedback_cursor: i64,
    pub model: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ArtifactRefRow {
    pub id: Uuid,
    pub kind: String,
    pub uri: String,
    pub note: String,
    pub created_by: String,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub digest: Option<String>,
    pub source_commit: Option<String>,
    pub runtime_generation: Option<String>,
    pub label: String,
    pub state: ArtifactRefState,
    pub created_at: DateTime<Utc>,
    pub superseded_at: Option<DateTime<Utc>>,
}

pub struct NewArtifactRef<'a> {
    pub kind: &'a str,
    pub uri: &'a str,
    pub note: &'a str,
    pub created_by: &'a str,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub digest: Option<&'a str>,
    pub source_commit: Option<&'a str>,
    pub runtime_generation: Option<&'a str>,
    pub label: &'a str,
}

#[derive(Debug, Serialize)]
pub struct ClaimedWork {
    pub work: WorkRow,
    pub attempt_id: Uuid,
    pub attempt_no: i32,
    pub session_id: String,
    pub input_fingerprint: String,
    pub inputs: Vec<ArtifactRefRow>,
    pub feedback: Vec<MessageRow>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkGateRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub name: String,
    pub cwd: String,
    pub command: serde_json::Value,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkGateRunRow {
    pub id: Uuid,
    pub gate_id: Uuid,
    pub attempt_id: Uuid,
    pub exit_code: Option<i32>,
    pub output_digest: String,
    pub output_excerpt: String,
    pub passed: bool,
    pub ran_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkAttemptInputRow {
    pub attempt_id: Uuid,
    pub artifact_ref_id: Uuid,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct WorkAttemptFeedbackRow {
    pub attempt_id: Uuid,
    pub message_id: i64,
}

pub struct NewWorkGate<'a> {
    pub work_id: Uuid,
    pub name: &'a str,
    pub cwd: &'a str,
    pub command: &'a [String],
    pub created_by: &'a str,
}

pub struct NewGateRun<'a> {
    pub gate_id: Uuid,
    pub attempt_id: Uuid,
    pub exit_code: Option<i32>,
    pub output_digest: &'a str,
    pub output_excerpt: &'a str,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct OwnerHandoffRow {
    pub id: Uuid,
    pub work_id: Uuid,
    pub attempt_id: Option<Uuid>,
    pub requested_by: String,
    pub category: OwnerHandoffCategory,
    pub requested_action: String,
    pub prepared_state: String,
    pub resume_condition: String,
    pub state: OwnerHandoffState,
    pub resolution: String,
    /// Who owes this judgement. `None` is the owner — which is what every row
    /// written before S06-T5 means, and why this is nullable rather than
    /// defaulted to an actor.
    pub assigned_to: Option<String>,
    /// The actor that held it before it reached the owner. A fall-through must
    /// be visible: a lead that silently swallows escalations is the S05-T7
    /// single point of failure one level down, with the evidence removed.
    pub escalated_from: Option<String>,
    pub escalated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct WorkGraphSnapshot {
    pub work: Vec<WorkRow>,
    pub edges: Vec<WorkEdgeRow>,
    pub attempts: Vec<WorkAttemptRow>,
    pub attempt_inputs: Vec<WorkAttemptInputRow>,
    pub attempt_feedback: Vec<WorkAttemptFeedbackRow>,
    pub artifacts: Vec<ArtifactRefRow>,
    pub gates: Vec<WorkGateRow>,
    pub gate_runs: Vec<WorkGateRunRow>,
    pub handoffs: Vec<OwnerHandoffRow>,
}

pub struct NewOwnerHandoff<'a> {
    pub work_id: Uuid,
    pub attempt_id: Option<Uuid>,
    pub requested_by: &'a str,
    pub category: OwnerHandoffCategory,
    pub requested_action: &'a str,
    pub prepared_state: &'a str,
    pub resume_condition: &'a str,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct ScheduleRow {
    pub id: Uuid,
    pub actor_id: String,
    pub work_id: Option<Uuid>,
    pub reason: String,
    pub fire_at: DateTime<Utc>,
    pub fired_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct MessageRow {
    pub id: i64,
    pub from_actor: String,
    pub to_actor: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct EventRow {
    pub id: i64,
    pub kind: String,
    pub actor_id: Option<String>,
    pub body: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
