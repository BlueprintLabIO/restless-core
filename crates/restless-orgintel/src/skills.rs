//! Company skills (Sprint 55). A skill is an ordinary Runtime directory in the
//! open `SKILL.md` format. OrgIntel stores only the company's decisions about
//! it: the observed digest, its disposition, who may use it, and which exact
//! version a Message or Work selected. It never stores the package body, and
//! selecting or activating a skill grants no credential, effect or budget.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One skill directory as the Runtime observed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct ObservedSkill {
    pub name: String,
    pub description: String,
    /// `builtin`, `company`, `project` or `candidate`.
    pub source: String,
    pub path: String,
    /// `sha256:<hex>` of the skill's `SKILL.md`.
    pub digest: String,
    pub has_scripts: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct SkillRow {
    pub name: String,
    pub description: String,
    pub source: String,
    pub path: String,
    pub digest: String,
    pub has_scripts: bool,
    /// `candidate`, `accepted` or `retired`.
    pub disposition: String,
    pub added_by: Option<String>,
    pub origin_url: Option<String>,
    pub origin_ref: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ts_rs::TS)]
pub struct SkillAssignmentRow {
    pub skill_name: String,
    /// `company`, `team` or `actor`.
    pub scope: String,
    pub scope_id: String,
    pub enabled: bool,
    pub assigned_by: String,
    pub assigned_at: DateTime<Utc>,
}

/// An exact skill version selected for a Message or Work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow, ts_rs::TS)]
pub struct SelectedSkill {
    pub skill_name: String,
    pub digest: String,
}

const SKILL_COLUMNS: &str = "name, description, source, path, digest, has_scripts, disposition, added_by, origin_url, origin_ref, observed_at, created_at, updated_at";

/// The Agent Skills specification name: lowercase letters, digits and
/// hyphens, at most 64 characters.
pub fn valid_skill_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn validate_observed(skill: &ObservedSkill) -> Result<()> {
    if !valid_skill_name(&skill.name) {
        return Err(OrgIntelError::InvalidSkill(format!(
            "skill name {:?} must be lowercase letters, digits and hyphens (at most 64)",
            skill.name
        )));
    }
    if !matches!(
        skill.source.as_str(),
        "builtin" | "company" | "project" | "candidate"
    ) {
        return Err(OrgIntelError::InvalidSkill(format!(
            "unknown skill source {:?}",
            skill.source
        )));
    }
    if !skill.digest.starts_with("sha256:") || skill.path.trim().is_empty() {
        return Err(OrgIntelError::InvalidSkill(format!(
            "skill {} needs a runtime path and a sha256 digest",
            skill.name
        )));
    }
    Ok(())
}

/// Pin explicitly selected skills onto Work inside the commissioning
/// transaction. A selection inherited from a Message keeps that Message's
/// digest; otherwise the current library digest is pinned.
pub(crate) async fn select_work_skills_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    work_id: Uuid,
    skills: &[String],
    selected_by: &str,
    source_message_id: Option<i64>,
) -> Result<Vec<SelectedSkill>> {
    let mut selected = Vec::new();
    for name in skills {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let inherited = match source_message_id {
            Some(message_id) => sqlx::query_scalar::<_, String>(
                "SELECT digest FROM message_skill_selections WHERE message_id=$1 AND skill_name=$2",
            )
            .bind(message_id)
            .bind(name)
            .fetch_optional(&mut **tx)
            .await?,
            None => None,
        };
        let digest = match inherited {
            Some(digest) => digest,
            None => usable_skill_digest(tx, name, selected_by).await?,
        };
        sqlx::query(
            "INSERT INTO work_skill_selections (work_id, skill_name, digest, source_message_id, selected_by) \
             VALUES ($1,$2,$3,$4,$5) ON CONFLICT (work_id, skill_name) DO NOTHING",
        )
        .bind(work_id)
        .bind(name)
        .bind(&digest)
        .bind(source_message_id)
        .bind(selected_by)
        .execute(&mut **tx)
        .await?;
        selected.push(SelectedSkill {
            skill_name: name.to_string(),
            digest,
        });
    }
    Ok(selected)
}

/// A skill may be selected when it is accepted, or when it is a candidate the
/// selecting actor added itself. Retired and unknown skills fail closed.
async fn usable_skill_digest(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    actor: &str,
) -> Result<String> {
    let row = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT digest, disposition, added_by FROM skills WHERE name=$1",
    )
    .bind(name)
    .fetch_optional(&mut **tx)
    .await?;
    match row {
        Some((digest, disposition, _)) if disposition == "accepted" => Ok(digest),
        Some((digest, disposition, added_by))
            if disposition == "candidate" && added_by.as_deref() == Some(actor) =>
        {
            Ok(digest)
        }
        Some((_, disposition, _)) if disposition == "candidate" => {
            Err(OrgIntelError::InvalidSkill(format!(
                "skill {name} is an unaccepted candidate; only the actor that added it may use it"
            )))
        }
        Some(_) => Err(OrgIntelError::InvalidSkill(format!(
            "skill {name} is retired"
        ))),
        None => Err(OrgIntelError::InvalidSkill(format!(
            "no company skill named {name}; run `restless skill list`"
        ))),
    }
}

impl OrgIntel {
    /// Record what the Runtime currently holds. New built-in and company skills
    /// are accepted: they are already company-owned files. New project skills
    /// (for example `npx skills add` into `.agents/skills`) become candidates.
    /// Existing dispositions are never changed by an observation.
    pub async fn observe_skills(&self, observed: &[ObservedSkill]) -> Result<Vec<SkillRow>> {
        let mut tx = self.pool.begin().await?;
        for skill in observed {
            validate_observed(skill)?;
            let disposition = match skill.source.as_str() {
                "builtin" | "company" => "accepted",
                _ => "candidate",
            };
            sqlx::query(
                "INSERT INTO skills (name, description, source, path, digest, has_scripts, disposition) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7) \
                 ON CONFLICT (name) DO UPDATE SET description=EXCLUDED.description, path=EXCLUDED.path, \
                   digest=EXCLUDED.digest, has_scripts=EXCLUDED.has_scripts, observed_at=now(), \
                   updated_at=CASE WHEN skills.digest IS DISTINCT FROM EXCLUDED.digest THEN now() ELSE skills.updated_at END",
            )
            .bind(&skill.name)
            .bind(skill.description.trim())
            .bind(&skill.source)
            .bind(&skill.path)
            .bind(&skill.digest)
            .bind(skill.has_scripts)
            .bind(disposition)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        self.list_skills().await
    }

    /// An actor-imported skill (`restless skill add`). It is usable only by the
    /// importing actor until the owner or its accountable lead accepts it.
    pub async fn register_skill_candidate(
        &self,
        skill: &ObservedSkill,
        added_by: &str,
        origin_url: Option<&str>,
        origin_ref: Option<&str>,
    ) -> Result<SkillRow> {
        validate_observed(skill)?;
        let mut tx = self.pool.begin().await?;
        let existing = sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT disposition, added_by FROM skills WHERE name=$1 FOR UPDATE",
        )
        .bind(&skill.name)
        .fetch_optional(&mut *tx)
        .await?;
        match existing {
            None => {
                sqlx::query(
                    "INSERT INTO skills (name, description, source, path, digest, has_scripts, disposition, added_by, origin_url, origin_ref) \
                     VALUES ($1,$2,'candidate',$3,$4,$5,'candidate',$6,$7,$8)",
                )
                .bind(&skill.name)
                .bind(skill.description.trim())
                .bind(&skill.path)
                .bind(&skill.digest)
                .bind(skill.has_scripts)
                .bind(added_by)
                .bind(origin_url)
                .bind(origin_ref)
                .execute(&mut *tx)
                .await?;
            }
            Some((disposition, owner)) if disposition == "candidate" && owner.as_deref() == Some(added_by) => {
                sqlx::query(
                    "UPDATE skills SET description=$2, path=$3, digest=$4, has_scripts=$5, origin_url=$6, \
                       origin_ref=$7, observed_at=now(), updated_at=now() WHERE name=$1",
                )
                .bind(&skill.name)
                .bind(skill.description.trim())
                .bind(&skill.path)
                .bind(&skill.digest)
                .bind(skill.has_scripts)
                .bind(origin_url)
                .bind(origin_ref)
                .execute(&mut *tx)
                .await?;
            }
            Some(_) => {
                return Err(OrgIntelError::InvalidSkill(format!(
                    "a company skill named {} already exists; choose another name or ask the owner to retire it",
                    skill.name
                )))
            }
        }
        sqlx::query(
            "INSERT INTO events (kind, actor_id, body) VALUES ('skill.candidate.added.v1',$1,$2)",
        )
        .bind(added_by)
        .bind(serde_json::json!({
            "skill": skill.name,
            "digest": skill.digest,
            "has_scripts": skill.has_scripts,
            "origin_url": origin_url,
            "origin_ref": origin_ref,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.skill(&skill.name)
            .await?
            .ok_or_else(|| OrgIntelError::InvalidSkill("candidate was not recorded".into()))
    }

    pub async fn skill(&self, name: &str) -> Result<Option<SkillRow>> {
        Ok(sqlx::query_as::<_, SkillRow>(&format!(
            "SELECT {SKILL_COLUMNS} FROM skills WHERE name=$1"
        ))
        .bind(name)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn list_skills(&self) -> Result<Vec<SkillRow>> {
        Ok(sqlx::query_as::<_, SkillRow>(&format!(
            "SELECT {SKILL_COLUMNS} FROM skills ORDER BY name"
        ))
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn list_skill_assignments(&self) -> Result<Vec<SkillAssignmentRow>> {
        Ok(sqlx::query_as::<_, SkillAssignmentRow>(
            "SELECT skill_name, scope, scope_id, enabled, assigned_by, assigned_at \
             FROM skill_assignments ORDER BY skill_name, scope, scope_id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    /// Accept, return to candidate, or retire a skill. Accepting a skill that
    /// contains scripts is the owner's (or its lead's) explicit trust decision.
    pub async fn set_skill_disposition(
        &self,
        name: &str,
        disposition: &str,
        decided_by: &str,
    ) -> Result<SkillRow> {
        if !matches!(disposition, "candidate" | "accepted" | "retired") {
            return Err(OrgIntelError::InvalidSkill(format!(
                "disposition must be candidate|accepted|retired, not {disposition:?}"
            )));
        }
        let mut tx = self.pool.begin().await?;
        let updated =
            sqlx::query("UPDATE skills SET disposition=$2, updated_at=now() WHERE name=$1")
                .bind(name)
                .bind(disposition)
                .execute(&mut *tx)
                .await?
                .rows_affected();
        if updated == 0 {
            return Err(OrgIntelError::InvalidSkill(format!(
                "no company skill named {name}"
            )));
        }
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('skill.disposition.changed.v1',$1,$2)")
            .bind(decided_by)
            .bind(serde_json::json!({ "skill": name, "disposition": disposition }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.skill(name)
            .await?
            .ok_or_else(|| OrgIntelError::InvalidSkill(format!("no company skill named {name}")))
    }

    /// Grant (`enabled = true`) or remove (`false`) a skill at one scope.
    /// `None` clears that scope's decision so the wider scope applies again.
    pub async fn assign_skill(
        &self,
        name: &str,
        scope: &str,
        scope_id: &str,
        enabled: Option<bool>,
        assigned_by: &str,
    ) -> Result<()> {
        let scope_id = if scope == "company" {
            ""
        } else {
            scope_id.trim()
        };
        if !matches!(scope, "company" | "team" | "actor")
            || (scope != "company" && scope_id.is_empty())
        {
            return Err(OrgIntelError::InvalidSkill(
                "assign a skill to the company, one team id or one actor id".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        match enabled {
            Some(enabled) => {
                sqlx::query(
                    "INSERT INTO skill_assignments (skill_name, scope, scope_id, enabled, assigned_by) \
                     VALUES ($1,$2,$3,$4,$5) ON CONFLICT (skill_name, scope, scope_id) \
                     DO UPDATE SET enabled=EXCLUDED.enabled, assigned_by=EXCLUDED.assigned_by, assigned_at=now()",
                )
                .bind(name)
                .bind(scope)
                .bind(scope_id)
                .bind(enabled)
                .bind(assigned_by)
                .execute(&mut *tx)
                .await
                .map_err(|error| match &error {
                    sqlx::Error::Database(db) if db.is_foreign_key_violation() => {
                        OrgIntelError::InvalidSkill(format!("no company skill named {name}"))
                    }
                    _ => OrgIntelError::Db(error),
                })?;
            }
            None => {
                sqlx::query(
                    "DELETE FROM skill_assignments WHERE skill_name=$1 AND scope=$2 AND scope_id=$3",
                )
                .bind(name)
                .bind(scope)
                .bind(scope_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('skill.assignment.changed.v1',$1,$2)")
            .bind(assigned_by)
            .bind(serde_json::json!({
                "skill": name, "scope": scope, "scope_id": scope_id, "enabled": enabled,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// The skills one actor may use now. Resolution is actor, then team, then
    /// company; with no decision at any scope an accepted skill is available.
    /// A candidate is visible only to the actor that added it.
    pub async fn actor_skills(&self, actor_id: &str) -> Result<Vec<SkillRow>> {
        Ok(sqlx::query_as::<_, SkillRow>(&format!(
            "SELECT {} FROM skills s \
             LEFT JOIN actors a ON a.id=$1 \
             WHERE (s.disposition='accepted' OR (s.disposition='candidate' AND s.added_by=$1)) \
               AND COALESCE( \
                 (SELECT enabled FROM skill_assignments x WHERE x.skill_name=s.name AND x.scope='actor' AND x.scope_id=$1), \
                 (SELECT enabled FROM skill_assignments x WHERE x.skill_name=s.name AND x.scope='team' AND x.scope_id=a.team_id::text), \
                 (SELECT enabled FROM skill_assignments x WHERE x.skill_name=s.name AND x.scope='company'), \
                 TRUE) \
             ORDER BY s.name",
            SKILL_COLUMNS
                .split(", ")
                .map(|column| format!("s.{column}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .bind(actor_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Record an explicit owner or member selection beside its Message. Retries
    /// of the same Message are idempotent and keep the first pinned digest.
    pub async fn record_message_skill_selections(
        &self,
        message_id: i64,
        selected_by: &str,
        skills: &[String],
    ) -> Result<Vec<SelectedSkill>> {
        let mut tx = self.pool.begin().await?;
        let mut selected = Vec::new();
        for name in skills {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let digest = usable_skill_digest(&mut tx, name, selected_by).await?;
            sqlx::query(
                "INSERT INTO message_skill_selections (message_id, skill_name, digest) \
                 VALUES ($1,$2,$3) ON CONFLICT (message_id, skill_name) DO NOTHING",
            )
            .bind(message_id)
            .bind(name)
            .bind(&digest)
            .execute(&mut *tx)
            .await?;
            let pinned = sqlx::query_scalar::<_, String>(
                "SELECT digest FROM message_skill_selections WHERE message_id=$1 AND skill_name=$2",
            )
            .bind(message_id)
            .bind(name)
            .fetch_one(&mut *tx)
            .await?;
            selected.push(SelectedSkill {
                skill_name: name.to_string(),
                digest: pinned,
            });
        }
        tx.commit().await?;
        Ok(selected)
    }

    /// Validate a selection before its Message exists, so a stale or retired
    /// skill fails the send instead of silently dropping the selection.
    pub async fn check_skill_selection(&self, actor: &str, skills: &[String]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for name in skills {
            usable_skill_digest(&mut tx, name.trim(), actor).await?;
        }
        tx.rollback().await?;
        Ok(())
    }

    pub async fn message_skill_selections(
        &self,
        message_ids: &[i64],
    ) -> Result<BTreeMap<i64, Vec<SelectedSkill>>> {
        let rows = sqlx::query_as::<_, (i64, String, String)>(
            "SELECT message_id, skill_name, digest FROM message_skill_selections \
             WHERE message_id = ANY($1) ORDER BY message_id, skill_name",
        )
        .bind(message_ids)
        .fetch_all(&self.pool)
        .await?;
        let mut selections: BTreeMap<i64, Vec<SelectedSkill>> = BTreeMap::new();
        for (message_id, skill_name, digest) in rows {
            selections
                .entry(message_id)
                .or_default()
                .push(SelectedSkill { skill_name, digest });
        }
        Ok(selections)
    }

    /// Add skills to existing Work (for example when a lead repairs a brief).
    /// The next Attempt carries them; a running Attempt is not interrupted.
    pub async fn select_work_skills(
        &self,
        work_id: Uuid,
        skills: &[String],
        selected_by: &str,
        source_message_id: Option<i64>,
    ) -> Result<Vec<SelectedSkill>> {
        let mut tx = self.pool.begin().await?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM work WHERE id=$1)")
            .bind(work_id)
            .fetch_one(&mut *tx)
            .await?;
        if !exists {
            return Err(OrgIntelError::InvalidWork(format!("no Work {work_id}")));
        }
        let selected =
            select_work_skills_in_tx(&mut tx, work_id, skills, selected_by, source_message_id)
                .await?;
        tx.commit().await?;
        Ok(selected)
    }

    pub async fn work_skill_selections(&self, work_id: Uuid) -> Result<Vec<SelectedSkill>> {
        Ok(sqlx::query_as::<_, SelectedSkill>(
            "SELECT skill_name, digest FROM work_skill_selections WHERE work_id=$1 ORDER BY skill_name",
        )
        .bind(work_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// An actor applied a skill (`restless skill use`). This is activity, not
    /// authority: it goes to the compactable operational event stream.
    pub async fn record_skill_activation(
        &self,
        actor_id: &str,
        skill: &SelectedSkill,
        work_id: Option<Uuid>,
        attempt_id: Option<Uuid>,
    ) -> Result<i64> {
        self.emit_event(
            "skill.activated.v1",
            Some(actor_id),
            serde_json::json!({
                "skill": skill.skill_name,
                "digest": skill.digest,
                "work_id": work_id,
                "attempt_id": attempt_id,
            }),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::valid_skill_name;

    #[test]
    fn skill_names_follow_the_open_specification() {
        assert!(valid_skill_name("frontend-design"));
        assert!(valid_skill_name("gauntlet"));
        assert!(!valid_skill_name("Frontend"));
        assert!(!valid_skill_name("-lead"));
        assert!(!valid_skill_name("../etc"));
        assert!(!valid_skill_name(&"a".repeat(65)));
    }
}
