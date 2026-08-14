//! Deterministic substrate health (sprint 01 friction F1/F2/F3/F12).
//!
//! Every expensive failure of sprint 01 was one failure wearing different
//! masks: the system could not distinguish "the substrate broke" from "the
//! agent decided something". A dead provider key became "unparseable
//! termination" and 20 owner mails in 3h; a full disk became a silent stall;
//! stopped containers and a hung Docker became nothing at all.
//!
//! LLM_CURE.md frame 2 names the error: substrate health is deterministic and
//! enumerable — disk bytes, container state, HTTP status, tokens consumed —
//! so it must never be answered by parsing model output. This module answers
//! it by looking.
//!
//! Two gates around every wake:
//!   * `preflight` — cheap local checks before any context is assembled or
//!     any token is spent. A company that cannot write, or whose computer is
//!     not running, does not get woken.
//!   * `classify` — what actually happened, read off how the turn ended plus
//!     token consumption and transport status, never prose. The load-bearing
//!     invariant is that **a turn which completed normally and consumed no
//!     tokens did not happen, it failed**; that single tell catches quota,
//!     auth, a deleted model, and a mis-negotiated capability alike.
//!
//! `classify` is deliberately *total over [`acp::TurnEnd`]* and is the only
//! reader of it. The previous shape — a predicate over two `Option`s, called
//! from four places — put the "completed normally" precondition in the
//! caller's head instead of in the type, and three of the four callers
//! eventually dropped it. See `acp::TurnEnd` for the full account.

use anyhow::{Context, Result};

use crate::acp;
use crate::runtime;

/// Free space below which a company is blocked rather than woken. The host
/// filled twice during sprint 01; the second fill was caught only because a
/// human happened to run `df`.
const MIN_FREE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Why the substrate is not ready. Enumerable by construction — if a new
/// cause appears that does not fit, that is the signal to add a variant, not
/// to fall back to prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// The company computer cannot write.
    Disk,
    /// The container is stopped, absent, or Docker is not answering.
    Container,
    /// Provider refused the credential (401/403).
    Credential,
    /// Provider refused on quota or rate (402/429).
    Quota,
    /// The configured model is not one the provider offers.
    Model,
    /// The turn completed but consumed nothing — it did not happen.
    NoOp,
    /// The company reached its spend ceiling. Owner action, not a fault.
    Budget,
    /// Transport or process failure with no clearer class.
    Transport,
}

impl BlockKind {
    /// One line the Exec and the owner can both act on. Plain language is the
    /// product surface here; the variant is what the daemon branches on.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disk => "disk",
            Self::Container => "container",
            Self::Credential => "credential",
            Self::Quota => "quota",
            Self::Model => "model",
            Self::NoOp => "no-op",
            Self::Budget => "budget",
            Self::Transport => "transport",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Blocked {
    pub kind: BlockKind,
    pub detail: String,
}

impl Blocked {
    fn new(kind: BlockKind, detail: impl Into<String>) -> Self {
        Self { kind, detail: detail.into() }
    }

    /// A transport or process failure that matched no sharper class. The
    /// detail is the error chain, trimmed — unclassified is still reported
    /// honestly rather than dressed up as a decision.
    #[must_use]
    pub fn transport(detail: &str) -> Self {
        Self::new(BlockKind::Transport, format!("the agent session failed: {}", trim(detail)))
    }

    /// The company is out of money. Reachable two ways — before a wake, from
    /// the ledger, and mid-turn, from the agent's own usage — and both must
    /// arrive at the owner looking identical, which is why neither formats its
    /// own prefix.
    #[must_use]
    pub fn budget(detail: impl Into<String>) -> Self {
        Self::new(BlockKind::Budget, detail)
    }

    /// The owner-facing sentence. No stack traces, no JSON — this is what
    /// lands in the milestone reason and the owner's mail.
    #[must_use]
    pub fn message(&self) -> String {
        format!("[{}] {}", self.kind.as_str(), self.detail)
    }
}

/// Cheap deterministic checks before a wake spends anything. Returns the
/// first blocking condition found, or `None` when the substrate is ready.
pub async fn preflight(company: &str) -> Result<Option<Blocked>> {
    match runtime::status(company).await {
        Ok(runtime::ContainerStatus::Running) => {}
        Ok(runtime::ContainerStatus::Stopped) => {
            return Ok(Some(Blocked::new(
                BlockKind::Container,
                format!("the {company} computer is stopped; start it before working"),
            )));
        }
        Ok(runtime::ContainerStatus::Absent) => {
            return Ok(Some(Blocked::new(
                BlockKind::Container,
                format!("no container for {company}; run `restless up -c {company}`"),
            )));
        }
        Err(error) => {
            // F12: a hung Docker must be a blocked company, not a wedged
            // daemon. Report and move on rather than propagating.
            return Ok(Some(Blocked::new(
                BlockKind::Container,
                format!("cannot read container state (is Docker running?): {error}"),
            )));
        }
    }

    let root = runtime::state_root();
    if let Some(free) = free_bytes_host(&root).await? {
        if free < MIN_FREE_BYTES {
            return Ok(Some(Blocked::new(
                BlockKind::Disk,
                format!(
                    "host is down to {} free at {} — the company cannot reliably write",
                    human_bytes(free),
                    root.display()
                ),
            )));
        }
    }

    if let Some(free) = free_bytes_container(company).await? {
        if free < MIN_FREE_BYTES {
            return Ok(Some(Blocked::new(
                BlockKind::Disk,
                format!("/company is down to {} free — work would fail mid-write", human_bytes(free)),
            )));
        }
    }

    Ok(None)
}

/// What the daemon should do about a finished turn. Three outcomes, because
/// "the turn stopped early" and "the company is stuck" are different facts
/// with different costs: one resumes for free on the next wake, the other
/// spends the owner's attention.
#[derive(Debug)]
pub enum Verdict {
    /// The turn ran and consumed tokens. Ask the agent how the work stands.
    Ran,
    /// The turn stopped early, but its work is on disk and the next wake
    /// continues it. Carries the sentence to record. Costs nobody anything.
    Resume(String),
    /// The company cannot proceed without the owner.
    Blocked(Blocked),
}

/// What the turn actually did — total over every way a turn can end.
///
/// The exhaustive `match` is the guarantee. The no-op tell ("consumed nothing,
/// so it never ran") lives *inside* the `Completed` arm and is unreachable
/// from the other three, which is precisely the property the old predicate
/// could not express: on a wedge, a failure, or a budget halt the token count
/// is **unknown**, and unknown is not zero. Three separate call sites read it
/// as zero anyway and told the owner the model never ran.
#[must_use]
pub fn classify(end: &acp::TurnEnd) -> Verdict {
    match end {
        // The only arm entitled to read consumption as a verdict: the turn
        // ended by the agent's own choice, so a usage report should exist.
        acp::TurnEnd::Completed { transcript } => match transcript.usage.map(|usage| usage.used) {
            Some(0) | None => Verdict::Blocked(Blocked::new(
                BlockKind::NoOp,
                "the turn ended without consuming any tokens — the model never ran. \
                 Check provider credit, credential validity, and that the configured \
                 model still exists",
            )),
            Some(_) => Verdict::Ran,
        },

        // Silence is not failure. The agent's work up to the wedge is on the
        // volume, and a fresh session rehydrates from it. Killing a turn that
        // had been streaming reasoning for 50 minutes and reporting it as
        // "the model never ran" is the exact bug this type exists to prevent.
        acp::TurnEnd::Wedged { idle, .. } => Verdict::Resume(format!(
            "the agent produced nothing for {} minutes and the turn was cut; \
             its work so far is on disk and the next wake continues it",
            idle.as_secs() / 60
        )),

        // Owner action, and a real blockage — but not a fault, and not a
        // no-op: money was spent, which is the opposite of nothing happening.
        acp::TurnEnd::OverBudget { transcript } => {
            let spent = transcript
                .usage
                .and_then(|usage| usage.cost_usd)
                .map_or(String::new(), |cost| format!(" (${cost:.2} this turn)"));
            Verdict::Blocked(Blocked::new(
                BlockKind::Budget,
                format!(
                    "the turn reached the company's remaining spend ceiling{spent}; \
                     work so far is on disk and the owner must raise the ceiling to continue"
                ),
            ))
        }

        // A failure classifies by its status class. It never falls through to
        // the consumption check — tokens may well have been spent before the
        // stream broke, and either way we do not know.
        acp::TurnEnd::Failed { error, .. } => Verdict::Blocked(
            classify_provider_error(error).unwrap_or_else(|| Blocked::transport(error)),
        ),
    }
}

/// Deterministic status-class read of a provider failure. This is envelope
/// parsing (a status code is a structured field), not judgement over content
/// — precisely the split frame 2 asks for. F1's open remainder.
///
/// Public because it has a second, legitimate entry point: omp streams an
/// upstream error body through as message *content*, so a turn can "succeed",
/// consume tokens, and still be nothing but `429 Insufficient balance`. That
/// caller is reading agent text, not a turn end, and must say so by name —
/// routing it through `classify` would be a category error.
#[must_use]
pub fn classify_provider_error(text: &str) -> Option<Blocked> {
    let lower = text.to_lowercase();
    let has = |needle: &str| lower.contains(needle);

    if has("402") || has("insufficient") || has("credit") || has("quota") {
        return Some(Blocked::new(
            BlockKind::Quota,
            format!("provider refused on quota or credit — owner action required: {}", trim(text)),
        ));
    }
    if has("401") || has("403") || has("invalid authentication") || has("unauthor") {
        return Some(Blocked::new(
            BlockKind::Credential,
            format!("provider rejected the credential — it is missing, expired, or revoked: {}", trim(text)),
        ));
    }
    if has("429") || has("rate limit") {
        return Some(Blocked::new(
            BlockKind::Quota,
            format!("provider rate-limited the company: {}", trim(text)),
        ));
    }
    if has("model") && (has("not found") || has("unknown") || has("does not exist")) {
        return Some(Blocked::new(
            BlockKind::Model,
            format!("the configured model is not offered by the provider: {}", trim(text)),
        ));
    }
    None
}

fn trim(text: &str) -> String {
    text.chars().take(200).collect()
}

/// Money burned with nothing completed before the company is worth a look.
/// Deliberately loose: a company mid-build legitimately spends before it
/// finishes anything, and a signal that fires on normal work is noise.
const EFFORT_WITHOUT_OUTPUT_USD: f64 = 5.0;
/// How many failures of the same capability before repetition is a pattern
/// rather than a provider having a bad minute.
const REPEATED_FAILURE_THRESHOLD: usize = 3;

/// A company that is *stuck* rather than *broken*.
///
/// Everything above this line answers "is the substrate working?" — disk,
/// container, credential, tokens. Those were the only signals sprint 01 had,
/// which meant the daemon could say the disk was full but never that the
/// company was spinning. These are the organisational counterparts
/// (`docs/specs/orgintel.md` §3.3).
///
/// §3.3 is labelled **Product hypothesis**, not Core contract, so this
/// implements only the three signals that are both computable from state we
/// already keep and grounded in something sprint 01 actually did. The rest of
/// its list stays unbuilt until a run produces it.
///
/// These never block. They are surfaced to the Exec in its next wake, because
/// the actor with the context is the one who can tell "stuck" from "hard"
/// (§7.2: trigger awareness, not a universal blocker).
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrgSignal {
    pub kind: &'static str,
    pub detail: String,
}

/// Read the company's own coordination state and report what looks wrong.
pub async fn organisational(
    org: &restless_orgintel::OrgIntel,
    spent_usd: f64,
) -> Result<Vec<OrgSignal>> {
    use restless_orgintel::CommitmentState;
    let mut signals = Vec::new();
    let commitments = org.list_commitments().await?;

    // 1. Effort without output. Observed: two cosmon wakes burned a 20-minute
    //    boundary each and produced nothing at all.
    let completed = commitments
        .iter()
        .filter(|c| matches!(c.state, CommitmentState::Completed))
        .count();
    if completed == 0 && spent_usd >= EFFORT_WITHOUT_OUTPUT_USD {
        signals.push(OrgSignal {
            kind: "effort-without-output",
            detail: format!(
                "${spent_usd:.2} spent and no commitment has ever completed — \
                 is the work too big to finish, or is something quietly failing?"
            ),
        });
    }

    // 2. Repeating a failed approach. Observed: Aris asked for ~95 capability
    //    names that did not exist, in one wake, and blocked on the owner.
    let mut failures: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for event in org.events_of_kind("effect").await? {
        let Some(capability) = event.body.get("capability").and_then(|v| v.as_str()) else {
            continue;
        };
        if event.body.get("outcome").is_some_and(|outcome| {
            crate::reconcile::outcome_of(outcome) == crate::reconcile::Outcome::Failed
        }) {
            *failures.entry(capability.to_string()).or_default() += 1;
        }
    }
    for (capability, count) in failures {
        if count >= REPEATED_FAILURE_THRESHOLD {
            signals.push(OrgSignal {
                kind: "repeating-a-failed-approach",
                detail: format!(
                    "{capability} has failed {count} times — the approach is not working; \
                     read the error rather than retrying it"
                ),
            });
        }
    }

    // 3. Blocked on a person. Observed: F1 latched blocked milestones so the
    //    company stops rather than re-mailing the owner every tick — which is
    //    correct, and also means a blockage can sit unnoticed.
    for commitment in commitments.iter().filter(|c| matches!(c.state, CommitmentState::Blocked)) {
        signals.push(OrgSignal {
            kind: "blocked-on-a-person",
            detail: format!(
                "\"{}\" is blocked and waiting on someone: {}",
                commitment.title.chars().take(60).collect::<String>(),
                commitment.resolution.chars().take(120).collect::<String>()
            ),
        });
    }
    Ok(signals)
}

async fn free_bytes_host(path: &std::path::Path) -> Result<Option<u64>> {
    let out = tokio::process::Command::new("df")
        .args(["-Pk", &path.to_string_lossy()])
        .output()
        .await
        .context("spawn df")?;
    if !out.status.success() {
        return Ok(None);
    }
    Ok(parse_df_available_kb(&String::from_utf8_lossy(&out.stdout)).map(|kb| kb * 1024))
}

async fn free_bytes_container(company: &str) -> Result<Option<u64>> {
    let name = runtime::container_name(company);
    let out = tokio::process::Command::new("docker")
        .args(["exec", &name, "df", "-Pk", "/company"])
        .output()
        .await
        .context("spawn docker exec df")?;
    if !out.status.success() {
        return Ok(None);
    }
    Ok(parse_df_available_kb(&String::from_utf8_lossy(&out.stdout)).map(|kb| kb * 1024))
}

/// `df -Pk` guarantees one record per filesystem in POSIX format: the
/// available column is the fourth field of the last non-empty line.
fn parse_df_available_kb(output: &str) -> Option<u64> {
    let line = output.lines().rfind(|line| !line.trim().is_empty())?;
    line.split_whitespace().nth(3)?.parse().ok()
}

fn human_bytes(bytes: u64) -> String {
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;
    if bytes >= GIB {
        format!("{:.1}GiB", bytes as f64 / GIB as f64)
    } else {
        format!("{}MiB", bytes / MIB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A transcript reporting `used` tokens and `cost` dollars.
    fn transcript(used: Option<u64>, cost: Option<f64>) -> acp::TurnTranscript {
        let mut transcript = acp::TurnTranscript::default();
        transcript.usage =
            used.map(|used| acp::TurnUsage { used, size: 256_000, cost_usd: cost });
        transcript
    }

    fn blocked(verdict: Verdict) -> Blocked {
        match verdict {
            Verdict::Blocked(blocked) => blocked,
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    /// The load-bearing invariant. Every sprint-01 silent failure — a dead
    /// key, a deleted model, a mis-negotiated fs capability — presented as a
    /// clean turn that consumed nothing.
    #[test]
    fn a_completed_turn_that_consumed_nothing_did_not_happen() {
        for used in [Some(0), None] {
            let end = acp::TurnEnd::Completed { transcript: transcript(used, None) };
            assert_eq!(blocked(classify(&end)).kind, BlockKind::NoOp, "{used:?}");
        }
        let end = acp::TurnEnd::Completed { transcript: transcript(Some(16_042), None) };
        assert!(matches!(classify(&end), Verdict::Ran));
    }

    /// F1: the provider's error channel is deterministic. These exact shapes
    /// were observed live — 402 and 403 from OpenRouter, and omp's
    /// "401 Invalid Authentication" from the dead Moonshot key.
    #[test]
    fn provider_failures_classify_by_status_not_prose() {
        let cases = [
            ("HTTP 402 insufficient credits", BlockKind::Quota),
            ("401 Invalid Authentication", BlockKind::Credential),
            ("upstream returned 403", BlockKind::Credential),
            ("429 rate limit exceeded", BlockKind::Quota),
            ("model glm-5.1 not found", BlockKind::Model),
        ];
        for (text, expected) in cases {
            let end = acp::TurnEnd::Failed {
                error: text.to_string(),
                transcript: transcript(Some(10), None),
            };
            assert_eq!(blocked(classify(&end)).kind, expected, "{text}");
        }
    }

    /// THE regression this refactor exists for. Three ways a turn can end
    /// without the token count meaning anything, and none of them may be
    /// reported as "the model never ran" — each cost an owner a trip to a
    /// healthy credential once. The `Completed` arm is the only reader of
    /// consumption, and these prove the other three cannot reach it even with
    /// a transcript that looks exactly like a no-op.
    #[test]
    fn unknown_consumption_is_never_reported_as_a_no_op() {
        let nothing = || transcript(Some(0), None);
        let ends = [
            acp::TurnEnd::Wedged {
                idle: std::time::Duration::from_secs(8 * 60),
                transcript: nothing(),
            },
            acp::TurnEnd::OverBudget { transcript: nothing() },
            acp::TurnEnd::Failed {
                error: "connection reset by peer".to_string(),
                transcript: nothing(),
            },
        ];
        for end in ends {
            let message = match classify(&end) {
                Verdict::Ran => panic!("{end:?} must not read as a normal turn"),
                Verdict::Resume(reason) => reason,
                Verdict::Blocked(blocked) => blocked.message(),
            };
            assert!(!message.contains("never ran"), "{end:?} -> {message}");
        }
    }

    /// A wedge is recoverable, not a blockage: the work is on the volume and
    /// the next wake rehydrates from it. Spending owner attention on it is the
    /// expensive half of the bug — a false kill 50 minutes into real work got
    /// reported as a company-stopping failure.
    #[test]
    fn a_wedge_resumes_rather_than_blocking_the_owner() {
        let end = acp::TurnEnd::Wedged {
            idle: std::time::Duration::from_secs(8 * 60),
            transcript: transcript(Some(120_000), None),
        };
        let Verdict::Resume(reason) = classify(&end) else {
            panic!("a wedge must resume");
        };
        assert!(reason.contains("8 minutes"), "{reason}");
        assert!(reason.contains("on disk"), "{reason}");
    }

    /// Over-budget is the owner's call and says so with the number, rather
    /// than arriving as a generic block.
    #[test]
    fn over_budget_blocks_on_the_owner_with_the_amount() {
        let end = acp::TurnEnd::OverBudget { transcript: transcript(Some(500_000), Some(3.5)) };
        let blocked = blocked(classify(&end));
        assert_eq!(blocked.kind, BlockKind::Budget);
        assert!(blocked.message().contains("$3.50"), "{}", blocked.message());
    }

    #[test]
    fn df_available_is_the_fourth_column_of_the_last_record() {
        let output = "Filesystem 1024-blocks Used Available Capacity Mounted on\n\
                      /dev/disk3s5 482797652 398765432 56123456 88% /System/Volumes/Data\n";
        assert_eq!(parse_df_available_kb(output), Some(56_123_456));
    }
}

#[cfg(test)]
mod org_tests {
    use super::*;

    /// A health check that cries wolf on normal work gets ignored, which is
    /// worse than not having one. Every company sprint 01 ran would have been
    /// silent: cosmon spent $7.16 and completed its milestones, aris $1.97,
    /// thymelake $2.78 — all under the effort-without-output threshold, or
    /// completing work, or both.
    #[test]
    fn real_sprint_01_spend_would_not_have_fired_the_signal() {
        for (company, spent, completed) in
            [("cosmon", 7.16, 4), ("aris", 1.97, 2), ("thymelake", 2.78, 2)]
        {
            let fires = completed == 0 && spent >= EFFORT_WITHOUT_OUTPUT_USD;
            assert!(!fires, "{company} would have been falsely flagged");
        }
        // And it does fire on the shape it exists for: money spent, nothing done.
        let stuck = 0 == 0 && 6.0 >= EFFORT_WITHOUT_OUTPUT_USD;
        assert!(stuck);
    }
}
