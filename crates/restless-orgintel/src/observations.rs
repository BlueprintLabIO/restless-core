//! It learns (Sprint 119). The company got better only when the owner corrected it, and one
//! correction became a standing rule in the same turn. `docs/specs/orgintel.md` §3.1: "One
//! incident should usually create a hypothesis, not a permanent rule."
//!
//! An observation is one thing that went wrong, written down once: what was expected, what
//! happened, and what kind of problem that is. It is not a rule and no agent is shown it on a
//! wake. It becomes a lesson only when the same kind of problem is observed a second time.
//!
//! - **Written without a model** when Work is sent back (by a reviewer, a lead or the owner)
//!   and when an Attempt fails for a reason that is the Work's own. Both are read from rows the
//!   same transaction is already writing.
//! - **Written by Exec** for a one-off correction from the owner (`restless lessons add
//!   --once`). An explicit standing instruction is still a lesson at once (`record_lesson`).
//! - **The same kind** is read the way Sprint 118 reads "the same way" for a failure: a typed
//!   reason (`[kind] …`) is its kind; free text is the same when its first twelve words match
//!   with numbers removed, or when the two share at least three words and half of all their
//!   words. It needs no model, and it misses a repeat that was reworded past that.
//! - **Promotion.** The second observation of a kind inside [`OBSERVATION_LIVES`] makes one
//!   lesson, recorded by the daemon, that names the observations it came from. Its words are
//!   what the lead said it would do differently, when the lead said so, and otherwise the
//!   reason itself. A third observation counts on that lesson instead of making another.
//! - **Expiry.** An observation that is not repeated inside [`OBSERVATION_LIVES`] is never
//!   counted again. Nothing is deleted.
//! - **Pruning.** A learned lesson whose problem has not come back for
//!   [`LEARNED_LESSON_UNUSED_AFTER`] is retired with that reason. It keeps its record, anyone
//!   who may change it can restore it, and it comes back by itself if the problem does. A
//!   lesson the owner ruled is never retired here.
//!
//! The record is an event. No table: an observation has one writer and is read back by kind.
//! Recoverable coordination state: an observation grants nothing and gates nothing.

use super::*;
use crate::health_signals::{outcome_body, overlap, text_signature, words};
use crate::lessons::{admit_lesson, refused};

/// The event that records one observation.
pub const OBSERVATION_EVENT: &str = "work_observation";
/// How many observations of one kind make a lesson.
pub const OBSERVATIONS_BEFORE_LESSON: usize = 2;
/// How long an observation waits for a repeat before it stops counting.
pub const OBSERVATION_LIVES: chrono::Duration = chrono::Duration::days(28);
/// How long a learned lesson is kept after its problem last came back.
pub const LEARNED_LESSON_UNUSED_AFTER: chrono::Duration = chrono::Duration::days(42);
/// Who records a lesson the company learned.
pub const LEARNED_BY: &str = "daemon";
/// Why a learned lesson is retired by the daemon.
pub const UNUSED_LESSON_REASON: &str =
    "The problem has not come back in six weeks, so this is no longer read on each wake.";
const EXPECTED_CHARS: usize = 200;
const HAPPENED_CHARS: usize = 300;
const QUOTED_REASON_CHARS: usize = 240;
const SHARED_WORDS: usize = 3;
const SHARED_SHARE: f64 = 0.5;
const OBSERVATIONS_READ: i64 = 400;

/// Where an observation came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationSource {
    /// A reviewer, a lead or the owner sent the Work back.
    SentBack,
    /// An Attempt failed for a reason that was the Work's own.
    Failed,
    /// The owner corrected something once.
    Correction,
}

impl ObservationSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SentBack => "sent_back",
            Self::Failed => "failed",
            Self::Correction => "correction",
        }
    }
}

/// One thing that went wrong, as recorded.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Observation {
    pub id: i64,
    pub at: DateTime<Utc>,
    /// `sent_back`, `failed` or `correction`.
    pub source: String,
    pub work_id: Option<Uuid>,
    pub work_title: String,
    /// Who sent it back, whose Attempt failed, or who recorded the correction.
    pub by: String,
    pub team_id: Option<Uuid>,
    /// What the Work was meant to produce.
    pub expected: String,
    /// The reason it was sent back, how it failed, or the correction.
    pub happened: String,
    /// What its lead would do differently, when the lead said.
    pub next_time: Option<String>,
    /// The kind of problem, for telling two observations apart.
    pub kind: String,
    /// The lesson this observation was promoted into or counted on.
    pub lesson_id: Option<Uuid>,
}

/// What recording an observation did to the lessons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LessonChange {
    /// The second of its kind: a lesson was recorded.
    Promoted,
    /// Its kind already had a lesson: the lesson counted it.
    Recurred,
    /// Its lesson had been dropped as unused: it is back.
    Restored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observed {
    pub observation_id: i64,
    pub lesson: Option<(Uuid, LessonChange)>,
}

pub(crate) struct NewObservation<'a> {
    /// What makes this the same observation if it is written twice.
    pub key: String,
    pub source: ObservationSource,
    pub work_id: Option<Uuid>,
    pub by: &'a str,
    pub happened: &'a str,
    pub next_time: Option<&'a str>,
    /// For a correction: who the rule would be for. Work observations are scoped by team.
    pub scope: Option<&'a LessonScope>,
    pub source_message_id: Option<i64>,
}

fn one_line(text: &str, chars: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut kept: String = text.chars().take(chars).collect();
    if text.chars().count() > chars {
        kept.push('…');
    }
    kept
}

/// The kind of problem a reason names: a typed reason (`[kind] …`) is its kind; free text is
/// its first twelve words with numbers and ids removed.
pub fn observation_kind(happened: &str) -> String {
    text_signature(happened)
}

/// Whether two reasons name the same kind of problem. A typed reason matches only its own
/// kind. Free text matches when the kinds are equal, or when the two share at least three
/// words and half of all their words (filler and short words left out).
pub fn same_kind(a: &str, b: &str) -> bool {
    let (kind_a, kind_b) = (observation_kind(a), observation_kind(b));
    if kind_a.is_empty() || kind_b.is_empty() {
        return false;
    }
    if kind_a.starts_with('[') || kind_b.starts_with('[') {
        return kind_a == kind_b;
    }
    if kind_a == kind_b {
        return true;
    }
    let (words_a, words_b) = (words(a), words(b));
    words_a.intersection(&words_b).count() >= SHARED_WORDS
        && overlap(&words_a, &words_b) >= SHARED_SHARE
}

/// The words of a learned lesson: what the lead said it would do differently when that reads
/// as a rule, otherwise the reason itself, quoted.
pub fn learned_rule(source: ObservationSource, happened: &str, next_time: Option<&str>) -> String {
    if let Some(rule) = next_time.and_then(|text| normalized_lesson_rule(text).ok()) {
        return rule;
    }
    let reason = one_line(happened, QUOTED_REASON_CHARS);
    let reason = reason.trim_end_matches('.');
    match source {
        ObservationSource::Failed => format!(
            "Work has failed twice the same way: \"{reason}\". Change the approach before it is \
             tried again."
        ),
        ObservationSource::SentBack => format!(
            "Work has been sent back twice for the same reason: \"{reason}\". Check for it before \
             handing work in."
        ),
        ObservationSource::Correction => {
            format!("The owner has corrected this twice: \"{reason}\". Hold to it.")
        }
    }
}

fn observation_from(id: i64, at: DateTime<Utc>, body: &serde_json::Value) -> Observation {
    let text = |key: &str| {
        body.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let uuid = |key: &str| {
        body.get(key)
            .and_then(|v| v.as_str())
            .and_then(|v| Uuid::parse_str(v).ok())
    };
    Observation {
        id,
        at,
        source: text("source"),
        work_id: uuid("work_id"),
        work_title: text("work_title"),
        by: text("by"),
        team_id: uuid("team_id"),
        expected: text("expected"),
        happened: text("happened"),
        next_time: Some(text("next_time")).filter(|next| !next.is_empty()),
        kind: text("kind"),
        lesson_id: uuid("lesson_id"),
    }
}

/// Record one observation inside the transaction that sent the Work back or closed the
/// Attempt, and promote, count or restore a lesson when it is a repeat. Writing the same key
/// twice writes once. Returns `None` when there was nothing to write.
pub(crate) async fn observe_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    now: DateTime<Utc>,
    new: NewObservation<'_>,
) -> Result<Option<Observed>> {
    let happened = one_line(new.happened, HAPPENED_CHARS);
    let kind = observation_kind(&happened);
    if kind.is_empty() {
        return Ok(None);
    }
    // One at a time, so two observations of one kind written at once promote once.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('work_observation'))")
        .execute(&mut **tx)
        .await?;
    let already: bool = sqlx::query_scalar(&format!(
        "SELECT EXISTS(SELECT 1 FROM events WHERE kind='{OBSERVATION_EVENT}' AND body->>'key'=$1)"
    ))
    .bind(&new.key)
    .fetch_one(&mut **tx)
    .await?;
    if already {
        return Ok(None);
    }
    let (work_title, expected, team_id) = match new.work_id {
        None => (String::new(), String::new(), None),
        Some(work_id) => {
            let row = sqlx::query(
                "SELECT w.title, w.outcome, \
                        (SELECT t.id FROM teams t WHERE t.disbanded_at IS NULL \
                            AND (t.id=a.team_id OR t.lead_actor_id=w.owner_id) \
                          ORDER BY COALESCE(t.id=a.team_id, false) DESC, t.created_at LIMIT 1) AS team_id \
                 FROM work w LEFT JOIN actors a ON a.id=w.owner_id WHERE w.id=$1",
            )
            .bind(work_id)
            .fetch_optional(&mut **tx)
            .await?;
            match row {
                None => (String::new(), String::new(), None),
                Some(row) => (
                    row.get::<String, _>("title"),
                    one_line(
                        outcome_body(row.get::<String, _>("outcome").as_str()),
                        EXPECTED_CHARS,
                    ),
                    row.get::<Option<Uuid>, _>("team_id"),
                ),
            }
        }
    };
    let next_time = new
        .next_time
        .map(|text| one_line(text, MAX_LESSON_RULE_CHARS))
        .filter(|text| !text.is_empty());
    // Earlier observations that can still count: recent ones, and any that already has a lesson.
    let earlier: Vec<Observation> = sqlx::query(&format!(
        "SELECT id, created_at, body FROM events WHERE kind='{OBSERVATION_EVENT}' \
           AND (created_at > $1 OR body->>'lesson_id' IS NOT NULL) \
         ORDER BY id DESC LIMIT {OBSERVATIONS_READ}"
    ))
    .bind(now - OBSERVATION_LIVES)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|row| {
        observation_from(
            row.get("id"),
            row.get("created_at"),
            &row.get::<serde_json::Value, _>("body"),
        )
    })
    .filter(|earlier| {
        same_kind(&earlier.happened, &happened)
            || earlier
                .next_time
                .as_deref()
                .is_some_and(|next| same_kind(next, &happened))
    })
    .collect();

    let scope_text = new.scope.map(|scope| match scope {
        LessonScope::Company => "company".to_string(),
        LessonScope::Team(id) => format!("team:{id}"),
        LessonScope::Actor(id) => format!("actor:{id}"),
    });
    let observation_id: i64 = sqlx::query_scalar(&format!(
        "INSERT INTO events (kind, actor_id, body, created_at) \
         VALUES ('{OBSERVATION_EVENT}', $1, $2, $3) RETURNING id"
    ))
    .bind(new.by)
    .bind(serde_json::json!({
        "key": new.key,
        "source": new.source.as_str(),
        "work_id": new.work_id,
        "work_title": work_title,
        "by": new.by,
        "team_id": team_id,
        "expected": expected,
        "happened": happened,
        "next_time": next_time,
        "kind": kind,
        "scope": scope_text,
        "source_message_id": new.source_message_id,
    }))
    .bind(now)
    .fetch_one(&mut **tx)
    .await?;

    let mut change = None;
    if let Some(lesson_id) = earlier.iter().find_map(|earlier| earlier.lesson_id) {
        // The kind already has a lesson: count this on it, or bring it back if it was dropped
        // as unused. A lesson someone retired on purpose stays retired.
        let lesson = sqlx::query(
            "SELECT retired_at IS NOT NULL AS retired, retired_by FROM company_lessons \
             WHERE id=$1 FOR UPDATE",
        )
        .bind(lesson_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(lesson) = lesson {
            let retired: bool = lesson.get("retired");
            let retired_by: Option<String> = lesson.get("retired_by");
            if !retired || retired_by.as_deref() == Some(LEARNED_BY) {
                let count: i32 = sqlx::query_scalar(
                    "UPDATE company_lessons SET recurrence_count = recurrence_count + 1, \
                       last_recurred_at = $2, retired_at = NULL, retired_by = NULL, \
                       retirement_reason = '' \
                     WHERE id=$1 RETURNING recurrence_count",
                )
                .bind(lesson_id)
                .bind(now)
                .fetch_one(&mut **tx)
                .await?;
                sqlx::query(
                    "INSERT INTO events (kind, actor_id, body, created_at) VALUES ($1,$2,$3,$4)",
                )
                .bind(if retired {
                    "lesson_restored"
                } else {
                    "lesson_recurred"
                })
                .bind(LEARNED_BY)
                .bind(serde_json::json!({
                    "lesson_id": lesson_id,
                    "recurrence_count": count,
                    "observation_id": observation_id,
                    "reason": "the same problem was observed again",
                }))
                .bind(now)
                .execute(&mut **tx)
                .await?;
                change = Some((
                    lesson_id,
                    if retired {
                        LessonChange::Restored
                    } else {
                        LessonChange::Recurred
                    },
                ));
            }
        }
        sqlx::query("UPDATE events SET body = body || jsonb_build_object('lesson_id', $2::text) WHERE id=$1")
            .bind(observation_id)
            .bind(lesson_id.to_string())
            .execute(&mut **tx)
            .await?;
    } else if earlier.len() + 1 >= OBSERVATIONS_BEFORE_LESSON {
        // A repeat: one lesson, for the team when every observation was that team's, otherwise
        // for the company. A correction keeps the scope Exec gave it.
        let same_team =
            team_id.filter(|team| earlier.iter().all(|earlier| earlier.team_id == Some(*team)));
        let (scope_kind, lesson_team, lesson_actor): (&str, Option<Uuid>, Option<String>) =
            match (new.scope, same_team) {
                (Some(LessonScope::Team(id)), _) => ("team", Some(*id), None),
                (Some(LessonScope::Actor(id)), _) => ("actor", None, Some(id.clone())),
                (Some(LessonScope::Company), _) => ("company", None, None),
                (None, Some(team)) => ("team", Some(team), None),
                (None, None) => ("company", None, None),
            };
        let said = next_time.as_deref().or_else(|| {
            earlier
                .iter()
                .find_map(|earlier| earlier.next_time.as_deref())
        });
        let rule = learned_rule(new.source, &happened, said);
        let mut from: Vec<i64> = earlier.iter().map(|earlier| earlier.id).collect();
        from.push(observation_id);
        from.sort_unstable();
        // The same words already stand as a lesson: count it there instead of saying it twice.
        let standing: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM company_lessons WHERE retired_at IS NULL AND lower(rule)=lower($1) \
             AND scope_kind=$2 AND team_id IS NOT DISTINCT FROM $3 \
             AND actor_id IS NOT DISTINCT FROM $4 LIMIT 1",
        )
        .bind(&rule)
        .bind(scope_kind)
        .bind(lesson_team)
        .bind(&lesson_actor)
        .fetch_optional(&mut **tx)
        .await?;
        let lesson_id = match standing {
            Some(lesson_id) => {
                sqlx::query(
                    "UPDATE company_lessons SET recurrence_count = recurrence_count + 1, \
                       last_recurred_at = $2 WHERE id=$1",
                )
                .bind(lesson_id)
                .bind(now)
                .execute(&mut **tx)
                .await?;
                change = Some((lesson_id, LessonChange::Recurred));
                lesson_id
            }
            None => {
                let lesson_id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO company_lessons \
                     (id, rule, scope_kind, team_id, actor_id, source_message_id, recorded_by, created_at) \
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
                )
                .bind(lesson_id)
                .bind(&rule)
                .bind(scope_kind)
                .bind(lesson_team)
                .bind(&lesson_actor)
                .bind(new.source_message_id)
                .bind(LEARNED_BY)
                .bind(now)
                .execute(&mut **tx)
                .await?;
                sqlx::query(
                    "INSERT INTO events (kind, actor_id, body, created_at) \
                     VALUES ('lesson_recorded',$1,$2,$3)",
                )
                .bind(LEARNED_BY)
                .bind(serde_json::json!({
                    "lesson_id": lesson_id,
                    "rule": rule,
                    "scope_kind": scope_kind,
                    "team_id": lesson_team,
                    "actor_id": lesson_actor,
                    "source_message_id": new.source_message_id,
                    "learned": true,
                    "observation_kind": kind,
                    "promoted_from": from,
                }))
                .bind(now)
                .execute(&mut **tx)
                .await?;
                change = Some((lesson_id, LessonChange::Promoted));
                lesson_id
            }
        };
        sqlx::query(
            "UPDATE events SET body = body || jsonb_build_object('lesson_id', $2::text) \
             WHERE id = ANY($1)",
        )
        .bind(&from)
        .bind(lesson_id.to_string())
        .execute(&mut **tx)
        .await?;
    }
    Ok(Some(Observed {
        observation_id,
        lesson: change,
    }))
}

/// Work was sent back: one observation, keyed by the Work and the revision that was rejected.
pub(crate) async fn observe_sent_back_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    work_id: Uuid,
    by: &str,
    reason: &str,
) -> Result<Option<Observed>> {
    let revision: Option<i64> = sqlx::query_scalar("SELECT revision FROM work WHERE id=$1")
        .bind(work_id)
        .fetch_optional(&mut **tx)
        .await?;
    let Some(revision) = revision else {
        return Ok(None);
    };
    observe_in_tx(
        tx,
        Utc::now(),
        NewObservation {
            key: format!("sent-back:{work_id}:{revision}"),
            source: ObservationSource::SentBack,
            work_id: Some(work_id),
            by,
            happened: reason,
            next_time: None,
            scope: None,
            source_message_id: None,
        },
    )
    .await
}

/// An Attempt failed: one observation, when the failure was the Work's own (Sprint 118's
/// reading: not the platform's, the provider's or a wait on a person).
pub(crate) async fn observe_failure_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    work_id: Uuid,
    attempt_id: Uuid,
    actor: &str,
    summary: &str,
) -> Result<Option<Observed>> {
    if crate::health_signals::failure_signature(WorkAttemptState::Failed, summary).is_none() {
        return Ok(None);
    }
    observe_in_tx(
        tx,
        Utc::now(),
        NewObservation {
            key: format!("failed:{attempt_id}"),
            source: ObservationSource::Failed,
            work_id: Some(work_id),
            by: actor,
            happened: summary,
            next_time: None,
            scope: None,
            source_message_id: None,
        },
    )
    .await
}

/// What a lead is asked, in the message that tells it Work was sent back. One sentence in a
/// wake that was already happening; nothing waits for the answer.
pub fn next_time_ask(work_id: Uuid) -> String {
    format!(
        " If you can say in one sentence what you would do differently next time, record it: \
         `restless lessons add --once --work {work_id} --rule '<one sentence>'`. It is noted, not \
         a rule; it becomes one only if the same thing happens again."
    )
}

impl OrgIntel {
    /// A one-off correction from the owner: an observation, not a lesson. The second of its
    /// kind becomes one. Who may record it, and for whom, is as for a lesson.
    pub async fn record_correction_at(
        &self,
        now: DateTime<Utc>,
        correction: NewLesson<'_>,
    ) -> Result<Observed> {
        let rule = normalized_lesson_rule(correction.rule)?;
        let mut tx = self.pool.begin().await?;
        if let Err(error) = admit_lesson(
            &mut tx,
            correction.recorded_by,
            &correction.scope,
            correction.source_message_id,
        )
        .await
        {
            tx.rollback().await?;
            return Err(error);
        }
        let key = match correction.source_message_id {
            Some(message_id) => format!("correction:{message_id}:{}", observation_kind(&rule)),
            None => format!("correction:{}", Uuid::new_v4()),
        };
        let observed = observe_in_tx(
            &mut tx,
            now,
            NewObservation {
                key,
                source: ObservationSource::Correction,
                work_id: None,
                by: correction.recorded_by,
                happened: &rule,
                next_time: Some(&rule),
                scope: Some(&correction.scope),
                source_message_id: correction.source_message_id,
            },
        )
        .await;
        match observed {
            Ok(Some(observed)) => {
                tx.commit().await?;
                Ok(observed)
            }
            Ok(None) => {
                tx.rollback().await?;
                Err(refused(
                    "that correction is already noted from that message",
                ))
            }
            Err(error) => {
                tx.rollback().await?;
                Err(error)
            }
        }
    }

    /// What a lead would do differently next time, said after its Work was sent back or
    /// failed. It is kept on that Work's latest observation. If that observation has already
    /// become a learned lesson, the lesson takes these words: a lead's own sentence reads
    /// better than a quoted reason. Only the Work's owner, its lead, Exec or the owner may say.
    pub async fn note_next_time(
        &self,
        work_id: Uuid,
        by: &str,
        next_time: &str,
    ) -> Result<Observation> {
        let said = normalized_lesson_rule(next_time)?;
        let mut tx = self.pool.begin().await?;
        let may: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors me WHERE me.id=$2 AND me.retired_at IS NULL) \
               AND ($2 IN ('owner','exec') OR EXISTS ( \
                 SELECT 1 FROM work w LEFT JOIN actors a ON a.id=w.owner_id \
                 LEFT JOIN teams t ON t.id=a.team_id AND t.disbanded_at IS NULL \
                 WHERE w.id=$1 AND (w.owner_id=$2 OR t.lead_actor_id=$2)))",
        )
        .bind(work_id)
        .bind(by)
        .fetch_one(&mut *tx)
        .await?;
        if !may {
            tx.rollback().await?;
            return Err(refused(
                "only the Work's owner, its lead or Exec may say what to do differently on it",
            ));
        }
        let row = sqlx::query(&format!(
            "SELECT id, created_at, body FROM events WHERE kind='{OBSERVATION_EVENT}' \
               AND body->>'work_id'=$1 ORDER BY id DESC LIMIT 1 FOR UPDATE"
        ))
        .bind(work_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.rollback().await?;
            return Err(refused(
                "nothing has been sent back or has failed on that Work, so there is nothing to note; \
                 for a standing instruction from the owner use `restless lessons add` without --once",
            ));
        };
        let id: i64 = row.get("id");
        let body: serde_json::Value = sqlx::query_scalar(
            "UPDATE events SET body = body || jsonb_build_object('next_time', $2::text, 'noted_by', $3::text) \
             WHERE id=$1 RETURNING body",
        )
        .bind(id)
        .bind(&said)
        .bind(by)
        .fetch_one(&mut *tx)
        .await?;
        let observation = observation_from(id, row.get("created_at"), &body);
        if let Some(lesson_id) = observation.lesson_id {
            let before: Option<String> = sqlx::query_scalar(
                "SELECT rule FROM company_lessons WHERE id=$1 AND retired_at IS NULL \
                   AND recorded_by=$2 AND lower(rule) <> lower($3) FOR UPDATE",
            )
            .bind(lesson_id)
            .bind(LEARNED_BY)
            .bind(&said)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(before) = before {
                sqlx::query("UPDATE company_lessons SET rule=$2 WHERE id=$1")
                    .bind(lesson_id)
                    .bind(&said)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query(
                    "INSERT INTO events (kind, actor_id, body) VALUES ('lesson_reworded',$1,$2)",
                )
                .bind(by)
                .bind(serde_json::json!({
                    "lesson_id": lesson_id,
                    "from": before,
                    "to": said,
                    "observation_id": id,
                }))
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(observation)
    }

    /// The latest observation on a Work, when nobody has yet said what they would do
    /// differently. A lead whose own Work was sent back is asked on its next Attempt.
    pub async fn unanswered_observation(&self, work_id: Uuid) -> Result<Option<Observation>> {
        Ok(sqlx::query(&format!(
            "SELECT id, created_at, body FROM events WHERE kind='{OBSERVATION_EVENT}' \
               AND body->>'work_id'=$1 ORDER BY id DESC LIMIT 1"
        ))
        .bind(work_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .map(|row| {
            observation_from(
                row.get("id"),
                row.get("created_at"),
                &row.get::<serde_json::Value, _>("body"),
            )
        })
        .filter(|observation| observation.next_time.is_none()))
    }

    /// The observations on record, newest first.
    pub async fn list_observations(&self, limit: i64) -> Result<Vec<Observation>> {
        Ok(sqlx::query(&format!(
            "SELECT id, created_at, body FROM events WHERE kind='{OBSERVATION_EVENT}' \
             ORDER BY id DESC LIMIT $1"
        ))
        .bind(limit.clamp(1, OBSERVATIONS_READ))
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            observation_from(
                row.get("id"),
                row.get("created_at"),
                &row.get::<serde_json::Value, _>("body"),
            )
        })
        .collect())
    }

    /// The observations a lesson was promoted from or counted on, oldest first.
    pub async fn lesson_observations(&self, lesson_id: Uuid) -> Result<Vec<Observation>> {
        Ok(sqlx::query(&format!(
            "SELECT id, created_at, body FROM events WHERE kind='{OBSERVATION_EVENT}' \
               AND body->>'lesson_id'=$1 ORDER BY id"
        ))
        .bind(lesson_id.to_string())
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| {
            observation_from(
                row.get("id"),
                row.get("created_at"),
                &row.get::<serde_json::Value, _>("body"),
            )
        })
        .collect())
    }

    /// Retire every learned lesson whose problem has not come back for
    /// [`LEARNED_LESSON_UNUSED_AFTER`], with the reason. No model. A lesson the owner ruled is
    /// never touched. Returns what was retired.
    pub async fn prune_learned_lessons_at(&self, now: DateTime<Utc>) -> Result<Vec<LessonRow>> {
        let mut tx = self.pool.begin().await?;
        let retired: Vec<Uuid> = sqlx::query_scalar(
            "UPDATE company_lessons SET retired_at=$1, retired_by=$2, retirement_reason=$3 \
             WHERE retired_at IS NULL AND recorded_by=$2 \
               AND GREATEST(created_at, COALESCE(last_recurred_at, created_at)) < $4 \
             RETURNING id",
        )
        .bind(now)
        .bind(LEARNED_BY)
        .bind(UNUSED_LESSON_REASON)
        .bind(now - LEARNED_LESSON_UNUSED_AFTER)
        .fetch_all(&mut *tx)
        .await?;
        for id in &retired {
            sqlx::query(
                "INSERT INTO events (kind, actor_id, body, created_at) \
                 VALUES ('lesson_retired',$1,$2,$3)",
            )
            .bind(LEARNED_BY)
            .bind(serde_json::json!({ "lesson_id": id, "reason": UNUSED_LESSON_REASON, "unused": true }))
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        let mut rows = Vec::with_capacity(retired.len());
        for id in retired {
            rows.push(self.lesson(id).await?);
        }
        Ok(rows)
    }

    /// What the company changed about how it works after `since`, one plain line each, for the
    /// weekly message to the owner: rules the owner gave, rules the company learned, rules
    /// dropped or brought back, team changes and what the team reviews concluded. No ids, no
    /// commands. Empty when nothing changed. Read from rows; no model.
    pub async fn working_changes_since(&self, since: Option<DateTime<Utc>>) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        let scope = |kind: &str, team: Option<String>, actor: Option<String>| match kind {
            "team" => format!("for the {} team", team.unwrap_or_else(|| "one".into())),
            "actor" => format!("for {}", actor.unwrap_or_else(|| "one person".into())),
            _ => "for the whole company".to_string(),
        };
        for row in sqlx::query(
            "SELECT e.kind, e.body, l.rule, l.scope_kind, t.name AS team_name, \
                    a.display AS actor_display, l.recorded_by, l.retirement_reason, \
                    l.retired_at IS NOT NULL AS retired \
             FROM events e JOIN company_lessons l ON l.id::text = e.body->>'lesson_id' \
             LEFT JOIN teams t ON t.id=l.team_id LEFT JOIN actors a ON a.id=l.actor_id \
             WHERE e.kind IN ('lesson_recorded','lesson_retired','lesson_restored') \
               AND ($1::timestamptz IS NULL OR e.created_at > $1) ORDER BY e.id LIMIT 40",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?
        {
            let kind: String = row.get("kind");
            let rule = one_line(row.get::<String, _>("rule").as_str(), 160);
            let who = scope(
                row.get::<String, _>("scope_kind").as_str(),
                row.get("team_name"),
                row.get("actor_display"),
            );
            let learned = row.get::<String, _>("recorded_by") == LEARNED_BY;
            let body: serde_json::Value = row.get("body");
            lines.push(match kind.as_str() {
                "lesson_recorded" if learned => {
                    format!("Learned, {who}, after the same thing went wrong twice: {rule}")
                }
                "lesson_recorded" => format!("New rule from the owner, {who}: {rule}"),
                "lesson_retired" => {
                    let reason = body
                        .get("reason")
                        .and_then(|reason| reason.as_str())
                        .unwrap_or_default();
                    format!("Dropped a rule ({rule}) because: {}", one_line(reason, 160))
                }
                _ => format!("Brought a rule back, {who}: {rule}"),
            });
        }
        for row in sqlx::query(
            "SELECT e.kind, e.body, t.name AS team_name, a.display AS actor_display \
             FROM events e \
             LEFT JOIN teams t ON t.id::text = e.body->>'team_id' \
             LEFT JOIN actors a ON a.id = e.body->>'actor_id' \
             WHERE e.kind IN ('team_commissioned','team_disbanded','team_lead_changed','actor_retired') \
               AND ($1::timestamptz IS NULL OR e.created_at > $1) ORDER BY e.id LIMIT 20",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?
        {
            let kind: String = row.get("kind");
            let body: serde_json::Value = row.get("body");
            let team: String = row
                .get::<Option<String>, _>("team_name")
                .unwrap_or_else(|| "A".into());
            let person: String = row
                .get::<Option<String>, _>("actor_display")
                .unwrap_or_else(|| "Someone".into());
            let reason = body
                .get("reason")
                .and_then(|reason| reason.as_str())
                .map(|reason| one_line(reason, 160))
                .filter(|reason| !reason.is_empty());
            let why = reason.map_or(String::new(), |reason| format!(" because: {reason}"));
            lines.push(match kind.as_str() {
                "team_commissioned" => format!("Started the {team} team{why}"),
                "team_disbanded" => format!("Closed the {team} team{why}"),
                "team_lead_changed" => format!("The {team} team has a new lead{why}"),
                _ => format!("{person} was let go{why}"),
            });
        }
        for row in sqlx::query(
            "SELECT reviewed_at, conclusion FROM org_reviews WHERE ($1::timestamptz IS NULL OR reviewed_at > $1) \
             ORDER BY id LIMIT 4",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?
        {
            lines.push(format!(
                "Reviewed how the team is organised: {}",
                one_line(row.get::<String, _>("conclusion").as_str(), 240)
            ));
        }
        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_reason_is_its_kind_and_free_text_is_compared_by_its_words() {
        // Typed: the kind is everything, whatever follows.
        assert!(same_kind(
            "[pricing] The quote shows prices without GST.",
            "[Pricing] GST missing from the menu totals."
        ));
        assert!(!same_kind(
            "[pricing] The quote shows prices without GST.",
            "[tone] The quote shows prices without GST."
        ));
        assert!(!same_kind(
            "[pricing] prices without GST",
            "prices without GST on the quote"
        ));
        // Free text: the same opening words, numbers aside.
        assert!(same_kind(
            "3 of 9 links on the page are broken",
            "5 of 12 links on the page are broken"
        ));
        // Free text: reworded a little, most of the words shared.
        assert!(same_kind(
            "Prices are shown without GST.",
            "The quote shows prices without GST"
        ));
        // Not the same: one shared word, or nothing said.
        assert!(!same_kind(
            "Prices are shown without GST.",
            "The photos are stock photos."
        ));
        assert!(!same_kind("", ""));
        assert!(!same_kind("Prices are shown without GST.", "  "));
    }

    #[test]
    fn a_learned_lesson_uses_the_leads_words_or_quotes_the_reason() {
        assert_eq!(
            learned_rule(
                ObservationSource::SentBack,
                "Prices are shown without GST.",
                Some("  Show every price with GST included.  ")
            ),
            "Show every price with GST included."
        );
        let quoted = learned_rule(
            ObservationSource::SentBack,
            "Prices are shown without GST.",
            None,
        );
        assert_eq!(
            quoted,
            "Work has been sent back twice for the same reason: \"Prices are shown without GST\". \
             Check for it before handing work in."
        );
        // What the lead said is not a rule anyone can follow: the reason is quoted instead.
        assert!(learned_rule(
            ObservationSource::Failed,
            "the build ran out of memory",
            Some("ok")
        )
        .starts_with("Work has failed twice the same way: \"the build ran out of memory\""));
        // Always a rule the lessons table accepts, however long the reason.
        let long = learned_rule(ObservationSource::Correction, &"word ".repeat(200), None);
        assert!(normalized_lesson_rule(&long).is_ok(), "{long}");
    }

    #[test]
    fn the_ask_rides_the_message_the_lead_already_gets() {
        let ask = next_time_ask(Uuid::nil());
        assert!(ask.contains("restless lessons add --once --work 00000000"));
        assert!(ask.contains("not a rule"));
        assert!(ask.len() < 300, "{} bytes", ask.len());
    }
}
