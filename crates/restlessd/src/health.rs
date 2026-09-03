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
//!     invariant is that **a turn which completed normally, consumed no tokens,
//!     and produced no observable activity did not happen, it failed**; that
//!     single tell catches quota, auth, a deleted model, and a mis-negotiated
//!     capability without erasing work when an ACP process exits before its
//!     final usage report.
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
    /// The provider session history no longer fits in one request. This is a
    /// responsibility-local continuity failure, not a dead model: discard the
    /// hot session and reconstruct the next wake from durable company state.
    Context,
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
            Self::Context => "context",
            Self::NoOp => "no-op",
            Self::Budget => "budget",
            Self::Transport => "transport",
        }
    }
}

/// Read the stable prefix produced by [`Blocked::message`]. This is used by
/// provider continuity after a turn has already been classified; it does not
/// re-interpret model prose.
#[must_use]
pub fn block_kind_from_message(message: &str) -> Option<BlockKind> {
    [
        BlockKind::Disk,
        BlockKind::Container,
        BlockKind::Credential,
        BlockKind::Quota,
        BlockKind::Model,
        BlockKind::Context,
        BlockKind::NoOp,
        BlockKind::Budget,
        BlockKind::Transport,
    ]
    .into_iter()
    .find(|kind| message.starts_with(&format!("[{}] ", kind.as_str())))
}

#[must_use]
pub fn is_provider_failover_kind(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Credential
            | BlockKind::Quota
            | BlockKind::Model
            | BlockKind::NoOp
            | BlockKind::Transport
    )
}

#[derive(Debug, Clone)]
pub struct Blocked {
    pub kind: BlockKind,
    pub detail: String,
}

impl Blocked {
    fn new(kind: BlockKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    /// A transport or process failure that matched no sharper class. The
    /// detail is the error chain, trimmed — unclassified is still reported
    /// honestly rather than dressed up as a decision.
    #[must_use]
    pub fn transport(detail: &str) -> Self {
        Self::new(
            BlockKind::Transport,
            format!("the agent session failed: {}", trim(detail)),
        )
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
                format!(
                    "/company is down to {} free — work would fail mid-write",
                    human_bytes(free)
                ),
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
        // ACP process termination can nevertheless surface as Completed after
        // text or tool activity but before the final UsageUpdate. Observable
        // activity makes that an interrupted, recoverable turn: unknown
        // consumption is not zero, and the durable Runtime may contain work.
        acp::TurnEnd::Completed { transcript } => match transcript.usage.map(|usage| usage.used) {
            Some(0) | None if transcript.has_observable_activity() => Verdict::Resume(
                "the agent process ended without a final usage report after observable activity; \
                 anything already written to the company computer is preserved, and the next wake \
                 continues from durable state"
                    .into(),
            ),
            Some(0) | None => Verdict::Blocked(Blocked::new(
                BlockKind::NoOp,
                "the turn ended without consuming any tokens or producing observable activity — \
                 the model never ran. Check provider credit, credential validity, and that the \
                 configured model still exists",
            )),
            Some(_) => Verdict::Ran,
        },

        // Silence is not failure. Files the agent already wrote survive on the
        // volume, but tool results and uncommitted reasoning may not. Sprint 04
        // prospecting proved that claiming *all* work was on disk was false:
        // browser evidence vanished across a cut until the task checkpointed
        // each candidate to /company. A fresh session rehydrates from durable
        // state only. Killing a turn that had been streaming reasoning for 50
        // minutes and reporting it as "the model never ran" is still the exact
        // bug this type exists to prevent.
        acp::TurnEnd::Wedged { idle, .. } => Verdict::Resume(format!(
            "the agent produced nothing for {} minutes and the turn was cut; \
             anything already written to the company computer is preserved, and the next wake \
             continues from durable state",
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
                     anything already written to the company computer is preserved, and the owner \
                     must raise the ceiling to continue"
                ),
            ))
        }

        // A material owner/lead direction is not a provider failure or no-op.
        // Preserve Runtime state and let the exact linked message become the
        // next supervised wake.
        acp::TurnEnd::Interrupted { .. } => Verdict::Resume(
            "a material direction interrupted this turn; anything already written to the company computer is preserved for supervised recovery"
                .into(),
        ),

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

    if has("413") && (has("payload too large") || has("request too large")) {
        return Some(Blocked::new(
            BlockKind::Context,
            format!(
                "the provider session history exceeded the request limit; the next wake will reconstruct from durable company state: {}",
                trim(text)
            ),
        ));
    }

    if has("402") || has("insufficient") || has("credit") || has("quota") {
        return Some(Blocked::new(
            BlockKind::Quota,
            format!(
                "provider refused on quota or credit — owner action required: {}",
                trim(text)
            ),
        ));
    }
    if has("401") || has("403") || has("invalid authentication") || has("unauthor") {
        return Some(Blocked::new(
            BlockKind::Credential,
            format!(
                "provider rejected the credential — it is missing, expired, or revoked: {}",
                trim(text)
            ),
        ));
    }
    if has("429") || has("rate limit") {
        return Some(Blocked::new(
            BlockKind::Quota,
            format!("provider rate-limited the company: {}", trim(text)),
        ));
    }
    if has("404") {
        return Some(Blocked::new(
            BlockKind::Transport,
            format!(
                "the configured provider route was not found: {}",
                trim(text)
            ),
        ));
    }
    if has("500")
        || has("502")
        || has("503")
        || has("504")
        || has("server_error")
        || has("server had an error")
    {
        return Some(Blocked::new(
            BlockKind::Transport,
            format!("the provider returned a server error: {}", trim(text)),
        ));
    }
    if has("model") && (has("not found") || has("unknown") || has("does not exist")) {
        return Some(Blocked::new(
            BlockKind::Model,
            format!(
                "the configured model is not offered by the provider: {}",
                trim(text)
            ),
        ));
    }
    None
}

/// Classify an upstream refusal that OMP delivered as assistant *content*.
///
/// Unlike [`classify_provider_error`], this entry point receives model prose,
/// not a failed transport envelope. Ordinary company language routinely says
/// things such as "unauthorised", "quota", "credit", or includes a commit
/// whose digits happen to contain an HTTP status. Those words are not
/// substrate evidence. Require the content itself to have the shape of a
/// provider error before reading its status class.
#[must_use]
pub fn classify_provider_error_content(text: &str) -> Option<Blocked> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let error = value.get("error")?;
        let encoded = if error.is_string() {
            error.as_str()?.to_string()
        } else {
            error.to_string()
        };
        return classify_provider_error(&encoded);
    }

    let first = trimmed.lines().next().unwrap_or(trimmed).trim();
    let lower = first.to_ascii_lowercase();
    let status_prefix = [
        "401", "402", "403", "404", "413", "429", "500", "502", "503", "504",
    ]
    .into_iter()
    .any(|status| {
        lower.starts_with(status)
            || lower.starts_with(&format!("http {status}"))
            || lower.starts_with(&format!("http/{status}"))
            || lower.starts_with(&format!("error {status}"))
            || lower.starts_with(&format!("error: {status}"))
            || lower.starts_with(&format!("provider error {status}"))
            || lower.starts_with(&format!("provider error: {status}"))
            || lower.starts_with(&format!("auth-gateway {status}"))
    });
    let named_error_prefix = [
        "invalid authentication",
        "authentication failed",
        "unauthorized",
        "unauthorised",
        "insufficient balance",
        "insufficient credit",
        "rate limit exceeded",
        "rate limited",
    ]
    .into_iter()
    .any(|prefix| lower.starts_with(prefix));
    let explicit_error_prefix = lower.starts_with("api error")
        || lower.starts_with("upstream error")
        || lower.starts_with("provider error")
        || lower.starts_with("error code ")
        || lower.starts_with("credit_balance_exhausted")
        || lower.starts_with("error:");

    (status_prefix || named_error_prefix || explicit_error_prefix)
        .then(|| classify_provider_error(trimmed))
        .flatten()
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
    authority: &crate::authority::AuthorityStore,
    company: &str,
    spent_usd: f64,
) -> Result<Vec<OrgSignal>> {
    use restless_orgintel::WorkStatus;
    let mut signals = Vec::new();
    let work = org.list_work().await?;

    // 1. Effort without output. Observed: two cosmon wakes burned a 20-minute
    //    boundary each and produced nothing at all.
    let completed = work
        .iter()
        .filter(|item| matches!(item.status, WorkStatus::Completed))
        .count();
    if completed == 0 && spent_usd >= EFFORT_WITHOUT_OUTPUT_USD {
        signals.push(OrgSignal {
            kind: "effort-without-output",
            detail: format!(
                "${spent_usd:.2} spent and no Work item has ever completed — \
                 is the work too big to finish, or is something quietly failing?"
            ),
        });
    }

    // 2. Repeating a failed approach. Observed: Aris asked for ~95 capability
    //    names that did not exist, in one wake, and blocked on the owner.
    let mut failures: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for event in authority.records_of_kind(company, "effect").await? {
        let Some(capability) = event
            .body
            .get("effect_class")
            .or_else(|| event.body.get("capability"))
            .and_then(|v| v.as_str())
        else {
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
    for item in work
        .iter()
        .filter(|item| matches!(item.status, WorkStatus::Blocked))
    {
        signals.push(OrgSignal {
            kind: "blocked-on-a-person",
            detail: format!(
                "\"{}\" is blocked and waiting on someone: {}",
                item.title.chars().take(60).collect::<String>(),
                item.resolution.chars().take(120).collect::<String>()
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
        transcript.usage = used.map(|used| acp::TurnUsage {
            used,
            size: 256_000,
            cost_usd: cost,
        });
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
            let end = acp::TurnEnd::Completed {
                transcript: transcript(used, None),
            };
            assert_eq!(blocked(classify(&end)).kind, BlockKind::NoOp, "{used:?}");
        }
        let end = acp::TurnEnd::Completed {
            transcript: transcript(Some(16_042), None),
        };
        assert!(matches!(classify(&end), Verdict::Ran));
    }

    /// EXP-10 killed a live ACP worker after it had edited the game and made
    /// tool calls. The adapter returned Completed without a final UsageUpdate,
    /// and the old zero-only tell erased that evidence as a provider no-op and
    /// cooled down a healthy model for fifteen minutes.
    #[test]
    fn completed_without_usage_after_activity_is_recoverable_not_a_no_op() {
        for used in [Some(0), None] {
            let mut observed = transcript(used, None);
            observed.text = "I have started the requested repair".into();
            observed.tool_calls.push("edit: main.gd".into());
            let end = acp::TurnEnd::Completed {
                transcript: observed,
            };
            let Verdict::Resume(reason) = classify(&end) else {
                panic!("observed activity with unknown usage must remain recoverable");
            };
            assert!(reason.contains("observable activity"), "{reason}");
            assert!(reason.contains("already written"), "{reason}");
            assert!(!reason.contains("never ran"), "{reason}");
        }
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
            ("HTTP 413 Payload Too Large", BlockKind::Context),
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

    #[test]
    fn provider_error_content_requires_an_error_envelope() {
        let normal_company_prose = [
            "The unauthorised action was correctly rejected and the work is complete.",
            "We stayed within the research quota and preserved credit attribution.",
            "Candidate 84ff1745b29267708599e94036ec6f7a2a7e0457 is ready.",
            "The model does not exist as a separate organisational role.",
        ];
        for prose in normal_company_prose {
            assert!(
                classify_provider_error_content(prose).is_none(),
                "ordinary model prose must not create a provider cooldown: {prose}"
            );
        }

        let provider_content = [
            (
                "429 [1113] Insufficient balance. Please recharge.",
                BlockKind::Quota,
            ),
            ("HTTP 403: credential refused", BlockKind::Credential),
            ("404 status code (no body)", BlockKind::Transport),
            ("Invalid Authentication", BlockKind::Credential),
            (
                r#"{"error":{"status":429,"message":"rate limit exceeded"}}"#,
                BlockKind::Quota,
            ),
            (
                "Error Code credit_balance_exhausted: You have no credits remaining.",
                BlockKind::Quota,
            ),
            ("auth-gateway 413: Payload Too Large", BlockKind::Context),
            (
                "500 The server had an error while processing your request (type=server_error)",
                BlockKind::Transport,
            ),
        ];
        for (content, expected) in provider_content {
            assert_eq!(
                classify_provider_error_content(content).map(|blocked| blocked.kind),
                Some(expected),
                "{content}"
            );
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
            acp::TurnEnd::OverBudget {
                transcript: nothing(),
            },
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

    /// A wedge is recoverable, not a blockage: files already written survive
    /// on the volume and the next wake rehydrates from durable state. It must
    /// not claim ephemeral tool results were checkpointed. Spending owner
    /// attention on the wedge is the expensive half of the original bug — a
    /// false kill 50 minutes into real work was reported as a company-stopping
    /// failure.
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
        assert!(reason.contains("already written"), "{reason}");
        assert!(reason.contains("durable state"), "{reason}");
    }

    /// Over-budget is the owner's call and says so with the number, rather
    /// than arriving as a generic block.
    #[test]
    fn over_budget_blocks_on_the_owner_with_the_amount() {
        let end = acp::TurnEnd::OverBudget {
            transcript: transcript(Some(500_000), Some(3.5)),
        };
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
        for (company, spent, completed) in [
            ("cosmon", 7.16, 4),
            ("aris", 1.97, 2),
            ("thymelake", 2.78, 2),
        ] {
            let fires = completed == 0 && spent >= EFFORT_WITHOUT_OUTPUT_USD;
            assert!(!fires, "{company} would have been falsely flagged");
        }
        // And it does fire on the shape it exists for: money spent, nothing done.
        let stuck = 0 == 0 && 6.0 >= EFFORT_WITHOUT_OUTPUT_USD;
        assert!(stuck);
    }
}
