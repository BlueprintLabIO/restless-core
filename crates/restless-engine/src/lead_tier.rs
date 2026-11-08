//! Which team-lead turns run on the everyday model (owner decision, 4 November 2026).
//!
//! Leads used to stay on the main model for every turn (S107). Most lead turns are routine
//! coordination, and a team task cost 3.7x a direct harness (token-parity task d), so a lead's
//! routine turn now runs on the everyday model with the main model behind it. The rule, in order:
//!
//! 1. **Main** when the owner switched it off ("Leads may use the everyday model"), the company
//!    has no everyday model, or the owner gave this lead its own model on People.
//! 2. **Main** for turns that are never routine: a repair or retry (a later Attempt whose
//!    predecessor failed, had changes requested or was abandoned, or the first Attempt of a revised
//!    Work), a review (the Work reviews another), owner-facing work (owner review required, the
//!    owner commissioned it, or a person wrote new feedback on it), the first planning of a new
//!    goal (the first Attempt of Work someone else commissioned, with no finished prerequisite),
//!    and conversation turns that answer the owner or a member, settle an owner handoff, answer a
//!    mention, start a scheduled responsibility, or read the Exec's direction.
//!
//!    A lead's Work blocks while its Staff work and resumes in a new Attempt when their results
//!    are in ("dependency graph became ready"). That resume is the routine step: reading the
//!    results and handing off the next one. The Work's feedback thread (its commissioning message
//!    and the Staff notes on it) rides on every Attempt, so only feedback new to this Attempt
//!    counts. Until 8 November any later Attempt or any feedback counted as a repair, and every
//!    resume of Exec-commissioned Work as first planning: no lead Work turn was routine.
//! 3. **Main** once an everyday attempt failed or asked to think harder: for every later turn of
//!    the same Work, and for the lead's conversation for the rest of the day (UTC). Escalate once,
//!    then stick.
//! 4. Anything else is a routine candidate: reading a Staff result and handing off the next step,
//!    a status update, acknowledging or routing a message, re-planning after a finished step. The
//!    decision model may keep a candidate on the main model when it leans not routine (below 0.5).
//!    Its routine signal on candidates sat between 0.28 and 0.68 in production (Restless, 5 Nov),
//!    so the earlier "at least 0.8" bar sent every candidate to the main model. Without a
//!    decision, the candidate runs on the everyday model. Escalation (3) is the quality net.
//! 5. A lead the owner put on another connection of the same provider with the company's main
//!    model (to spread a subscription's limits, as Reno's lead is) keeps that connection and runs
//!    its everyday turns there. A different model chosen for the lead is exact (1).
//!
//! Every lead turn is recorded in the decision log (`lead_tier`, with the reason), and an
//! escalation settles that record and leaves its own (`lead_escalation`).

use restless_orgintel::{ClaimedWork, NewDecision, OrgIntel, WorkAttemptState, WorkRow};
use serde_json::{json, Value};

use crate::runtime::{AgentHarness, CompanyConfig};

pub const LEAD_TIER: &str = "lead_tier";
pub const LEAD_ESCALATION: &str = "lead_escalation";
/// A routine candidate stays on the main model when the decision model's routine signal is below
/// this: it leans not routine. The rule has already removed every turn that is never routine.
pub const MAIN_BELOW: f64 = 0.5;

/// What the router knows about one lead turn before it starts.
#[derive(Debug, Clone, Copy)]
pub enum LeadTurn<'a> {
    /// A Work Attempt the lead owns.
    Work {
        work: &'a WorkRow,
        attempt_no: i32,
        /// How the previous Attempt of this revision ended, if there was one.
        previous: Option<WorkAttemptState>,
        /// The Work reviews other Work.
        reviews: bool,
        /// Feedback no earlier Attempt received.
        fresh_feedback: bool,
        /// Some of that new feedback is from a person (the owner or a member).
        person_feedback: bool,
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
            previous,
            reviews,
            person_feedback,
            prerequisites,
            ..
        } => {
            // A lead resumes its Work after waiting on Staff (Blocked) or after a step it
            // produced; anything else before it failed or was sent back.
            let resumes = matches!(
                previous,
                Some(WorkAttemptState::Blocked | WorkAttemptState::Produced)
            );
            if (attempt_no > 1 && !resumes) || (attempt_no == 1 && work.revision > 1) {
                Some("a repair or retry")
            } else if reviews {
                Some("a review")
            } else if work.owner_review_required
                || work.commissioned_by == "owner"
                || person_feedback
            {
                Some("owner-facing work")
            } else if attempt_no == 1 && work.commissioned_by != actor && !prerequisites {
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

/// What a claimed Attempt says about its turn, owned so a [`LeadTurn::Work`] can borrow it.
#[derive(Debug, Clone, Copy)]
pub struct WorkFacts {
    pub fresh_feedback: bool,
    pub person_feedback: bool,
}

impl WorkFacts {
    /// Which of the Attempt's feedback is new, and whether a person wrote any of it.
    pub async fn of(org: &OrgIntel, claimed: &ClaimedWork) -> Self {
        let mut person_feedback = false;
        for message in claimed
            .feedback
            .iter()
            .filter(|message| claimed.fresh_feedback_ids.contains(&message.id))
        {
            // An unknown sender counts as a person: the safe side is the main model.
            if org
                .is_human_actor(&message.from_actor)
                .await
                .unwrap_or(true)
            {
                person_feedback = true;
                break;
            }
        }
        Self {
            fresh_feedback: !claimed.fresh_feedback_ids.is_empty(),
            person_feedback,
        }
    }

    pub fn turn<'a>(&self, claimed: &'a ClaimedWork) -> LeadTurn<'a> {
        LeadTurn::Work {
            work: &claimed.work,
            attempt_no: claimed.attempt_no,
            previous: claimed.previous_attempt_state,
            reviews: !claimed.review_targets.is_empty(),
            fresh_feedback: self.fresh_feedback,
            person_feedback: self.person_feedback,
            prerequisites: !claimed.inputs.is_empty(),
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
            attempt_no,
            previous,
            fresh_feedback,
            prerequisites,
            ..
        } => json!({
            "company_mission": config.mission,
            "turn": if *attempt_no > 1 {
                "the lead resumes Work it owns now that the step it waited on is done"
            } else {
                "the lead works on Work it owns"
            },
            "work_title": work.title,
            "outcome": bounded(&work.outcome, 3000),
            "expected_artifact": work.expected_artifact,
            "commissioned_by_the_lead": work.commissioned_by == work.owner_id,
            "follows_a_finished_step": prerequisites,
            "previous_attempt": previous.map(WorkAttemptState::as_str),
            "new_team_messages": fresh_feedback,
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
    let mut decided: Option<crate::decisions::Decision> = None;
    // The early answers are recorded too, so the decision log says why a lead never used the
    // everyday model (Reno, 5-8 Nov: its lead's own connection left no record at all).
    let route = if !config.leads_may_use_everyday() {
        LeadRoute::main("leads stay on the main model")
    } else if config.everyday_for(harness).is_none() {
        LeadRoute::main("no everyday model")
    } else if crate::model_gateway::pins_own_model(config, actor) {
        LeadRoute::main("the lead has its own model")
    } else if let Some(reason) = never_routine(actor, &turn) {
        LeadRoute::main(reason)
    } else if escalated(org, &sticky_subject(actor, &turn)).await {
        LeadRoute::main("an everyday attempt was escalated earlier")
    } else {
        match decide(config, &turn).await {
            Some(decision) => {
                let routine = decision.noul("routine").unwrap_or(0.0);
                decided = Some(decision);
                if routine < MAIN_BELOW {
                    LeadRoute::main("not routine, by the decision model")
                } else {
                    LeadRoute::everyday("routine, by the decision model")
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
        .settle_decision_outcomes(
            LEAD_TIER,
            std::slice::from_ref(&escalation.turn_id),
            "escalated",
        )
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
            previous: None,
            reviews: false,
            fresh_feedback: false,
            person_feedback: false,
            prerequisites: false,
        }
    }

    fn resumed(work: &WorkRow, previous: WorkAttemptState) -> LeadTurn<'_> {
        LeadTurn::Work {
            work,
            attempt_no: 3,
            previous: Some(previous),
            reviews: false,
            fresh_feedback: true,
            person_feedback: false,
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
            previous: None,
            reviews: true,
            fresh_feedback: false,
            person_feedback: false,
            prerequisites: true,
        };
        assert_eq!(never_routine("lead", &review), Some("a review"));
        for previous in [
            WorkAttemptState::Failed,
            WorkAttemptState::ChangesRequested,
            WorkAttemptState::Abandoned,
        ] {
            assert_eq!(
                never_routine("lead", &resumed(&own_step, previous)),
                Some("a repair or retry")
            );
        }
        let mut revised = work("lead", false);
        revised.revision = 2;
        assert_eq!(
            never_routine("lead", &attempt(&revised)),
            Some("a repair or retry")
        );
        // A person's new note on the Work is owner-facing.
        let noted = LeadTurn::Work {
            work: &own_step,
            attempt_no: 2,
            previous: Some(WorkAttemptState::Blocked),
            reviews: false,
            fresh_feedback: true,
            person_feedback: true,
            prerequisites: false,
        };
        assert_eq!(never_routine("lead", &noted), Some("owner-facing work"));
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
            previous: None,
            reviews: false,
            fresh_feedback: false,
            person_feedback: false,
            prerequisites: true,
        };
        assert_eq!(never_routine("lead", &after_step), None);
        // Reno, 5 Nov: the lead's Exec-commissioned Work blocked on its Staff and resumed as
        // Attempts 2-5, each carrying the Work's commissioning thread. Those resumes are the
        // routine step, not repairs and not first planning.
        for previous in [WorkAttemptState::Blocked, WorkAttemptState::Produced] {
            assert_eq!(
                never_routine("lead", &resumed(&commissioned, previous)),
                None
            );
        }

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
