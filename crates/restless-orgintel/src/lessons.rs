//! Company lessons: a correction the owner made, kept as a standing rule.
//!
//! A lesson is one rule in plain words for the whole company, one team or one agent. Every agent
//! it applies to reads it on each wake (`lessons_block`). When the same problem shows up again the
//! lesson counts it rather than being recorded twice, so a rule that does not hold is visible.
//!
//! Recoverable coordination state: a lesson grants no authority and gates nothing.

use super::*;

/// A rule is one or two plain sentences.
pub const MAX_LESSON_RULE_CHARS: usize = 400;
const MIN_LESSON_RULE_CHARS: usize = 12;
const MIN_LESSON_RULE_WORDS: usize = 3;
const MAX_LESSON_REASON_CHARS: usize = 400;
/// What one wake carries; the rest is one `restless lessons list` away.
pub const LESSONS_SHOWN: usize = 20;
pub const LESSONS_BLOCK_CHARS: usize = 2_000;

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
}

impl LessonRow {
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
     l.last_recurrence_message_id \
     FROM company_lessons l \
     LEFT JOIN teams t ON t.id = l.team_id \
     LEFT JOIN actors a ON a.id = l.actor_id";

fn refused(reason: impl Into<String>) -> OrgIntelError {
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

/// The "What the owner has ruled" block for one wake: newest first, at most `LESSONS_SHOWN`
/// lessons and `LESSONS_BLOCK_CHARS` characters of them, then where the rest is. Empty when there
/// are no lessons, so a company without any pays nothing. `lessons` arrive newest first.
pub fn lessons_block(lessons: &[LessonRow]) -> String {
    if lessons.is_empty() {
        return String::new();
    }
    let mut lines = String::new();
    let mut used = 0usize;
    let mut shown = 0usize;
    for lesson in lessons.iter().take(LESSONS_SHOWN) {
        let came_back = match lesson.recurrence_count {
            0 => String::new(),
            1 => ", came back once".to_string(),
            count => format!(", came back {count} times"),
        };
        let line = format!(
            "- {} ({}, lesson {}{came_back})\n",
            lesson.rule,
            lesson.scope_label(),
            lesson.short_id()
        );
        let chars = line.chars().count();
        if used + chars > LESSONS_BLOCK_CHARS {
            break;
        }
        used += chars;
        shown += 1;
        lines.push_str(&line);
    }
    let rest = lessons.len() - shown;
    if rest > 0 {
        lines.push_str(&format!(
            "- … {rest} more not shown: `restless lessons list`\n"
        ));
    }
    format!("# What the owner has ruled [owner policy; follow each one; newest first]\n{lines}")
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

impl OrgIntel {
    /// Record a standing rule. The owner or Exec may record one for the company, a team or an
    /// agent; a team lead only for its own team or its members.
    pub async fn record_lesson(&self, input: NewLesson<'_>) -> Result<LessonRow> {
        let rule = normalized_lesson_rule(input.rule)?;
        let mut tx = self.pool.begin().await?;
        let Some(authority) = lesson_authority(&mut tx, input.recorded_by).await? else {
            tx.rollback().await?;
            return Err(refused(
                "only the owner, Exec or a team lead may record a lesson",
            ));
        };
        let (scope_kind, team_id, actor_id) = match &input.scope {
            LessonScope::Company => ("company", None, None),
            LessonScope::Team(id) => ("team", Some(*id), None),
            LessonScope::Actor(id) => ("actor", None, Some(id.as_str())),
        };
        if let Some(team_id) = team_id {
            let live: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM teams WHERE id=$1 AND disbanded_at IS NULL)",
            )
            .bind(team_id)
            .fetch_one(&mut *tx)
            .await?;
            if !live {
                tx.rollback().await?;
                return Err(refused("no live team with that id"));
            }
        }
        if let Some(actor_id) = actor_id {
            let agent: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
                 AND actor_class='agent')",
            )
            .bind(actor_id)
            .fetch_one(&mut *tx)
            .await?;
            if !agent {
                tx.rollback().await?;
                return Err(refused(format!("no active agent with id {actor_id:?}")));
            }
        }
        if let LessonAuthority::Team(led) = authority {
            if !inside_team(&mut tx, led, team_id, actor_id).await? {
                tx.rollback().await?;
                return Err(refused(
                    "a team lead records lessons for its own team or its members; ask Exec for a wider one",
                ));
            }
        }
        if let Some(message_id) = input.source_message_id {
            if !check_person_message(&mut tx, message_id).await? {
                tx.rollback().await?;
                return Err(refused(format!(
                    "message {message_id} is not a message from a person; cite the message the rule came from"
                )));
            }
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
        }
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
        assert!(block.chars().count() <= LESSONS_BLOCK_CHARS + 200);
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
