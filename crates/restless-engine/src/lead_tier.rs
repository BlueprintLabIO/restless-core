//! Which team-lead turns run on the everyday model (owner decision, 4 November 2026).
//!
//! Leads used to stay on the main model for every turn (S107). Most lead turns are routine
//! coordination, and a team task cost 3.7x a direct harness (token-parity task d), so a lead's
//! routine turn now runs on the everyday model with the main model behind it. The rule, in order:
//!
//! 1. **Main** when the owner switched it off ("Leads may use the everyday model"), the company
//!    has no everyday model, or the owner gave this lead its own model on People.
//! 2. **Main** for turns that are never routine: a repair or retry (any Attempt after the first,
//!    or one carrying feedback), a review (the Work reviews another), owner-facing work (owner
//!    review required, or the owner commissioned it), the first planning of a new goal (Work
//!    someone else commissioned, with no finished prerequisite), and conversation turns that
//!    answer the owner or a member, settle an owner handoff, answer a mention, start a scheduled
//!    responsibility, or read the Exec's direction.
//! 3. **Main** once an everyday attempt failed or asked to think harder: for every later turn of
//!    the same Work, and for the lead's conversation for the rest of the day (UTC). Escalate once,
//!    then stick.
//! 4. Anything else is a routine candidate: reading a Staff result and handing off the next step,
//!    a status update, acknowledging or routing a message, re-planning after a finished step. The
//!    decision model's routine signal decides when the company has one (at least 0.8 →
//!    everyday). Without a decision, the candidate runs on the everyday model.
//!
//! Every routed lead turn is recorded in the decision log (`lead_tier`, with the reason), and an
//! escalation settles that record and leaves its own (`lead_escalation`).

use restless_orgintel::{NewDecision, OrgIntel, WorkRow};
use serde_json::{json, Value};

use crate::runtime::{AgentHarness, CompanyConfig};

pub const LEAD_TIER: &str = "lead_tier";
pub const LEAD_ESCALATION: &str = "lead_escalation";
/// A lead turn goes to the everyday model only when the decision model is at least this sure it
/// is routine, as for Staff Work (S102).
const EVERYDAY_AT: f64 = 0.8;

/// What the router knows about one lead turn before it starts.
#[derive(Debug, Clone, Copy)]
pub enum LeadTurn<'a> {
    /// A Work Attempt the lead owns.
    Work {
        work: &'a WorkRow,
        attempt_no: i32,
        /// The Work reviews other Work.
        reviews: bool,
        /// The Attempt carries feedback (a repair or a returned review).
        feedback: bool,
        /// A finished prerequisite's artifacts are inputs: a step after a finished one.
        prerequisites: bool,
    },
    /// A conversation wake.
    Conversation {
        /// The owner or a member wrote, or an owner handoff is being settled.
        owner_facing: bool,
        /// A focused mention in a document or Room.
        mention: bool,
        /// A scheduled responsibility (Opportunity) starts.
        scheduled: bool,
        /// The Exec wrote: new direction, so planning.
        from_exec: bool,
        /// The addressed internal mail, for the decision model.
        mail: &'a [String],
    },
}

/// Where a lead turn runs, and why, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeadRoute {
    pub everyday: bool,
    pub reason: &'static str,
}

impl LeadRoute {
    fn main(reason: &'static str) -> Self {
        Self {
            everyday: false,
            reason,
        }
    }
    fn everyday(reason: &'static str) -> Self {
        Self {
            everyday: true,
            reason,
        }
    }
}

/// The turns that are never routine, whatever any model says. None: a routine candidate.
pub fn never_routine(actor: &str, turn: &LeadTurn<'_>) -> Option<&'static str> {
    match *turn {
        LeadTurn::Work {
            work,
            attempt_no,
            reviews,
            feedback,
            prerequisites,
        } => {
            if attempt_no > 1 || feedback {
                Some("a repair or retry")
            } else if reviews {
                Some("a review")
            } else if work.owner_review_required || work.commissioned_by == "owner" {
                Some("owner-facing work")
            } else if work.commissioned_by != actor && !prerequisites {
                Some("first planning of a new goal")
            } else {
                None
            }
        }
        LeadTurn::Conversation {
            owner_facing,
            mention,
            scheduled,
            from_exec,
            ..
        } => {
            if owner_facing {
                Some("an owner-facing reply")
            } else if mention {
                Some("a mention")
            } else if scheduled {
                Some("a scheduled responsibility starting")
            } else if from_exec {
                Some("direction from the Exec")
            } else {
                None
            }
        }
    }
}

/// The key an escalation sticks to: the Work, or the lead's conversation for the day.
pub fn sticky_subject(actor: &str, turn: &LeadTurn<'_>) -> String {
    match turn {
        LeadTurn::Work { work, .. } => format!("work:{}", work.id),
        LeadTurn::Conversation { .. } => format!(
            "conversation:{actor}:{}",
            chrono::Utc::now().format("%Y-%m-%d")
        ),
    }
}

fn questions() -> Value {
    json!({
        "routine": {"type": "noul", "instructions": "Is this team lead's turn routine coordination that a small, fast model with tools can do as well as a large one?",
            "criteria": {"true": "Reading a finished result and handing off the next step, a status update, acknowledging or routing a message, or re-planning after a finished step when no new decision is needed.",
                         "false": "Planning a new goal, a decision or draft the owner will see, reviewing or judging the quality of work, a new trade-off or priority call, or anything ambiguous."}}
    })
}

fn state(config: &CompanyConfig, turn: &LeadTurn<'_>) -> Value {
    let bounded = |text: &str, limit: usize| text.chars().take(limit).collect::<String>();
    match turn {
        LeadTurn::Work {
            work,
            prerequisites,
            ..
        } => json!({
            "company_mission": config.mission,
            "turn": "the lead works on Work it owns",
            "work_title": work.title,
            "outcome": bounded(&work.outcome, 3000),
            "expected_artifact": work.expected_artifact,
            "commissioned_by_the_lead": work.commissioned_by == work.owner_id,
            "follows_a_finished_step": prerequisites,
        }),
        LeadTurn::Conversation { mail, .. } => json!({
            "company_mission": config.mission,
            "turn": "the lead reads messages from its team",
            "messages": bounded(&mail.join("\n"), 4000),
        }),
    }
}

/// Route one lead turn and record why under `turn_id` (unique per turn). The everyday model's
/// availability on `harness` and any cooldown are the caller's: this only says whether the turn
/// may use it.
pub async fn route(
    org: &OrgIntel,
    config: &CompanyConfig,
    actor: &str,
    harness: AgentHarness,
    turn: LeadTurn<'_>,
    turn_id: &str,
) -> LeadRoute {
    if !config.leads_may_use_everyday() {
        return LeadRoute::main("leads stay on the main model");
    }
    if config.everyday_for(harness).is_none() {
        return LeadRoute::main("no everyday model");
    }
    if crate::model_gateway::has_own_model(config, actor) {
        return LeadRoute::main("the lead has its own model");
    }
    let mut decided: Option<crate::decisions::Decision> = None;
    let route = if let Some(reason) = never_routine(actor, &turn) {
        LeadRoute::main(reason)
    } else if escalated(org, &sticky_subject(actor, &turn)).await {
        LeadRoute::main("an everyday attempt was escalated earlier")
    } else {
        match decide(config, &turn).await {
            Some(decision) => {
                let routine = decision.noul("routine").unwrap_or(0.0);
                decided = Some(decision);
                if routine >= EVERYDAY_AT {
                    LeadRoute::everyday("routine, by the decision model")
                } else {
                    LeadRoute::main("not routine, by the decision model")
                }
            }
            None => LeadRoute::everyday("routine coordination"),
        }
    };
    let mut answers = decided
        .as_ref()
        .map_or_else(|| json!({}), |decision| decision.answers.clone());
    if let Some(object) = answers.as_object_mut() {
        object.insert("reason".into(), json!(route.reason));
    }
    let model = decided
        .as_ref()
        .map_or("rule", |decision| decision.model.as_str());
    if let Err(error) = org
        .record_decision(NewDecision {
            kind: LEAD_TIER,
            subject: turn_id,
            mode: "active",
            model,
            answers: &answers,
            choice: if route.everyday { "everyday" } else { "main" },
            latency_ms: decided.as_ref().map_or(0, |decision| decision.latency_ms),
        })
        .await
    {
        tracing::warn!(actor, "could not record the lead tier: {error:#}");
    }
    route
}

async fn decide(config: &CompanyConfig, turn: &LeadTurn<'_>) -> Option<crate::decisions::Decision> {
    if !crate::decisions::enabled() {
        return None;
    }
    let client = crate::decisions::DecisionClient::for_company(config).await?;
    match client.decide(state(config, turn), questions()).await {
        Ok(decision) => Some(decision),
        Err(error) => {
            tracing::warn!(company = %config.name, "lead tier decision unavailable: {error:#}");
            None
        }
    }
}

async fn escalated(org: &OrgIntel, subject: &str) -> bool {
    org.decided_subjects(LEAD_ESCALATION, &[subject.to_string()])
        .await
        .is_ok_and(|found| !found.is_empty())
}

/// A routed everyday lead turn: what to record if it escalates.
#[derive(Debug, Clone)]
pub struct Escalation {
    /// The `lead_tier` record of this turn.
    pub turn_id: String,
    /// Where the escalation sticks.
    pub sticky_subject: String,
}

/// What the everyday model reads at the end of its turn instructions: how to ask for the main
/// model. Only the everyday model is told.
pub const ESCALATE_INSTRUCTION: &str = "\n# Model\nThis turn runs on the company's everyday model because it looks routine. If it needs more than routine coordination (a new plan, a trade-off, a quality judgement, or anything the owner will see), stop before acting and end with `<!--restless-turn:{\"decision\":\"escalate\",\"reason\":\"<why, one plain line>\"}-->`. The same turn then reruns on the main model.\n";

/// Why an everyday lead attempt should rerun on the main model, or None to keep its outcome.
/// A failed turn, an invalid ending (a malformed or missing decision, a loop, an empty reply) and
/// an explicit request escalate. Spend limits, an oversized session and provider failures do not:
/// the owner or ordinary failover handles those.
pub fn escalation_reason(
    outcome: &anyhow::Result<(crate::exec::Termination, String)>,
) -> Option<String> {
    use crate::health::{block_kind_from_message, classify_provider_error_content, BlockKind};
    let bounded = |text: &str| text.chars().take(300).collect::<String>();
    match outcome {
        Err(error) => Some(format!(
            "the turn failed: {}",
            bounded(&format!("{error:#}"))
        )),
        Ok((crate::exec::Termination::Blocked, reason)) => {
            if let Some(asked) = reason.strip_prefix(crate::exec::ESCALATE_PREFIX) {
                return Some(format!(
                    "the lead asked to think harder: {}",
                    bounded(asked)
                ));
            }
            let kind = block_kind_from_message(reason)
                .or_else(|| classify_provider_error_content(reason).map(|blocked| blocked.kind));
            match kind {
                Some(BlockKind::Budget | BlockKind::Context) => None,
                Some(kind) if crate::health::is_provider_failover_kind(kind) => None,
                _ if reason.starts_with("[budget]")
                    || reason.starts_with("[invocation_budget]") =>
                {
                    None
                }
                _ => Some(format!("the turn ended blocked: {}", bounded(reason))),
            }
        }
        Ok(_) => None,
    }
}

/// Record an escalation: it sticks to its subject, and the turn's routing record is settled.
pub async fn record_escalation(
    org: &OrgIntel,
    actor: &str,
    escalation: &Escalation,
    from: &str,
    to: &str,
    reason: &str,
) {
    let answers = json!({"reason": reason, "to": to});
    if let Err(error) = org
        .record_decision(NewDecision {
            kind: LEAD_ESCALATION,
            subject: &escalation.sticky_subject,
            mode: "active",
            model: from,
            answers: &answers,
            choice: "main",
            latency_ms: 0,
        })
        .await
    {
        tracing::warn!(actor, "could not record the escalation: {error:#}");
    }
    if let Err(error) = org
        .settle_decision_outcomes(LEAD_TIER, std::slice::from_ref(&escalation.turn_id), "escalated")
        .await
    {
        tracing::warn!(actor, "could not settle the lead tier: {error:#}");
    }
    if let Err(error) = org
        .emit_event(
            "model_escalated",
            Some(actor),
            json!({"from": from, "to": to, "reason": reason}),
        )
        .await
    {
        tracing::warn!(actor, "could not record the escalation event: {error:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::Termination;

    fn work(commissioned_by: &str, owner_review_required: bool) -> WorkRow {
        WorkRow {
            id: uuid::Uuid::new_v4(),
            goal_id: None,
            owner_id: "lead".into(),
            title: "Hand the draft to the checker".into(),
            outcome: "Next step".into(),
            status: restless_orgintel::WorkStatus::Active,
            resolution: String::new(),
            priority: 0,
            expected_artifact: String::new(),
            owner_review_required,
            producing_topology: Default::default(),
            commissioned_by: commissioned_by.into(),
            repo: None,
            base_ref: None,
            integration_branch: None,
            worktree: None,
            revision: 1,
            attempt_limit: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn attempt(work: &WorkRow) -> LeadTurn<'_> {
        LeadTurn::Work {
            work,
            attempt_no: 1,
            reviews: false,
            feedback: false,
            prerequisites: false,
        }
    }

    /// Owner, 4 Nov: owner-facing outputs and reviews never run on the everyday model, nor do
    /// repairs, first planning, or conversations with the owner, a mention, a schedule or the Exec.
    #[test]
    fn owner_facing_reviews_and_planning_stay_on_the_main_model() {
        let own_step = work("lead", false);
        assert_eq!(never_routine("lead", &attempt(&own_step)), None);

        let for_owner = work("lead", true);
        assert_eq!(
            never_routine("lead", &attempt(&for_owner)),
            Some("owner-facing work")
        );
        let from_owner = work("owner", false);
        assert_eq!(
            never_routine("lead", &attempt(&from_owner)),
            Some("owner-facing work")
        );
        let review = LeadTurn::Work {
            work: &own_step,
            attempt_no: 1,
            reviews: true,
            feedback: false,
            prerequisites: true,
        };
        assert_eq!(never_routine("lead", &review), Some("a review"));
        for (attempt_no, feedback) in [(2, false), (1, true)] {
            let repair = LeadTurn::Work {
                work: &own_step,
                attempt_no,
                reviews: false,
                feedback,
                prerequisites: true,
            };
            assert_eq!(never_routine("lead", &repair), Some("a repair or retry"));
        }
        // The Exec's commission is the lead's first planning of a new goal; a commissioned step
        // after a finished prerequisite is re-planning, a routine candidate.
        let commissioned = work("exec", false);
        assert_eq!(
            never_routine("lead", &attempt(&commissioned)),
            Some("first planning of a new goal")
        );
        let after_step = LeadTurn::Work {
            work: &commissioned,
            attempt_no: 1,
            reviews: false,
            feedback: false,
            prerequisites: true,
        };
        assert_eq!(never_routine("lead", &after_step), None);

        let mail = ["- message 1 from maya: the draft is done".to_string()];
        let conversation = |owner_facing, mention, scheduled, from_exec| LeadTurn::Conversation {
            owner_facing,
            mention,
            scheduled,
            from_exec,
            mail: &mail,
        };
        assert_eq!(
            never_routine("lead", &conversation(false, false, false, false)),
            None
        );
        for (turn, reason) in [
            (
                conversation(true, false, false, false),
                "an owner-facing reply",
            ),
            (conversation(false, true, false, false), "a mention"),
            (
                conversation(false, false, true, false),
                "a scheduled responsibility starting",
            ),
            (
                conversation(false, false, false, true),
                "direction from the Exec",
            ),
        ] {
            assert_eq!(never_routine("lead", &turn), Some(reason));
        }
    }

    #[test]
    fn failures_and_requests_escalate_but_limits_do_not() {
        let blocked = |reason: &str| -> anyhow::Result<(Termination, String)> {
            Ok((Termination::Blocked, reason.to_string()))
        };
        assert!(
            escalation_reason(&Err(anyhow::anyhow!("staff turn: tool call was invalid")))
                .unwrap()
                .starts_with("the turn failed")
        );
        assert!(escalation_reason(&blocked(&format!(
            "{}this needs a trade-off",
            crate::exec::ESCALATE_PREFIX
        )))
        .unwrap()
        .contains("asked to think harder: this needs a trade-off"));
        assert!(escalation_reason(&blocked(
            "Staff completion protocol was malformed or ambiguous"
        ))
        .unwrap()
        .starts_with("the turn ended blocked"));
        assert!(escalation_reason(&blocked("actor produced no conversation result")).is_some());
        assert_eq!(
            escalation_reason(&blocked("[budget] The company reached its spend limit.")),
            None
        );
        assert_eq!(
            escalation_reason(&Ok((Termination::OutcomeMet, "done".into()))),
            None
        );
        assert_eq!(
            escalation_reason(&Ok((Termination::Continue, "waiting".into()))),
            None
        );
    }

    #[test]
    fn escalate_is_a_turn_decision() {
        let (_, decision) = crate::exec::take_turn_marker(
            "I need more thought.\n<!--restless-turn:{\"decision\":\"escalate\",\"reason\":\"a new plan\"}-->",
        );
        let decision = decision.unwrap();
        assert_eq!(decision.termination, Termination::Blocked);
        assert_eq!(
            decision.reason,
            format!("{}a new plan", crate::exec::ESCALATE_PREFIX)
        );
    }

    #[test]
    fn the_switch_and_the_everyday_model_gate_leads() {
        let mut config: CompanyConfig =
            toml::from_str("name = \"lead_switch_test\"\nmodel = \"anthropic/claude-opus-5-5\"\n")
                .unwrap();
        assert!(config.leads_may_use_everyday());
        assert_eq!(
            config
                .everyday_for(AgentHarness::RestlessManaged)
                .as_deref(),
            Some("anthropic/claude-sonnet-5-5")
        );
        config.leads_use_everyday = Some(false);
        assert!(!config.leads_may_use_everyday());
        let saved = toml::to_string(&config).unwrap();
        assert!(saved.contains("leads_use_everyday = false"), "{saved}");
        config.set_everyday_model("off").unwrap();
        assert!(config.everyday_for(AgentHarness::RestlessManaged).is_none());
    }
}
