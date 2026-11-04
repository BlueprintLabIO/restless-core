//! Rough progress estimates on Work (owner decision, 3 November 2026).
//!
//! The actor that created a Work (`work.commissioned_by`) is the source of truth for how far it
//! has come: at each checkpoint (an Attempt ends, a handoff is raised or settled, a review
//! returns) it records one honest overall estimate, rounded to tens, with one line on what is
//! left. Exec may record one too. Done Work is 100%, recorded when it completes; abandoned Work
//! keeps its last estimate. A blocked estimate drops the number and says why.
//!
//! When an estimate moves 20 points or more from the last one the owner was told, or the Work
//! completes after being estimated, one compact line goes into the creator's chat with the owner.

use super::*;

/// How many recent estimates each Work carries in projections: enough for a small trend.
pub const PROGRESS_HISTORY: i64 = 8;
/// How far an estimate must move from the last one posted before chat hears about it.
pub const PROGRESS_POST_STEP: i16 = 20;
const NEXT_MAX_CHARS: usize = 160;
const LINE_TITLE_MAX_CHARS: usize = 80;

/// What a creator reports at a checkpoint.
#[derive(Debug, Clone, Copy)]
pub enum ProgressReport<'a> {
    /// Overall percent (rounded to tens; 100 is kept for done Work) and what is left.
    Estimate { percent: i16, next: &'a str },
    /// No number: what it waits on.
    Blocked { why: &'a str },
}

/// One line, trimmed, inner whitespace collapsed, bounded.
fn one_line(text: &str, what: &str) -> Result<String> {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.is_empty() {
        return Err(OrgIntelError::InvalidWork(format!(
            "a progress estimate needs {what} in one line"
        )));
    }
    if line.chars().count() > NEXT_MAX_CHARS {
        return Err(OrgIntelError::InvalidWork(format!(
            "keep {what} under {NEXT_MAX_CHARS} characters: one honest line"
        )));
    }
    Ok(line)
}

/// Round to the nearest ten: estimates are rough, and say so.
pub fn rounded_percent(percent: i16) -> Result<i16> {
    if !(0..=100).contains(&percent) {
        return Err(OrgIntelError::InvalidWork(
            "a progress estimate is a percent from 0 to 100".into(),
        ));
    }
    let rounded = (percent + 5) / 10 * 10;
    if rounded >= 100 {
        return Err(OrgIntelError::InvalidWork(
            "100% is for done Work and is recorded when the Work completes; say 90 if only the last check is left".into(),
        ));
    }
    Ok(rounded)
}

fn short_title(title: &str) -> String {
    let title = title.trim();
    if title.chars().count() <= LINE_TITLE_MAX_CHARS {
        return title.to_string();
    }
    let cut = title
        .chars()
        .take(LINE_TITLE_MAX_CHARS - 1)
        .collect::<String>();
    format!("{}…", cut.trim_end())
}

/// The compact chat line for an estimate, e.g. "Magnets: ~60% (next: export print files)".
pub fn progress_line(title: &str, percent: i16, next: &str) -> String {
    if percent >= 100 {
        format!("{}: done (100%)", short_title(title))
    } else {
        format!(
            "{}: ~{percent}% (next: {})",
            short_title(title),
            next.trim()
        )
    }
}

/// Whether a move from the last posted percent deserves a chat line.
pub fn progress_moves_enough(last_posted: Option<i16>, percent: i16) -> bool {
    (percent - last_posted.unwrap_or(0)).abs() >= PROGRESS_POST_STEP
        || (percent >= 100 && last_posted != Some(100))
}

const PROGRESS_COLUMNS: &str = "id, work_id, state, percent, next, estimated_by, recorded_at";

/// Each Work's recent estimates, oldest first, optionally for some Work only.
pub(crate) async fn recent_progress_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    work_ids: Option<&[Uuid]>,
) -> Result<Vec<WorkProgressRow>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {PROGRESS_COLUMNS} FROM ( \
           SELECT {PROGRESS_COLUMNS}, \
                  row_number() OVER (PARTITION BY work_id ORDER BY id DESC) AS recent \
           FROM work_progress WHERE ($1::uuid[] IS NULL OR work_id = ANY($1)) \
         ) progress WHERE recent <= $2 ORDER BY work_id, id"
    ))
    .bind(work_ids)
    .bind(PROGRESS_HISTORY)
    .fetch_all(&mut **tx)
    .await?)
}

/// Post one compact line from the creator into its chat with the owner. The line is already
/// read: it informs, it never asks, so it raises no unread mark and wakes nobody.
async fn post_progress_line_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    creator: &str,
    line: &str,
) -> Result<Option<i64>> {
    let speaks: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND actor_class='agent' \
         AND retired_at IS NULL)",
    )
    .bind(creator)
    .fetch_one(&mut **tx)
    .await?;
    if !speaks {
        return Ok(None);
    }
    let room_id = ensure_direct_message_room_in_tx(tx, creator, None).await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO messages (room_id,from_actor,to_actor,body,read_at) \
         VALUES ($1,$2,NULL,$3,now()) RETURNING id",
    )
    .bind(room_id)
    .bind(creator)
    .bind(line)
    .fetch_one(&mut **tx)
    .await?;
    Ok(Some(id))
}

async fn last_posted_percent_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    work_id: Uuid,
) -> Result<Option<i16>> {
    Ok(sqlx::query_scalar(
        "SELECT percent FROM work_progress \
         WHERE work_id=$1 AND posted_message_id IS NOT NULL ORDER BY id DESC LIMIT 1",
    )
    .bind(work_id)
    .fetch_optional(&mut **tx)
    .await?
    .flatten())
}

/// Record 100% as Work completes, inside the transaction that completes it. A Work that was
/// estimated along the way closes its story in chat with one "done" line.
pub(crate) async fn record_done_progress_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    work_id: Uuid,
) -> Result<()> {
    let Some((creator, title)) = sqlx::query_as::<_, (String, String)>(
        "SELECT commissioned_by, title FROM work WHERE id=$1",
    )
    .bind(work_id)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(());
    };
    let latest: Option<String> = sqlx::query_scalar(
        "SELECT state FROM work_progress WHERE work_id=$1 ORDER BY id DESC LIMIT 1",
    )
    .bind(work_id)
    .fetch_optional(&mut **tx)
    .await?;
    if latest.as_deref() == Some("done") {
        return Ok(());
    }
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO work_progress (work_id,state,percent,next,estimated_by) \
         VALUES ($1,'done',100,'',$2) RETURNING id",
    )
    .bind(work_id)
    .bind(&creator)
    .fetch_one(&mut **tx)
    .await?;
    // Only a Work the owner has heard about gets its closing line; quiet Work stays quiet.
    let told: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM work_progress \
         WHERE work_id=$1 AND posted_message_id IS NOT NULL)",
    )
    .bind(work_id)
    .fetch_one(&mut **tx)
    .await?;
    if told {
        if let Some(message_id) =
            post_progress_line_in_tx(tx, &creator, &progress_line(&title, 100, "")).await?
        {
            sqlx::query("UPDATE work_progress SET posted_message_id=$2 WHERE id=$1")
                .bind(id)
                .bind(message_id)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

impl OrgIntel {
    /// Record the creator's overall estimate for one Work at a checkpoint.
    pub async fn record_work_progress(
        &self,
        work_id: Uuid,
        by: &str,
        report: ProgressReport<'_>,
    ) -> Result<WorkProgressRecorded> {
        let (state, percent, next) = match report {
            ProgressReport::Estimate { percent, next } => (
                "estimate",
                Some(rounded_percent(percent)?),
                one_line(next, "what is left")?,
            ),
            ProgressReport::Blocked { why } => {
                ("blocked", None, one_line(why, "why it is blocked")?)
            }
        };
        let mut tx = self.pool.begin().await?;
        let (creator, status, title) = sqlx::query_as::<_, (String, WorkStatus, String)>(
            "SELECT commissioned_by, status, title FROM work WHERE id=$1 FOR UPDATE",
        )
        .bind(work_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork(format!("Work {work_id} does not exist")))?;
        if by != creator && by != "exec" {
            tx.rollback().await?;
            return Err(OrgIntelError::InvalidWork(format!(
                "only the Work's creator ({creator}) or Exec records its progress estimate"
            )));
        }
        if matches!(status, WorkStatus::Completed | WorkStatus::Abandoned) {
            tx.rollback().await?;
            return Err(OrgIntelError::InvalidWork(
                "settled Work keeps its last estimate: done Work is 100%, stopped Work stays where it stopped".into(),
            ));
        }
        let last_posted = last_posted_percent_in_tx(&mut tx, work_id).await?;
        let progress: WorkProgressRow = sqlx::query_as(&format!(
            "INSERT INTO work_progress (work_id,state,percent,next,estimated_by) \
             VALUES ($1,$2,$3,$4,$5) RETURNING {PROGRESS_COLUMNS}"
        ))
        .bind(work_id)
        .bind(state)
        .bind(percent)
        .bind(&next)
        .bind(by)
        .fetch_one(&mut *tx)
        .await?;
        let mut posted_message_id = None;
        if let Some(percent) =
            percent.filter(|percent| progress_moves_enough(last_posted, *percent))
        {
            // The creator's thread with the owner, even when Exec records the estimate for it.
            posted_message_id =
                post_progress_line_in_tx(&mut tx, &creator, &progress_line(&title, percent, &next))
                    .await?;
            if let Some(message_id) = posted_message_id {
                sqlx::query("UPDATE work_progress SET posted_message_id=$2 WHERE id=$1")
                    .bind(progress.id)
                    .bind(message_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        sqlx::query("INSERT INTO events (kind, actor_id, body) VALUES ('work_progress',$1,$2)")
            .bind(by)
            .bind(serde_json::json!({
                "work_id": work_id,
                "state": state,
                "percent": percent,
                "next": next,
                "posted_message_id": posted_message_id,
            }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(WorkProgressRecorded {
            progress,
            posted_message_id,
        })
    }

    /// Recent estimates, oldest first: for one Work, or for every Work.
    pub async fn work_progress(&self, work_id: Option<Uuid>) -> Result<Vec<WorkProgressRow>> {
        let mut tx = self.pool.begin().await?;
        let ids = work_id.map(|id| vec![id]);
        let rows = recent_progress_in_tx(&mut tx, ids.as_deref()).await?;
        tx.commit().await?;
        Ok(rows)
    }

    /// Open Work this actor created that has passed a checkpoint since its last estimate: an
    /// Attempt finished, or a handoff was raised or settled. Newest first, bounded.
    pub async fn work_progress_owed(&self, creator: &str) -> Result<Vec<WorkProgressOwed>> {
        Ok(sqlx::query_as(
            "SELECT w.id AS work_id, w.title, w.status, \
                    latest.percent AS last_percent, latest.state AS last_state \
             FROM work w \
             LEFT JOIN LATERAL (SELECT percent, state, recorded_at FROM work_progress p \
                                WHERE p.work_id=w.id ORDER BY p.id DESC LIMIT 1) latest ON true \
             WHERE w.commissioned_by=$1 AND w.status IN ('proposed','active','blocked') \
               AND GREATEST( \
                     (SELECT max(a.finished_at) FROM work_attempts a WHERE a.work_id=w.id), \
                     (SELECT max(GREATEST(h.created_at, COALESCE(h.resolved_at, h.created_at))) \
                        FROM owner_handoffs h WHERE h.work_id=w.id) \
                   ) > COALESCE(latest.recorded_at, '-infinity'::timestamptz) \
             ORDER BY w.updated_at DESC LIMIT 12",
        )
        .bind(creator)
        .fetch_all(&self.pool)
        .await?)
    }
}

/// A Goal's roll-up across its Work: every live part counts equally. Done is 100, an estimate
/// counts as given, a blocked part keeps its last number, and a part nobody has estimated yet
/// counts as 0. Stopped Work is left out. `None` when no part has an estimate and none is done,
/// so a Goal nobody has estimated shows nothing rather than 0%.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct GoalProgress {
    pub percent: Option<i16>,
    pub done: usize,
    pub total: usize,
}

/// The current number for one Work: done is 100, otherwise its newest numeric estimate.
pub fn current_percent(work: &WorkRow, progress: &[WorkProgressRow]) -> Option<i16> {
    if work.status == WorkStatus::Completed {
        return Some(100);
    }
    progress
        .iter()
        .filter(|row| row.work_id == work.id)
        .max_by_key(|row| row.id)
        .and_then(|latest| {
            latest.percent.or_else(|| {
                // Blocked drops the number in words; the bar keeps the last one it had.
                progress
                    .iter()
                    .filter(|row| row.work_id == work.id && row.percent.is_some())
                    .max_by_key(|row| row.id)
                    .and_then(|row| row.percent)
            })
        })
}

pub fn goal_progress(
    goal_id: Uuid,
    work: &[WorkRow],
    progress: &[WorkProgressRow],
) -> GoalProgress {
    let parts = work
        .iter()
        .filter(|item| item.goal_id == Some(goal_id) && item.status != WorkStatus::Abandoned)
        .collect::<Vec<_>>();
    let done = parts
        .iter()
        .filter(|item| item.status == WorkStatus::Completed)
        .count();
    let numbers = parts
        .iter()
        .map(|item| current_percent(item, progress))
        .collect::<Vec<_>>();
    let percent = (!parts.is_empty() && numbers.iter().any(Option::is_some)).then(|| {
        let sum: i32 = numbers.iter().map(|n| i32::from(n.unwrap_or(0))).sum();
        let average = (sum as f64 / parts.len() as f64).round() as i16;
        // 100% means every part is done; rounding never says it early.
        if done < parts.len() {
            average.min(99)
        } else {
            average
        }
    });
    GoalProgress {
        percent,
        done,
        total: parts.len(),
    }
}

/// The marker a turn's final reply carries its estimates in, so recording them costs no tool
/// call (S110): `<!--restless-progress:[{"work":"<id>","percent":60,"next":"…"},
/// {"work":"<id>","blocked":"…"}]-->`. The Runtime takes it out before the reply is saved.
pub const PROGRESS_MARKER: &str = "<!--restless-progress:";

/// One estimate a reply carried.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ReportedProgress {
    pub work: Uuid,
    #[serde(default)]
    pub percent: Option<i16>,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub blocked: Option<String>,
}

/// The reply without its progress markers, and the estimates they held. A marker that does not
/// parse is still removed (the owner never sees it) and simply records nothing.
pub fn take_progress_markers(text: &str) -> (String, Vec<ReportedProgress>) {
    if !text.contains(PROGRESS_MARKER) {
        return (text.to_string(), Vec::new());
    }
    let mut clean = String::with_capacity(text.len());
    let mut reported = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(PROGRESS_MARKER) {
        clean.push_str(&rest[..start]);
        let after = &rest[start + PROGRESS_MARKER.len()..];
        let Some(end) = after.find("-->") else {
            rest = "";
            break;
        };
        let payload = after[..end].trim();
        if let Ok(items) = serde_json::from_str::<Vec<ReportedProgress>>(payload) {
            reported.extend(items);
        } else if let Ok(item) = serde_json::from_str::<ReportedProgress>(payload) {
            reported.push(item);
        }
        rest = &after[end + 3..];
    }
    clean.push_str(rest);
    (clean.trim().to_string(), reported)
}

impl OrgIntel {
    /// Record the estimates a reply carried, as `by`. Each stands alone: one the creator may not
    /// record (wrong Work, finished Work, a bad number) is skipped with its reason, never failing
    /// the turn that carried it.
    pub async fn record_reported_progress(
        &self,
        by: &str,
        reported: &[ReportedProgress],
    ) -> Vec<std::result::Result<WorkProgressRecorded, String>> {
        let mut results = Vec::with_capacity(reported.len());
        for item in reported {
            let report = match (item.percent, item.next.as_deref(), item.blocked.as_deref()) {
                (_, _, Some(why)) => ProgressReport::Blocked { why },
                (Some(percent), Some(next), None) => ProgressReport::Estimate { percent, next },
                _ => {
                    results.push(Err(format!(
                        "Work {}: give percent with next, or blocked",
                        item.work
                    )));
                    continue;
                }
            };
            results.push(
                self.record_work_progress(item.work, by, report)
                    .await
                    .map_err(|error| format!("Work {}: {error}", item.work)),
            );
        }
        results
    }
}

/// The section a creator's wake carries when estimates are owed: the doctrine, then one line per
/// Work. Empty when nothing is owed, so a quiet wake costs nothing.
pub fn progress_owed_section(owed: &[WorkProgressOwed], doctrine: &str) -> String {
    if owed.is_empty() {
        return String::new();
    }
    let lines = owed
        .iter()
        .map(|item| {
            let last = match (item.last_state.as_deref(), item.last_percent) {
                (Some("blocked"), _) => "last: blocked".to_string(),
                (_, Some(percent)) => format!("last: ~{percent}%"),
                _ => "no estimate yet".to_string(),
            };
            format!(
                "- Work {} \"{}\" [{}] · {last}",
                item.work_id,
                short_title(&item.title),
                format!("{:?}", item.status).to_lowercase()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "# Progress estimates owed [Work you created]\n{}\n{lines}\n",
        doctrine.trim()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_round_to_tens_and_keep_100_for_done() {
        assert_eq!(rounded_percent(0).unwrap(), 0);
        assert_eq!(rounded_percent(44).unwrap(), 40);
        assert_eq!(rounded_percent(45).unwrap(), 50);
        assert_eq!(rounded_percent(90).unwrap(), 90);
        assert!(rounded_percent(95).is_err());
        assert!(rounded_percent(100).is_err());
        assert!(rounded_percent(-1).is_err());
        assert!(rounded_percent(101).is_err());
    }

    #[test]
    fn chat_hears_about_big_moves_and_done_only() {
        assert!(!progress_moves_enough(None, 10));
        assert!(progress_moves_enough(None, 20));
        assert!(!progress_moves_enough(Some(40), 50));
        assert!(progress_moves_enough(Some(40), 60));
        assert!(progress_moves_enough(Some(60), 40));
        assert!(progress_moves_enough(Some(90), 100));
        assert!(!progress_moves_enough(Some(100), 100));
    }

    #[test]
    fn the_chat_line_is_compact() {
        assert_eq!(
            progress_line("Magnets", 60, "export print files"),
            "Magnets: ~60% (next: export print files)"
        );
        assert_eq!(progress_line("Magnets", 100, ""), "Magnets: done (100%)");
        let long = "x".repeat(200);
        assert!(progress_line(&long, 10, "y").chars().count() < 110);
    }

    #[test]
    fn next_is_one_bounded_line() {
        assert_eq!(
            one_line("  export\n print   files ", "x").unwrap(),
            "export print files"
        );
        assert!(one_line("   ", "x").is_err());
        assert!(one_line(&"a".repeat(161), "x").is_err());
    }

    #[test]
    fn estimates_ride_on_the_reply_and_never_reach_the_owner() {
        let id = Uuid::nil();
        let reply = format!(
            "Site is live.
<!--restless-progress:[{{\"work\":\"{id}\",\"percent\":60,\"next\":\"export\"}},{{\"work\":\"{id}\",\"blocked\":\"printer spec\"}}]-->
<!--restless-intent:{{}}-->"
        );
        let (clean, reported) = take_progress_markers(&reply);
        assert_eq!(
            clean,
            "Site is live.

<!--restless-intent:{}-->"
        );
        assert_eq!(reported.len(), 2);
        assert_eq!(reported[0].percent, Some(60));
        assert_eq!(reported[1].blocked.as_deref(), Some("printer spec"));
        // A broken marker is still hidden; a reply without one is untouched.
        let (clean, reported) = take_progress_markers("Done. <!--restless-progress:nope-->");
        assert_eq!((clean.as_str(), reported.len()), ("Done.", 0));
        assert_eq!(take_progress_markers("plain").0, "plain");
    }

    #[test]
    fn nothing_owed_adds_nothing_to_a_wake() {
        assert!(progress_owed_section(&[], "doctrine").is_empty());
        let section = progress_owed_section(
            &[WorkProgressOwed {
                work_id: Uuid::nil(),
                title: "Magnets".into(),
                status: WorkStatus::Blocked,
                last_percent: Some(40),
                last_state: Some("estimate".into()),
            }],
            "Record `restless work progress`.",
        );
        assert!(section.contains("restless work progress"));
        assert!(section.contains("last: ~40%"));
        assert!(section.contains("[blocked]"));
    }
}
