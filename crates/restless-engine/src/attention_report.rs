//! The attention report as the daemon serves it (Sprint 120): the company's own records, read
//! by `restless_orgintel`, plus the owner's Authority decisions, which that database does not
//! hold. Read-only, no model. The kinds and what the records cannot tell apart are described in
//! `restless_orgintel::attention_report`.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use restless_orgintel::{AttentionKind, AttentionReport, Intervention, OrgIntel};

use crate::runtime::CompanyConfig;

/// How far back a report reaches when no start is given: the length of the away test.
pub const DEFAULT_DAYS: i64 = 3;

/// A day (`2026-11-16`, from midnight UTC) or an exact time (`2026-11-16T09:30:00Z`).
pub fn parse_moment(text: &str) -> Result<DateTime<Utc>, String> {
    let text = text.trim();
    if let Ok(day) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Ok(day.and_hms_opt(0, 0, 0).expect("midnight exists").and_utc());
    }
    DateTime::parse_from_rfc3339(text)
        .map(|at| at.with_timezone(&Utc))
        .map_err(|_| {
            format!("{text:?} must be a day (YYYY-MM-DD) or a time (2026-11-16T09:30:00Z)")
        })
}

/// The stretch a report covers: `from` to `to`, defaulting to the last three days up to now.
pub fn range(
    from: Option<&str>,
    to: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let to = to.map(parse_moment).transpose()?.unwrap_or(now);
    let from = from
        .map(parse_moment)
        .transpose()?
        .unwrap_or(to - Duration::days(DEFAULT_DAYS));
    if from >= to {
        return Err("the report's start must be before its end".into());
    }
    Ok((from, to))
}

pub async fn report(
    config: &CompanyConfig,
    authority: &crate::authority::AuthorityStore,
    org: &OrgIntel,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> anyhow::Result<AttentionReport> {
    // Authority approvals and account requests the owner answered themselves. Inbox items the
    // company's own database holds are read there, so they are left out here (`org` is `None`).
    let approvals = crate::attention::decision_history(config, authority, None, None, usize::MAX)
        .await?
        .decisions
        .into_iter()
        .filter(|decision| decision.by_owner)
        .map(|decision| {
            Intervention::new(
                decision.observed_at,
                AttentionKind::Approved,
                &format!("{} {}", decision.title, decision.recorded_decision),
                "approval",
            )
        })
        .collect();
    Ok(org
        .attention_report(from, to, crate::org_review::is_nudge, approvals)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_covers_the_last_three_days_unless_told() {
        let now = parse_moment("2026-11-19T08:00:00Z").unwrap();
        assert_eq!(
            range(None, None, now).unwrap(),
            (parse_moment("2026-11-16T08:00:00Z").unwrap(), now)
        );
        assert_eq!(
            range(Some("2026-11-16"), Some("2026-11-19"), now).unwrap(),
            (
                parse_moment("2026-11-16T00:00:00Z").unwrap(),
                parse_moment("2026-11-19T00:00:00Z").unwrap()
            )
        );
        assert!(range(Some("2026-11-19"), Some("2026-11-16"), now).is_err());
        assert!(range(Some("yesterday"), None, now).is_err());
    }
}
