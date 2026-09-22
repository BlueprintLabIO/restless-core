//! OrgIntel core (ARCHITECTURE.md §4.4): recoverable coordination state for
//! one company — actors, goals, Work/Attempts, messages, artifact refs,
//! decisions, events. Explicitly outside the constitutional trust boundary
//! (§4.9): nothing here is a ledger, a custody machine, or kernel truth.
//!
//! One schema per company; the company name is the schema name. One
//! [`OrgIntel`] handle = one company, with `search_path` pinned on every
//! pooled connection so no query can cross companies.

use chrono::{DateTime, Utc};
use sha2::{Digest as _, Sha256};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

mod access;
mod actor_context;
mod actors;
mod artifacts;
mod attempts;
mod colleague_names;
mod constitution;
mod culture;
mod documents;
mod events;
mod goals_work;
mod identity;
mod invocations;
mod messages;
mod notifications;
mod review;
mod rooms;
mod schedules;
mod substrate;
mod types;
mod visual;
mod voice;
mod work_collaboration;

pub use access::*;
pub use actor_context::*;
pub use documents::*;
pub use events::EventReplayPage;
pub use invocations::*;
pub use notifications::*;
pub use types::*;

/// Resolve the canonical direct Room for one durable Message write, creating
/// that Room and its immutable two-Actor audience inside the caller's
/// transaction when necessary. Internal workflows use this boundary instead
/// of relying on database inference from nullable legacy fields.
///
/// `to_actor = None` still means the owner is the directed recipient; it does
/// not mean the Message has no audience. The returned Room id is always bound
/// explicitly by the eventual `INSERT INTO messages` statement.
pub(crate) async fn ensure_direct_message_room_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    from_actor: &str,
    to_actor: Option<&str>,
) -> Result<Uuid> {
    let from_actor = from_actor.trim();
    let peer_actor = to_actor.unwrap_or("owner").trim();
    if from_actor.is_empty() || peer_actor.is_empty() {
        return Err(OrgIntelError::InvalidRoom(
            "a directed Message needs stable sender and recipient Actors".into(),
        ));
    }
    let (first_actor, second_actor) = if from_actor <= peer_actor {
        (from_actor, peer_actor)
    } else {
        (peer_actor, from_actor)
    };
    let canonical_key = rooms::direct_room_canonical_key(first_actor, second_actor);
    let participant_actor_ids = if first_actor == second_actor {
        vec![first_actor]
    } else {
        vec![first_actor, second_actor]
    };
    let existing_actor_ids =
        sqlx::query_scalar::<_, String>("SELECT id FROM actors WHERE id=ANY($1) ORDER BY id")
            .bind(&participant_actor_ids)
            .fetch_all(&mut **tx)
            .await?;
    if existing_actor_ids
        != participant_actor_ids
            .iter()
            .map(|actor_id| (*actor_id).to_string())
            .collect::<Vec<_>>()
    {
        return Err(OrgIntelError::InvalidRoom(
            "every directed Message participant must be a durable Actor".into(),
        ));
    }

    // The candidate UUID is the canonical pre-release Room identity used by
    // the original data migration. The unique canonical key arbitrates races;
    // every contender then reads the same committed Room.
    sqlx::query(
        "INSERT INTO rooms (id,kind,title,created_by,canonical_key) \
         VALUES (md5('restless:room:' || $1)::uuid,'direct',$2,$3,$1) \
         ON CONFLICT (canonical_key) DO NOTHING",
    )
    .bind(&canonical_key)
    .bind(if first_actor == second_actor {
        format!("Notes for {first_actor}")
    } else {
        format!("{first_actor} / {second_actor}")
    })
    .bind(first_actor)
    .execute(&mut **tx)
    .await?;
    let (room_id, room_creator): (Uuid, String) = sqlx::query_as(
        "SELECT id,created_by FROM rooms \
         WHERE canonical_key=$1 AND kind='direct' AND archived_at IS NULL",
    )
    .bind(&canonical_key)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        OrgIntelError::InvalidRoom("the canonical direct Room is missing or archived".into())
    })?;

    // A trusted internal directed write reactivates the exact canonical
    // audience. Sort order above keeps participant FK acquisition stable.
    for actor_id in participant_actor_ids {
        sqlx::query(
            "INSERT INTO room_participants (room_id,actor_id,role) \
             VALUES ($1,$2,CASE WHEN $2=$3 THEN 'owner'::room_participant_role \
                                ELSE 'member'::room_participant_role END) \
             ON CONFLICT (room_id,actor_id) DO UPDATE \
             SET role=EXCLUDED.role,joined_at=now(),left_at=NULL \
             WHERE room_participants.left_at IS NOT NULL \
                OR room_participants.role<>EXCLUDED.role",
        )
        .bind(room_id)
        .bind(actor_id)
        .bind(&room_creator)
        .execute(&mut **tx)
        .await?;
    }
    Ok(room_id)
}

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
        // The Runtime uses the same bound for repository and worktree path
        // segments. Reject an unusable coordinate before durable Work exists.
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_staff_actor_id(value: &str) -> bool {
    if value.len() > 32 {
        return false;
    }
    let mut segments = value.split('-');
    let valid_segment = |segment: &str| {
        if segment.is_empty()
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        {
            return false;
        }
        let version_suffix = segment.starts_with('v')
            && segment.len() > 1
            && segment[1..].bytes().all(|byte| byte.is_ascii_digit());
        !matches!(
            segment,
            "staff"
                | "lead"
                | "live"
                | "test"
                | "dev"
                | "prod"
                | "stage"
                | "retry"
                | "attempt"
                | "revision"
                | "impl"
                | "implementation"
        ) && !version_suffix
    };
    matches!(
        (segments.next(), segments.next(), segments.next()),
        (Some(domain), Some(craft), None) if valid_segment(domain) && valid_segment(craft)
    )
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
    value == "@attempt"
        || value == "/company"
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

    /// Close every clone of this company's pool before the account plane
    /// removes a throwaway cell database. `PgPool::close` is shared across
    /// clones, so a registry handle cannot keep the database alive invisibly.
    pub async fn close(&self) {
        self.pool.close().await;
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
                    // Keep pg_temp last so SECURITY DEFINER functions created
                    // with `SET search_path FROM CURRENT` cannot resolve an
                    // attacker-controlled temporary relation first.
                    sqlx::query(&format!("SET search_path TO {schema}, pg_catalog, pg_temp"))
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
                    sqlx::query(&format!("SET search_path TO {schema}, pg_catalog, pg_temp"))
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(database_url)
            .await?;
        let org = Self { pool, schema };
        org.align_team_member_names().await?;
        Ok(org)
    }

    pub fn schema(&self) -> &str {
        &self.schema
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
    let accountability = actors::lock_work_accountability_in_tx(tx, target).await?;
    let target_owner = accountability.owner_id;
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
        let room_id = ensure_direct_message_room_in_tx(tx, reviewer, Some(&target_owner)).await?;
        let message_id: i64 = sqlx::query_scalar(
            "INSERT INTO messages (room_id,from_actor,to_actor,body) \
             VALUES ($1,$2,$3,$4) RETURNING id",
        )
        .bind(room_id)
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
    if let Some(lead) = accountability
        .lead_actor_id
        .filter(|lead| lead != &target_owner)
    {
        let room_id = ensure_direct_message_room_in_tx(tx, reviewer, Some(&lead)).await?;
        sqlx::query(
            "INSERT INTO messages (room_id,from_actor,to_actor,body) VALUES ($1,$2,$3,$4)",
        )
            .bind(room_id)
            .bind(reviewer)
            .bind(&lead)
            .bind(format!(
                "Review rejected Work {target} owned by {target_owner}: {reason}. Change the failed mechanism, then resume the node."
            ))
            .execute(&mut **tx)
            .await?;
    }
    // A rejected Work generation also retires its exact native-document
    // review obligation. Leaving a pending dependency attached to the prior
    // revision would make the next producer Attempt unable to request its own
    // review: the document path correctly refuses to resume stale coordinates.
    sqlx::query(
        "WITH RECURSIVE affected(id) AS (\
           VALUES ($1::uuid) \
           UNION \
           SELECT edge.to_work_id FROM work_edges edge \
            JOIN affected prior ON edge.from_work_id = prior.id \
            WHERE edge.kind = 'requires'\
         ) UPDATE native_document_reviews review \
           SET status='stale',stale_detected_by_actor_id=$2,stale_at=now(),version=version+1 \
           FROM native_document_work_review_dependencies dependency \
           WHERE dependency.work_id IN (SELECT id FROM affected) \
             AND dependency.status='pending' \
             AND review.id=dependency.review_id \
             AND review.document_id=dependency.document_id \
             AND review.status='requested'",
    )
    .bind(target)
    .bind(reviewer)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "WITH RECURSIVE affected(id) AS (\
           VALUES ($1::uuid) \
           UNION \
           SELECT edge.to_work_id FROM work_edges edge \
            JOIN affected prior ON edge.from_work_id = prior.id \
            WHERE edge.kind = 'requires'\
         ) UPDATE native_document_work_review_dependencies \
           SET status='stale',resolved_by_actor_id=$2,resolved_at=now(),resumed_at=now() \
           WHERE work_id IN (SELECT id FROM affected) AND status='pending'",
    )
    .bind(target)
    .bind(reviewer)
    .execute(&mut **tx)
    .await?;
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

#[cfg(test)]
mod actor_identity_tests {
    use super::valid_staff_actor_id;

    #[test]
    fn staff_identity_is_domain_plus_craft_not_assignment_history() {
        for valid in [
            "centre-critic",
            "copy-critic",
            "release-build",
            "prospect-research",
        ] {
            assert!(valid_staff_actor_id(valid), "{valid} should be durable");
        }
        for invalid in [
            "staff-centre-critic",
            "site-validation-lead",
            "centre-critic-live",
            "copy-critic-v2",
            "release-build-retry",
            "sales-lead",
            "release-impl",
            "critic",
            "Centre-critic",
        ] {
            assert!(
                !valid_staff_actor_id(invalid),
                "{invalid} encodes class, assignment or lifecycle state"
            );
        }
    }
}
