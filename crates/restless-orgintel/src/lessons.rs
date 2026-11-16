//! Company lessons: a standing rule, in plain words, for the whole company, one team or one
//! agent. Every agent it applies to reads it on each wake (`lessons_block`).
//!
//! A lesson comes from one of two places (Sprint 119):
//!
//! - **The owner ruled it.** A standing instruction ("from now on", "always", "never") is
//!   recorded at once, by Exec or a lead, citing the owner's message. It is never dropped
//!   automatically.
//! - **The company learned it.** Work sent back or failed the same way a second time
//!   (`observations`). The daemon records it, so `recorded_by` is `daemon`, and drops it again
//!   when the problem has not come back for six weeks.
//!
//! When the same problem shows up again the lesson counts it rather than being recorded twice,
//! so a rule that does not hold is visible.
//!
//! Recoverable coordination state: a lesson grants no authority and gates nothing.

use super::*;

/// A rule is one or two plain sentences.
pub const MAX_LESSON_RULE_CHARS: usize = 400;
const MIN_LESSON_RULE_CHARS: usize = 12;
const MIN_LESSON_RULE_WORDS: usize = 3;
const MAX_LESSON_REASON_CHARS: usize = 400;
/// What one wake carries, at most: this many lessons and this many bytes of them. A lesson is
/// never cut in half; one that does not fit is left out and counted. The rest is one
/// `restless lessons list` away.
pub const LESSONS_SHOWN: usize = 20;
pub const LESSONS_BLOCK_BYTES: usize = 2_000;
/// Places kept for what the company learned when the owner's rulings alone would fill the
/// block, so a company with many rulings still reads what it learned most recently.
pub const LEARNED_LESSONS_KEPT: usize = 4;

/// Who a lesson is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LessonScope {
    Company,
    Team(Uuid),
    Actor(String),
}

impl LessonScope {
    /// `company`, `team:<team id>` or `actor:<actor id>`.
    pub fn parse(value: &str) -> std::result::Result<Self, String> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("company") {
            return Ok(Self::Company);
        }
        match value.split_once(':') {
            Some(("team", id)) => Uuid::parse_str(id.trim())
                .map(Self::Team)
                .map_err(|_| format!("team scope needs a team id, as in team:<id>; got {id:?}")),
            Some(("actor", id)) if !id.trim().is_empty() => Ok(Self::Actor(id.trim().to_string())),
            _ => Err(format!(
                "scope is company, team:<team id> or actor:<actor id>; got {value:?}"
            )),
        }
    }
}

pub struct NewLesson<'a> {
    pub rule: &'a str,
    pub scope: LessonScope,
    /// The person's message the rule came from.
    pub source_message_id: Option<i64>,
    pub recorded_by: &'a str,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct LessonRow {
    pub id: Uuid,
    pub rule: String,
    /// `company`, `team` or `actor`.
    pub scope_kind: String,
    pub team_id: Option<Uuid>,
    /// The team's name, for a team lesson.
    pub team_name: Option<String>,
    pub actor_id: Option<String>,
    /// The agent's display name, for a lesson about one agent.
    pub actor_display: Option<String>,
    pub source_message_id: Option<i64>,
    pub recorded_by: String,
    pub created_at: DateTime<Utc>,
    /// `active` or `retired`.
    pub status: String,
    pub retired_at: Option<DateTime<Utc>>,
    pub retired_by: Option<String>,
    pub retirement_reason: String,
    /// How many times the same problem came back after the lesson was recorded.
    pub recurrence_count: i32,
    pub last_recurred_at: Option<DateTime<Utc>>,
    pub last_recurrence_message_id: Option<i64>,
    /// The company learned it from work that went wrong the same way twice; the owner did not
    /// rule it. Only these are ever dropped automatically.
    pub learned: bool,
}

impl LessonRow {
    /// When the lesson last earned its place: when it was recorded, or when its problem last
    /// came back.
    pub fn last_useful_at(&self) -> DateTime<Utc> {
        self.last_recurred_at
            .map_or(self.created_at, |at| at.max(self.created_at))
    }

    /// Who the lesson is for, as an agent or the owner reads it.
    pub fn scope_label(&self) -> String {
        match self.scope_kind.as_str() {
            "team" => match (&self.team_name, self.team_id) {
                (Some(name), _) => format!("team {name}"),
                (None, Some(id)) => format!("team {id}"),
                _ => "one team".into(),
            },
            "actor" => self.actor_id.clone().unwrap_or_else(|| "one agent".into()),
            _ => "whole company".into(),
        }
    }

    /// Enough of the id to name the lesson in a command.
    pub fn short_id(&self) -> String {
        self.id.simple().to_string()[..8].to_string()
    }
}

const LESSON_COLUMNS: &str = "l.id, l.rule, l.scope_kind, l.team_id, t.name AS team_name, \
     l.actor_id, a.display AS actor_display, l.source_message_id, l.recorded_by, l.created_at, \
     CASE WHEN l.retired_at IS NULL THEN 'active' ELSE 'retired' END AS status, \
     l.retired_at, l.retired_by, l.retirement_reason, l.recurrence_count, l.last_recurred_at, \
     l.last_recurrence_message_id, (l.recorded_by = 'daemon') AS learned \
     FROM company_lessons l \
     LEFT JOIN teams t ON t.id = l.team_id \
     LEFT JOIN actors a ON a.id = l.actor_id";

pub(crate) fn refused(reason: impl Into<String>) -> OrgIntelError {
    OrgIntelError::InvalidWork(reason.into())
}

/// The rule as stored: one line, single spaces. Refuses a rule that says nothing or runs long.
pub fn normalized_lesson_rule(rule: &str) -> Result<String> {
    let rule = rule.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars = rule.chars().count();
    if chars < MIN_LESSON_RULE_CHARS || rule.split(' ').count() < MIN_LESSON_RULE_WORDS {
        return Err(refused(
            "a lesson needs a rule someone can follow: one or two plain sentences",
        ));
    }
    if chars > MAX_LESSON_RULE_CHARS {
        return Err(refused(format!(
            "a lesson is one or two plain sentences, at most {MAX_LESSON_RULE_CHARS} characters; this one has {chars}"
        )));
    }
    Ok(rule)
}

fn lesson_line(lesson: &LessonRow) -> String {
    let came_back = match lesson.recurrence_count {
        0 => String::new(),
        1 => ", came back once".to_string(),
        count => format!(", came back {count} times"),
    };
    format!(
        "- {} ({}, lesson {}{came_back})\n",
        lesson.rule,
        lesson.scope_label(),
        lesson.short_id()
    )
}

/// Which lessons one wake carries, as indexes into `lessons`: the owner's rulings and what the
/// company learned, each in the order it is shown. The cap is [`LESSONS_SHOWN`] lessons and
/// [`LESSONS_BLOCK_BYTES`] bytes of their lines; a lesson is never cut.
///
/// The order they are kept in, so what is dropped first is what was useful longest ago:
/// 1. the [`LEARNED_LESSONS_KEPT`] learned lessons used most recently;
/// 2. the owner's rulings, newest first;
/// 3. the other learned lessons, most recently useful first.
///
/// A learned lesson is useful when it is recorded and each time its problem comes back
/// ([`LessonRow::last_useful_at`]). Leaving a lesson out changes nothing in the record.
/// `lessons` arrive newest first.
pub fn lessons_for_wake(lessons: &[LessonRow]) -> (Vec<usize>, Vec<usize>) {
    let ruled: Vec<usize> = (0..lessons.len())
        .filter(|&index| !lessons[index].learned)
        .collect();
    let mut learned: Vec<usize> = (0..lessons.len())
        .filter(|&index| lessons[index].learned)
        .collect();
    learned.sort_by_key(|&index| std::cmp::Reverse(lessons[index].last_useful_at()));
    let kept = learned.len().min(LEARNED_LESSONS_KEPT);
    let mut used = 0usize;
    let mut shown = 0usize;
    let mut admit = |index: usize| {
        let bytes = lesson_line(&lessons[index]).len();
        let fits = shown < LESSONS_SHOWN && used + bytes <= LESSONS_BLOCK_BYTES;
        if fits {
            used += bytes;
            shown += 1;
        }
        fits
    };
    let first: Vec<usize> = learned[..kept]
        .iter()
        .copied()
        .filter(|&index| admit(index))
        .collect();
    let ruled: Vec<usize> = ruled.into_iter().filter(|&index| admit(index)).collect();
    let rest: Vec<usize> = learned[kept..]
        .iter()
        .copied()
        .filter(|&index| admit(index))
        .collect();
    (ruled, first.into_iter().chain(rest).collect())
}

/// The lessons block for one wake: what the owner has ruled, newest first, then what the
/// company has learned, most recently useful first, within the cap of [`lessons_for_wake`],
/// then how many were left out and where they are. Empty when there are no lessons, so a
/// company without any pays nothing. `lessons` arrive newest first.
pub fn lessons_block(lessons: &[LessonRow]) -> String {
    if lessons.is_empty() {
        return String::new();
    }
    let (ruled, learned) = lessons_for_wake(lessons);
    let mut block = String::new();
    if !ruled.is_empty() {
        block
            .push_str("# What the owner has ruled [owner policy; follow each one; newest first]\n");
        for &index in &ruled {
            block.push_str(&lesson_line(&lessons[index]));
        }
    }
    if !learned.is_empty() {
        block.push_str(
            "# What the company has learned [work went wrong this way more than once; follow \
             each one unless the owner has ruled otherwise]\n",
        );
        for &index in &learned {
            block.push_str(&lesson_line(&lessons[index]));
        }
    }
    let rest = lessons.len() - ruled.len() - learned.len();
    if rest > 0 {
        block.push_str(&format!(
            "- … {rest} more not shown: `restless lessons list`\n"
        ));
    }
    block
}

/// What `by` may do with lessons: everything (owner, Exec), or only its own team's.
enum LessonAuthority {
    Company,
    Team(Uuid),
}

async fn lesson_authority(
    tx: &mut Transaction<'_, Postgres>,
    by: &str,
) -> Result<Option<LessonAuthority>> {
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
    )
    .bind(by)
    .fetch_one(&mut **tx)
    .await?;
    if !active {
        return Ok(None);
    }
    if matches!(by, "owner" | "exec") {
        return Ok(Some(LessonAuthority::Company));
    }
    let led: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM teams WHERE lead_actor_id=$1 AND disbanded_at IS NULL \
         ORDER BY created_at LIMIT 1",
    )
    .bind(by)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(led.map(LessonAuthority::Team))
}

/// Whether a lesson for `team_id` / `actor_id` sits inside the team `led` leads.
async fn inside_team(
    tx: &mut Transaction<'_, Postgres>,
    led: Uuid,
    team_id: Option<Uuid>,
    actor_id: Option<&str>,
) -> Result<bool> {
    if let Some(team_id) = team_id {
        return Ok(team_id == led);
    }
    let Some(actor_id) = actor_id else {
        return Ok(false);
    };
    Ok(
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND team_id=$2)")
            .bind(actor_id)
            .bind(led)
            .fetch_one(&mut **tx)
            .await?,
    )
}

/// A lesson comes from something a person said: the message must exist and be a person's.
async fn check_person_message(tx: &mut Transaction<'_, Postgres>, message_id: i64) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM messages m JOIN actors a ON a.id = m.from_actor \
         WHERE m.id=$1 AND a.actor_class='human')",
    )
    .bind(message_id)
    .fetch_one(&mut **tx)
    .await?)
}

/// Whether `by` may record a rule or an observation for `scope`, citing `source_message_id`:
/// the owner or Exec for anyone, a team lead for its own team or its members, and the message
/// must be a person's. The caller ends the transaction on a refusal.
pub(crate) async fn admit_lesson(
    tx: &mut Transaction<'_, Postgres>,
    by: &str,
    scope: &LessonScope,
    source_message_id: Option<i64>,
) -> Result<()> {
    let Some(authority) = lesson_authority(tx, by).await? else {
        return Err(refused(
            "only the owner, Exec or a team lead may record a lesson",
        ));
    };
    let (team_id, actor_id) = match scope {
        LessonScope::Company => (None, None),
        LessonScope::Team(id) => (Some(*id), None),
        LessonScope::Actor(id) => (None, Some(id.as_str())),
    };
    if let Some(team_id) = team_id {
        let live: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM teams WHERE id=$1 AND disbanded_at IS NULL)",
        )
        .bind(team_id)
        .fetch_one(&mut **tx)
        .await?;
        if !live {
            return Err(refused("no live team with that id"));
        }
    }
    if let Some(actor_id) = actor_id {
        let agent: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
             AND actor_class='agent')",
        )
        .bind(actor_id)
        .fetch_one(&mut **tx)
        .await?;
        if !agent {
            return Err(refused(format!("no active agent with id {actor_id:?}")));
        }
    }
    if let LessonAuthority::Team(led) = authority {
        if !inside_team(tx, led, team_id, actor_id).await? {
            return Err(refused(
                "a team lead records lessons for its own team or its members; ask Exec for a wider one",
            ));
        }
    }
    if let Some(message_id) = source_message_id {
        if !check_person_message(tx, message_id).await? {
            return Err(refused(format!(
                "message {message_id} is not a message from a person; cite the message the rule came from"
            )));
        }
    }
    Ok(())
}

impl OrgIntel {
    /// Record a standing rule. The owner or Exec may record one for the company, a team or an
    /// agent; a team lead only for its own team or its members.
    pub async fn record_lesson(&self, input: NewLesson<'_>) -> Result<LessonRow> {
        let rule = normalized_lesson_rule(input.rule)?;
        let mut tx = self.pool.begin().await?;
        let (scope_kind, team_id, actor_id) = match &input.scope {
            LessonScope::Company => ("company", None, None),
            LessonScope::Team(id) => ("team", Some(*id), None),
            LessonScope::Actor(id) => ("actor", None, Some(id.as_str())),
        };
        if let Err(error) = admit_lesson(
            &mut tx,
            input.recorded_by,
            &input.scope,
            input.source_message_id,
        )
        .await
        {
            tx.rollback().await?;
            return Err(error);
        }
        let same: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM company_lessons WHERE retired_at IS NULL AND lower(rule)=lower($1) \
             AND scope_kind=$2 AND team_id IS NOT DISTINCT FROM $3 \
             AND actor_id IS NOT DISTINCT FROM $4 LIMIT 1",
        )
        .bind(&rule)
        .bind(scope_kind)
        .bind(team_id)
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(same) = same {
            tx.rollback().await?;
            return Err(refused(format!(
                "lesson {same} already says this; if the problem came back, mark it with `restless lessons recurred --lesson {same}`"
            )));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO company_lessons \
             (id, rule, scope_kind, team_id, actor_id, source_message_id, recorded_by) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(&rule)
        .bind(scope_kind)
        .bind(team_id)
        .bind(actor_id)
        .bind(input.source_message_id)
        .bind(input.recorded_by)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('lesson_recorded',$1,$2)")
            .bind(input.recorded_by)
            .bind(serde_json::json!({
                "lesson_id": id,
                "rule": rule,
                "scope_kind": scope_kind,
                "team_id": team_id,
                "actor_id": actor_id,
                "source_message_id": input.source_message_id,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.lesson(id).await
    }

    /// One lesson, active or retired.
    pub async fn lesson(&self, id: Uuid) -> Result<LessonRow> {
        sqlx::query_as(&format!("SELECT {LESSON_COLUMNS} WHERE l.id=$1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| refused("no lesson with that id"))
    }

    /// The lesson a reference names: its id, or the first characters of it as a wake shows them.
    pub async fn resolve_lesson(&self, reference: &str) -> Result<Uuid> {
        let reference = reference.trim().to_ascii_lowercase();
        if let Ok(id) = Uuid::parse_str(&reference) {
            return Ok(id);
        }
        let prefix: String = reference.chars().filter(|c| *c != '-').collect();
        if prefix.len() < 6 || !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(refused(format!(
                "{reference:?} does not name a lesson; `restless lessons list` shows their ids"
            )));
        }
        let matches: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM company_lessons WHERE replace(id::text,'-','') LIKE $1 || '%' LIMIT 2",
        )
        .bind(&prefix)
        .fetch_all(&self.pool)
        .await?;
        match matches.as_slice() {
            [id] => Ok(*id),
            [] => Err(refused(format!(
                "no lesson starts with {reference:?}; `restless lessons list` shows their ids"
            ))),
            _ => Err(refused(format!(
                "more than one lesson starts with {reference:?}; use the whole id"
            ))),
        }
    }

    /// Every lesson, newest first; retired ones only when asked for.
    pub async fn list_lessons(&self, include_retired: bool) -> Result<Vec<LessonRow>> {
        Ok(sqlx::query_as(&format!(
            "SELECT {LESSON_COLUMNS} WHERE ($1 OR l.retired_at IS NULL) \
             ORDER BY l.created_at DESC, l.id"
        ))
        .bind(include_retired)
        .fetch_all(&self.pool)
        .await?)
    }

    /// The active lessons one agent works under, newest first: the company's, its team's (the
    /// team it belongs to or leads) and its own.
    pub async fn lessons_for_actor(&self, actor_id: &str) -> Result<Vec<LessonRow>> {
        Ok(sqlx::query_as(&format!(
            "SELECT {LESSON_COLUMNS} WHERE l.retired_at IS NULL AND ( \
                 l.scope_kind='company' \
                 OR l.actor_id=$1 \
                 OR l.team_id IN ( \
                     SELECT team_id FROM actors WHERE id=$1 AND team_id IS NOT NULL \
                     UNION \
                     SELECT id FROM teams WHERE lead_actor_id=$1 AND disbanded_at IS NULL)) \
             ORDER BY l.created_at DESC, l.id"
        ))
        .bind(actor_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// The same problem came back: count it on the lesson instead of recording a second one.
    /// Citing the same message twice counts once.
    pub async fn mark_lesson_recurred(
        &self,
        id: Uuid,
        marked_by: &str,
        source_message_id: Option<i64>,
    ) -> Result<LessonRow> {
        let mut tx = self.pool.begin().await?;
        let lesson = self.lock_lesson_for(&mut tx, id, marked_by).await;
        let (retired, last_message) = match lesson {
            Ok(found) => found,
            Err(error) => {
                tx.rollback().await?;
                return Err(error);
            }
        };
        if retired {
            tx.rollback().await?;
            return Err(refused(
                "that lesson is retired; restore it or record a new one",
            ));
        }
        if let Some(message_id) = source_message_id {
            if !check_person_message(&mut tx, message_id).await? {
                tx.rollback().await?;
                return Err(refused(format!(
                    "message {message_id} is not a message from a person; cite the message that showed the problem came back"
                )));
            }
            if last_message == Some(message_id) {
                tx.rollback().await?;
                return self.lesson(id).await;
            }
        }
        let count: i32 = sqlx::query_scalar(
            "UPDATE company_lessons SET recurrence_count = recurrence_count + 1, \
             last_recurred_at = now(), last_recurrence_message_id = $2 WHERE id=$1 \
             RETURNING recurrence_count",
        )
        .bind(id)
        .bind(source_message_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('lesson_recurred',$1,$2)")
            .bind(marked_by)
            .bind(serde_json::json!({
                "lesson_id": id,
                "recurrence_count": count,
                "source_message_id": source_message_id,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.lesson(id).await
    }

    /// Stop applying a lesson. It keeps its record and can be restored.
    pub async fn retire_lesson(
        &self,
        id: Uuid,
        retired_by: &str,
        reason: &str,
    ) -> Result<LessonRow> {
        let reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
        if reason.is_empty() {
            return Err(refused("retiring a lesson needs a reason"));
        }
        let reason: String = reason.chars().take(MAX_LESSON_REASON_CHARS).collect();
        let mut tx = self.pool.begin().await?;
        let lesson = self.lock_lesson_for(&mut tx, id, retired_by).await;
        let retired = match lesson {
            Ok((retired, _)) => retired,
            Err(error) => {
                tx.rollback().await?;
                return Err(error);
            }
        };
        if retired {
            tx.rollback().await?;
            return Err(refused("that lesson is already retired"));
        }
        sqlx::query(
            "UPDATE company_lessons SET retired_at=now(), retired_by=$2, retirement_reason=$3 \
             WHERE id=$1",
        )
        .bind(id)
        .bind(retired_by)
        .bind(&reason)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('lesson_retired',$1,$2)")
            .bind(retired_by)
            .bind(serde_json::json!({ "lesson_id": id, "reason": reason }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.lesson(id).await
    }

    /// Bring a retired lesson back, with its history.
    pub async fn restore_lesson(&self, id: Uuid, restored_by: &str) -> Result<LessonRow> {
        let mut tx = self.pool.begin().await?;
        let lesson = self.lock_lesson_for(&mut tx, id, restored_by).await;
        let retired = match lesson {
            Ok((retired, _)) => retired,
            Err(error) => {
                tx.rollback().await?;
                return Err(error);
            }
        };
        if !retired {
            tx.rollback().await?;
            return Err(refused("that lesson is already active"));
        }
        sqlx::query(
            "UPDATE company_lessons SET retired_at=NULL, retired_by=NULL, retirement_reason='' \
             WHERE id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('lesson_restored',$1,$2)")
            .bind(restored_by)
            .bind(serde_json::json!({ "lesson_id": id }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.lesson(id).await
    }

    /// Lock one lesson for a change by `by`, refusing someone who may not change it. Returns
    /// whether it is retired and the last message counted as a recurrence. The caller ends the
    /// transaction on a refusal.
    async fn lock_lesson_for(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
        by: &str,
    ) -> Result<(bool, Option<i64>)> {
        let Some(authority) = lesson_authority(tx, by).await? else {
            return Err(refused(
                "only the owner, Exec or a team lead may change a lesson",
            ));
        };
        let row = sqlx::query(
            "SELECT team_id, actor_id, retired_at IS NOT NULL AS retired, \
             last_recurrence_message_id FROM company_lessons WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| refused("no lesson with that id"))?;
        if let LessonAuthority::Team(led) = authority {
            let team_id: Option<Uuid> = row.get("team_id");
            let actor_id: Option<String> = row.get("actor_id");
            if !inside_team(tx, led, team_id, actor_id.as_deref()).await? {
                return Err(refused(
                    "a team lead changes only its own team's lessons; ask Exec about this one",
                ));
            }
        }
        Ok((row.get("retired"), row.get("last_recurrence_message_id")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lesson(index: usize, rule: &str, recurrence_count: i32) -> LessonRow {
        LessonRow {
            id: Uuid::from_u128(0x1000_0000_0000_0000_0000_0000_0000_0000 + index as u128),
            rule: rule.to_string(),
            scope_kind: "company".into(),
            team_id: None,
            team_name: None,
            actor_id: None,
            actor_display: None,
            source_message_id: Some(7),
            recorded_by: "exec".into(),
            created_at: Utc::now(),
            status: "active".into(),
            retired_at: None,
            retired_by: None,
            retirement_reason: String::new(),
            recurrence_count,
            last_recurred_at: None,
            last_recurrence_message_id: None,
            learned: false,
        }
    }

    /// A lesson the company learned, last useful `days_ago`.
    fn learned(index: usize, rule: &str, days_ago: i64) -> LessonRow {
        let mut row = lesson(index, rule, 0);
        row.learned = true;
        row.recorded_by = "daemon".into();
        row.source_message_id = None;
        row.created_at = Utc::now() - chrono::Duration::days(60);
        row.last_recurred_at = Some(Utc::now() - chrono::Duration::days(days_ago));
        row
    }

    #[test]
    fn what_the_company_learned_is_shown_apart_from_what_the_owner_ruled() {
        let block = lessons_block(&[
            learned(1, "Show every price with GST included.", 2),
            lesson(2, "Never use stock photos on the site.", 0),
        ]);
        let ruled = block.find("# What the owner has ruled").unwrap();
        let photos = block.find("Never use stock photos").unwrap();
        let heading = block.find("# What the company has learned").unwrap();
        let prices = block.find("Show every price with GST").unwrap();
        assert!(
            ruled < photos && photos < heading && heading < prices,
            "{block}"
        );
        // Only learned lessons: no heading that claims the owner ruled them.
        let only = lessons_block(&[learned(1, "Show every price with GST included.", 2)]);
        assert!(
            !only.contains("the owner has ruled\n")
                && only.starts_with("# What the company has learned"),
            "{only}"
        );
    }

    #[test]
    fn the_cap_holds_and_the_least_recently_useful_is_dropped_first() {
        // Ten rulings and fourteen learned lessons, the fourteenth useful longest ago: twenty
        // are shown, and the four left out are the learned ones useful longest ago.
        let mut lessons: Vec<LessonRow> = (0..10)
            .map(|index| lesson(index, &format!("Owner ruling number {index} here."), 0))
            .collect();
        lessons.extend((0..14).map(|index| {
            learned(
                100 + index,
                &format!("Learned lesson {index} here."),
                index as i64,
            )
        }));
        let (ruled, shown) = lessons_for_wake(&lessons);
        assert_eq!(ruled.len(), 10);
        assert_eq!(shown.len(), LESSONS_SHOWN - 10);
        let block = lessons_block(&lessons);
        for index in 0..10 {
            assert!(
                block.contains(&format!("Learned lesson {index} here.")),
                "{index}"
            );
        }
        for index in 10..14 {
            assert!(
                !block.contains(&format!("Learned lesson {index} here.")),
                "{index}"
            );
        }
        assert!(
            block.contains("- … 4 more not shown: `restless lessons list`"),
            "{block}"
        );
        // Most recently useful first.
        assert!(block.find("Learned lesson 0 here.") < block.find("Learned lesson 9 here."));

        // Thirty rulings and eight learned: the four learned lessons used most recently keep
        // their place, the newest sixteen rulings take the rest, and nothing else is shown.
        let mut lessons: Vec<LessonRow> = (0..30)
            .map(|index| lesson(index, &format!("Owner ruling number {index} here."), 0))
            .collect();
        lessons.extend((0..8).map(|index| {
            learned(
                100 + index,
                &format!("Learned lesson {index} here."),
                index as i64,
            )
        }));
        let (ruled, shown) = lessons_for_wake(&lessons);
        assert_eq!(
            (ruled.len(), shown.len()),
            (LESSONS_SHOWN - LEARNED_LESSONS_KEPT, LEARNED_LESSONS_KEPT)
        );
        let block = lessons_block(&lessons);
        assert!(
            block.contains("Owner ruling number 15 here.")
                && !block.contains("Owner ruling number 16 here.")
        );
        assert!(
            block.contains("Learned lesson 3 here.") && !block.contains("Learned lesson 4 here.")
        );
        assert!(block.contains("- … 18 more not shown"), "{block}");
    }

    #[test]
    fn a_lesson_is_never_cut_in_half() {
        // Long rules: the byte cap leaves some out whole, and every line shown is complete.
        let long = "x".repeat(MAX_LESSON_RULE_CHARS - 10);
        let mut lessons: Vec<LessonRow> = (0..6)
            .map(|index| lesson(index, &format!("{long} {index}"), 0))
            .collect();
        lessons.push(learned(50, "Show every price with GST included.", 1));
        let block = lessons_block(&lessons);
        let lines: Vec<&str> = block
            .lines()
            .filter(|line| line.starts_with("- ") && !line.contains("more not shown"))
            .collect();
        assert!((2..7).contains(&lines.len()), "{} shown", lines.len());
        assert!(lines.iter().all(|line| line.ends_with(')')), "{block}");
        assert!(lines.iter().map(|line| line.len() + 1).sum::<usize>() <= LESSONS_BLOCK_BYTES);
        // The learned lesson kept its place ahead of the long rulings.
        assert!(block.contains("Show every price with GST included."));
        assert!(
            block.contains(&format!("{} more not shown", 7 - lines.len())),
            "{block}"
        );
    }

    #[test]
    fn no_lessons_means_no_block() {
        assert_eq!(lessons_block(&[]), "");
    }

    #[test]
    fn the_block_keeps_the_order_it_is_given_and_names_each_lesson() {
        let mut team = lesson(2, "Quotes go out with GST shown separately.", 2);
        team.scope_kind = "team".into();
        team.team_id = Some(Uuid::from_u128(9));
        team.team_name = Some("Sales".into());
        let block = lessons_block(&[lesson(1, "Never use stock photos on the site.", 0), team]);
        let newest = block.find("Never use stock photos").unwrap();
        let older = block.find("Quotes go out with GST").unwrap();
        assert!(block.starts_with("# What the owner has ruled"));
        assert!(newest < older, "{block}");
        assert!(
            block.contains("(whole company, lesson 10000000)"),
            "{block}"
        );
        assert!(
            block.contains("(team Sales, lesson 10000000, came back 2 times)"),
            "{block}"
        );
        assert!(!block.contains("more not shown"), "{block}");
    }

    #[test]
    fn the_block_stops_at_twenty_lessons_and_points_to_the_rest() {
        let lessons: Vec<_> = (0..25)
            .map(|index| lesson(index, &format!("Short rule number {index} here."), 0))
            .collect();
        let block = lessons_block(&lessons);
        assert_eq!(block.matches("\n- Short rule").count(), LESSONS_SHOWN);
        assert!(block.contains("Short rule number 0 here."));
        assert!(!block.contains("Short rule number 20 here."));
        assert!(
            block.contains("- … 5 more not shown: `restless lessons list`"),
            "{block}"
        );
    }

    #[test]
    fn the_block_stops_at_its_character_budget() {
        let long = "x".repeat(MAX_LESSON_RULE_CHARS - 10);
        let lessons: Vec<_> = (0..10)
            .map(|index| lesson(index, &format!("{long} {index}"), 0))
            .collect();
        let block = lessons_block(&lessons);
        let shown = block.matches("\n- x").count();
        assert!((1..10).contains(&shown), "{shown} shown");
        assert!(block.contains(&format!("{} more not shown", 10 - shown)));
        // The lessons themselves fit the budget; the heading and the pointer are one line each.
        assert!(block.len() <= LESSONS_BLOCK_BYTES + 200);
    }

    #[test]
    fn a_rule_is_one_or_two_plain_sentences() {
        assert!(normalized_lesson_rule("  ").is_err());
        assert!(normalized_lesson_rule("be better").is_err());
        assert!(normalized_lesson_rule("nostockphotosanywhere").is_err());
        assert!(normalized_lesson_rule(&"word ".repeat(100)).is_err());
        assert_eq!(
            normalized_lesson_rule("  No stock photos\n on the site. ").unwrap(),
            "No stock photos on the site."
        );
        // The cap counts characters, not bytes.
        let accented = format!("a b {}", "é".repeat(MAX_LESSON_RULE_CHARS - 4));
        assert!(normalized_lesson_rule(&accented).is_ok());
    }

    #[test]
    fn scope_parses_company_team_and_actor() {
        assert_eq!(LessonScope::parse("company"), Ok(LessonScope::Company));
        let team = Uuid::from_u128(5);
        assert_eq!(
            LessonScope::parse(&format!("team:{team}")),
            Ok(LessonScope::Team(team))
        );
        assert_eq!(
            LessonScope::parse("actor:site-builder"),
            Ok(LessonScope::Actor("site-builder".into()))
        );
        assert!(LessonScope::parse("team:sales").is_err());
        assert!(LessonScope::parse("actor:").is_err());
        assert!(LessonScope::parse("everyone").is_err());
    }
}
