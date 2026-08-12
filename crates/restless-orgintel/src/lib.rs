//! OrgIntel core (ARCHITECTURE.md §4.4): recoverable coordination state for
//! one company — actors, goals, commitments, messages, artifact refs,
//! decisions, events. Explicitly outside the constitutional trust boundary
//! (§4.9): nothing here is a ledger, a custody machine, or kernel truth.
//!
//! One schema per company; the company name is the schema name. One
//! [`OrgIntel`] handle = one company, with `search_path` pinned on every
//! pooled connection so no query can cross companies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

/// The commitment lifecycle — the one place a state machine is correct
/// (deterministic, enumerable; LLM_CURE.md frame 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "commitment_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CommitmentState {
    Proposed,
    Active,
    Blocked,
    Completed,
    Abandoned,
}

#[derive(Debug, thiserror::Error)]
pub enum OrgIntelError {
    #[error("invalid company schema name {0:?}")]
    BadSchemaName(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
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
        let rows = sqlx::query(
            "SELECT tablename FROM pg_tables WHERE schemaname = $1 ORDER BY tablename",
        )
        .bind(&self.schema)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    /// Drop this company's schema and everything in it. Company teardown —
    /// the one destructive operation here, and it is never called implicitly.
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
        sqlx::query(
            "INSERT INTO actors (id, kind, display) VALUES ($1, $2, $3) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(id)
        .bind(kind)
        .bind(display)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ---- goals ----

    pub async fn add_goal(&self, title: &str, body: &str, created_by: &str) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO goals (id, title, body, created_by) VALUES ($1, $2, $3, $4)")
            .bind(id)
            .bind(title)
            .bind(body)
            .bind(created_by)
            .execute(&self.pool)
            .await?;
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

    // ---- commitments ----

    pub async fn add_commitment(
        &self,
        owner_id: &str,
        title: &str,
        body: &str,
        goal_id: Option<Uuid>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO commitments (id, goal_id, owner_id, title, body) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(goal_id)
        .bind(owner_id)
        .bind(title)
        .bind(body)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn set_commitment_state(
        &self,
        id: Uuid,
        state: CommitmentState,
        resolution: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE commitments SET state = $2, resolution = $3, updated_at = now() \
             WHERE id = $1",
        )
        .bind(id)
        .bind(state)
        .bind(resolution)
        .execute(&self.pool)
        .await?;
        // Completion NOTIFYs come from the database trigger (migration
        // 0002): a result lands when its row lands, whoever wrote it (T6).
        Ok(())
    }

    pub async fn list_commitments(&self) -> Result<Vec<CommitmentRow>> {
        Ok(sqlx::query_as(
            "SELECT id, goal_id, owner_id, title, body, state, resolution, created_at, updated_at \
             FROM commitments ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?)
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

    pub async fn mark_read(&self, message_id: i64) -> Result<()> {
        sqlx::query("UPDATE messages SET read_at = now() WHERE id = $1")
            .bind(message_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- artifact refs (references only, never custody — §6.3) ----

    pub async fn add_artifact_ref(
        &self,
        kind: &str,
        uri: &str,
        note: &str,
        created_by: &str,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO artifact_refs (id, kind, uri, note, created_by) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(kind)
        .bind(uri)
        .bind(note)
        .bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(id)
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

    // ---- scheduler reads (T6) ----

    /// The channel internal wakeups travel on. One channel per database;
    /// the payload carries the company (schema) name.
    pub const NOTIFY_CHANNEL: &'static str = "restless_orgintel";

    /// When the most recent event of a kind happened (e.g. the last wake).
    pub async fn latest_event_at(&self, kind: &str) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query(
            "SELECT created_at FROM events WHERE kind = $1 ORDER BY id DESC LIMIT 1",
        )
        .bind(kind)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.get(0)))
    }

    /// The fire time of the most recently recorded wake schedule, if any.
    pub async fn latest_wake_schedule(&self) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query(
            "SELECT (body->>'fire_at')::timestamptz FROM events \
             WHERE kind = 'wake_scheduled' ORDER BY id DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.get(0)))
    }

}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GoalRow {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CommitmentRow {
    pub id: Uuid,
    pub goal_id: Option<Uuid>,
    pub owner_id: String,
    pub title: String,
    pub body: String,
    pub state: CommitmentState,
    pub resolution: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MessageRow {
    pub id: i64,
    pub from_actor: String,
    pub to_actor: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EventRow {
    pub id: i64,
    pub kind: String,
    pub actor_id: Option<String>,
    pub body: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
