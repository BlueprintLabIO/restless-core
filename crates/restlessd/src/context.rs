//! Context assembly on wake (sprint 01 T7): a PURE function from a
//! read-only state snapshot to the Exec's provider-facing system and user
//! prompts plus a digest.
//! The digest makes the Exec's worldview reproducible — two wakes over the
//! same snapshot must see byte-identical context, and any new state (a
//! message or Work transition) must change it.
//!
//! Sources are labelled by trust (ARCHITECTURE.md §9.5): the owner mandate
//! is read-only and authoritative; the plan is the Exec's own editable
//! working hypothesis; the journal is historical memory; Work and
//! internal messages are internal decisions. No untrusted external content
//! exists this sprint — nothing ingests it yet.

use restless_orgintel::{MessageRow, OwnerHandoffRow, WorkRow};
use sha2::Digest as _;

/// Canonical standing company-agent rules shipped with this daemon build.
/// A loose state-root copy used to drift silently from the repository and
/// could even be absent, leaving live actors without their operating rules.
pub(crate) const COMPANY_OPERATING_RULES: &str =
    include_str!("../../../docs/COMPANY_OPERATING_RULES.md");

/// Read-only inputs to one wake's context. Gathering this is the only IO;
/// `assemble` itself is pure.
pub struct ContextSnapshot {
    pub company: String,
    /// Standing rules for every company actor, compiled from the canonical
    /// `docs/COMPANY_OPERATING_RULES.md`. Layer 1 of four — see `assemble`.
    pub operating_rules: String,
    pub mission: String,
    /// Authority-owned, owner-approved safe business identity projection.
    /// Restricted KYB/identity material has no representation here.
    pub legal_identity: Option<serde_json::Value>,
    pub current_plan: String,
    /// Filename + content of the most recent journal entry, if any.
    pub latest_journal: Option<String>,
    pub open_work: Vec<WorkRow>,
    pub inbox: Vec<MessageRow>,
    /// Ordinary organisational judgement currently owed by the Exec. The
    /// five irreducible human categories never appear here.
    pub owed_judgements: Vec<OwnerHandoffRow>,
    pub wake_reason: String,
    /// Remaining budget in USD, and the ceiling. An agent that cannot see its
    /// own budget cannot decide how ambitious to be, and finds out it is broke
    /// by being killed mid-turn.
    pub budget_remaining_usd: f64,
    pub budget_ceiling_usd: f64,
    /// What the kernel's receipts record this company having actually done to
    /// the world. The company's own narrative is a claim; this is the
    /// observation. Handing it back each wake is what lets the Exec notice it
    /// has reported revenue its receipts do not support.
    pub effect_ledger: String,
    /// What looks wrong with the company itself, as opposed to its computer.
    /// Advisory: the Exec is the actor with enough context to tell "stuck"
    /// from "hard", so these are shown, never enforced.
    pub org_signals: Vec<String>,
}

/// The assembled prompt pair and its content digest (sha256, hex). Standing
/// identity, policy, and trusted company state belong to `system_prompt`;
/// owner-authored input and the immediate wake belong to `user_prompt`.
pub struct ContextPackage {
    pub system_prompt: String,
    pub user_prompt: String,
    pub digest: String,
}

/// Pure: same snapshot in, same package out. No clock, no randomness, no IO.
///
/// # What belongs in an agent's context
///
/// Four layers, in decreasing stability. The rule that decides membership:
/// **carry what the agent cannot cheaply discover for itself; point at the
/// rest.** A payload costs tokens on every wake whether it is needed or not
/// and goes stale silently; a pointer costs one line and is always current.
///
/// 1. **Operating rules** — standing rules for every actor everywhere. Changes
///    rarely, changes the meaning of everything below it.
/// 2. **Mission** — owner-authored, what this company is for.
/// 3. **State** — plan, journal, Work, inbox, wake reason, budget. What
///    is true right now and nowhere else discoverable.
/// 4. **Pointers** — one line each for surfaces the agent can interrogate
///    itself. Never inline what a tool call answers better.
///
pub fn assemble(snapshot: &ContextSnapshot) -> ContextPackage {
    let mut work = String::new();
    for item in &snapshot.open_work {
        work.push_str(&format!(
            "- {} rev {} [{}] {} (owner: {}): {}\n",
            item.id,
            item.revision,
            format!("{:?}", item.status).to_lowercase(),
            item.title,
            item.owner_id,
            item.outcome
        ));
    }
    let mut inbox = String::new();
    let mut owner_input = String::new();
    for message in &snapshot.inbox {
        // Owner input is authoritative in source but not pre-classified. The
        // Exec decides whether it is conversation, Work feedback, durable
        // direction, or a request for an Authority decision.
        if message.from_actor == "owner" {
            owner_input.push_str(&format!(
                "- owner message {}: {}\n",
                message.id, message.body
            ));
        } else {
            inbox.push_str(&format!(
                "- message {} [internal decision] from {}: {}\n",
                message.id, message.from_actor, message.body
            ));
        }
    }
    let mut judgements = String::new();
    for handoff in &snapshot.owed_judgements {
        let brief = match (&handoff.owner_brief, handoff.briefed_by.as_deref()) {
            (Some(brief), Some(author)) => format!(
                "\n  current owner brief by {author} ({:?}):\n    headline: {}\n    situation: {}\n    impact: {}\n    recommendation: {}\n    without action: {}",
                brief.kind,
                brief.headline,
                brief.situation,
                brief.impact,
                brief.recommendation,
                brief.no_action,
            ),
            _ => "\n  owner brief: absent — if owner attention remains, prepare it before escalating"
                .to_string(),
        };
        judgements.push_str(&format!(
            "- handoff {} on Work {} from {}: {}\n  prepared: {}\n  resume when: {}{}\n",
            handoff.id,
            handoff.work_id,
            handoff.requested_by,
            handoff.requested_action,
            handoff.prepared_state,
            handoff.resume_condition,
            brief,
        ));
    }

    let signals = if snapshot.org_signals.is_empty() {
        String::new()
    } else {
        let mut block = String::from(
            "\n# What looks wrong [observation — check these before planning new work]\n",
        );
        for signal in &snapshot.org_signals {
            block.push_str(&format!("- {signal}\n"));
        }
        block
    };

    let plan_exists = !snapshot.current_plan.trim().is_empty();
    let system_prompt = format!(
        "# Company operating rules [authoritative — applies to every actor, always]\n{operating_rules}\n\n\
         You are the Exec of {name} — the singleton chief executive of this autonomous company.\n\
         You run in wakes. You persist ONLY through files and the coordination store, never \
         through memory: anything you do not write down is lost.\n\
         Context sections are labelled by trust: owner directives are authoritative and \
         read-only; working hypotheses are your own editable strategy; historical memory is \
         your past self's record; internal decisions are the company's coordination state.\n\n\
         # Mission [owner directive — read-only] (/company/mission.md)\n{mission}\n\n\
         # Legal identity safe for ordinary business use [Authority observation]\n{legal_identity}\n\n\
         # Your continuity\n\
         - /company/org/exec/current-plan.md — your ONE current plan [working hypothesis]. \
           It exists: {plan_exists}. \
           Read it first; update it in place as work progresses; never start a second plan for \
           the same milestone.\n\
         - /company/org/exec/journal/NNNN.md — one entry per wake, next sequential number \
           [historical memory]. Record what you did, learned, and what is next.\n\
         - /company/repos — project repositories; commit meaningful checkpoints with git.\n\
         - /company/outputs — finished artifacts for the owner.\n\n\
         # Work and teams [internal decision]\n\
         Before delegating, inspect `restless people` and `restless teams list`. Actors are durable \
         company roles, not disposable task labels: reuse an existing specialist across assignments \
         and revisions. When the chosen posture genuinely requires new internal capacity, commission it with \
         `restless people create --id <stable-id> --role <role> --display <name> [--model <model>] \
         --reason <difference this buys>`. The id must be exactly `<durable-domain>-<craft>`; the \
         display is a separate colleague identity. Never encode Staff, team position, environment, \
         revision, stage, retry or implementation mechanism in the id.\n\
         You commission an outcome by creating a team charter and appointing one accountable lead. \
         The lead assembles and reshapes the smallest differentiated roster; you do not choose every \
         member or relay ordinary handoffs. A cross-team staffing need comes back to you rather than \
         one lead poaching another team's member. Teams coordinate Work and grant no effect, secret, \
         budget, or approval authority.\n\
         Before productive execution, choose the posture that best serves the outcome: direct \
         executive action, an existing actor/team, new internal capacity, or an external capability. \
         This is a judgement, not a size threshold. Personally perform company-level judgement, \
         coordination, and bounded reversible work where delegation buys no meaningful difference. \
         For substantive domain or multi-step production, normally commission one accountable lead \
         and exact Work, then let the scheduler launch the Attempt. Do not privately implement a \
         delegated outcome inside the Exec turn, and do not merely narrate delegation: make it true \
         in the Work graph. For an owner-directed request, acknowledge the interpretation and \
         commissioned next step promptly rather than keeping the owner conversation open while \
         performing Staff work.\n\
         Delegated machine work has one form: `restless work add`. Give each node a stable outcome, \
         existing owner role/model, expected artifact and exact workspace. Declare its initial \
         dependencies in that same command with repeatable `--requires <prerequisite-work-id>` and \
         `--revises <producer-work-id>` flags; they commit atomically so the scheduler cannot claim a \
         half-built node. `work edge` is for a later graph repair. For requires, `--from` is the \
         prerequisite and `--to` is the dependent; revises runs reviewer to producer. Remove a \
         mistaken edge with `--remove --as <actor> --reason <evidence>`. The \
         scheduler starts ready nodes itself. Messages and process commands never own work.\n\
         Producers link exact outputs with restless work artifact; deterministic checks belong \
         in restless work gate. A review result of changes_requested invalidates its producer \
         and hard descendants into a new revision. Conversations stay free-form; the graph owns \
         kickoff, handover and input versions. A failed Attempt or rejected review stays blocked: \
         the accountable lead changes the smallest failed mechanism and records it with \
         `restless work resume --work <id> --reason <what changed>` before another Attempt starts.\n\
         restless work handoff is only for identity, CAPTCHA, MFA, legal attestation, payment \
         confirmation, or irreducible owner judgement. Preserve the prepared browser state and \
         name an observable resume condition. Ordinary failure is not an owner browser task.\n\
         # Sourcing a missing capability [shared skill]\n{sourcing}\n\
         # Presenting to the owner [shared skill]\n{owner_briefing}\n\n\
         # Affecting the world [internal decision]\n\
         Use installed Linux tools directly for reversible work. Wrap material external argv with \
         restless effect --class <class> --purpose <why> [--party <party>] \
         [--artifact <path-or-url>] [--secret ENV=<binding>] --key <key> -- \
         <program> <args...>. Restless does not own an email or Git API: it gates \
         the ordinary process, injects named secrets only into that child, and records generic JSON. \
         Probe tools with their own help, commands, doctor, or dry-run support. A _test company \
         must use a fake CLI and cannot receive live secret bindings.\n\n\
         # What your receipts actually record [observation — stronger than your own notes]\n\
         {ledger}\n\
         These are counted from kernel receipts, not from your journal. If your plan or \
         journal claims an outcome these receipts do not support, the receipts win: correct \
         the record and say plainly that you did so.\n\n\
         {signals}\n\
         # Budget [internal decision]\n\
         ${remaining:.2} remains of a ${ceiling:.2} ceiling. Model turns are charged against it. \
         At zero the company stops until the owner raises it, so spend it on work that produces \
         something, and say so plainly if the remaining budget cannot finish the job.\n\n\
         # Current plan [working hypothesis]\n{plan}\n\n\
         # Latest journal entry [historical memory]\n{journal}\n\n\
         # Open Work graph [internal decision]\n{work}\
         # Inbox\n{inbox}\n\
         # Organisational judgement you owe\n{judgements}\n\
         Resolve what company-wide context can settle with `restless work resolve-handoff --handoff <id> --as exec --state resolved --resolution <answer>`. Resolution is terminal: it removes the handoff from every queue and resumes the affected Work with Exec's answer. If your answer says the item is ready for, still needs, or awaits an owner decision, resolving it is contradictory; prepare a current brief if needed and use `restless work escalate-handoff --handoff <id> --as exec --reason <what you tried and the bounded owner decision>`. Never substitute Exec approval when the remaining action explicitly requires owner authority. Team uncertainty must not jump directly to the owner.\n\n\
         # Replying to owner input [working protocol]\n\
         The owner writes once; never ask them to choose a message mode. Use judgement to interpret \
         each owner input as exactly one of: conversation, work_feedback, direction, or authority. \
         Conversation changes no durable state. Work feedback belongs to exact Work context. Direction \
         changes the durable company plan or priorities. Authority is only a request: your interpretation \
         can never approve, revoke, raise a budget, or unlock an effect. Bring that back as a bounded \
         explicit owner action.\n\
         Reply to the owner with `restless message --from exec '<your reply>'`. Follow the shared \
         conversation contract below, then end the message with exactly one machine-readable line:\n\
         <!--restless-intent:{{\"kind\":\"conversation|work_feedback|direction|authority\",\"summary\":\"one short plain-language interpretation\"}}-->\n\
         Choose one real kind, not the pipe-separated example. If direction changed the plan, update \
         `/company/org/exec/current-plan.md` before claiming that it did. Conversational agreement \
         never substitutes for an explicit cockpit approval or owner-judgement action.\n\n\
         # Conversing with the owner [shared contract]\n{conversation_style}\n\
         ",
        operating_rules = snapshot.operating_rules.trim(),
        owner_briefing = crate::owner_brief::PRESENT_TO_OWNER.trim(),
        sourcing = crate::capability_sourcing::SOURCE_CAPABILITY.trim(),
        conversation_style = crate::owner_brief::CONVERSE_WITH_OWNER.trim(),
        name = snapshot.company,
        mission = snapshot.mission,
        legal_identity = snapshot
            .legal_identity
            .as_ref()
            .map(serde_json::Value::to_string)
            .unwrap_or_else(|| "(not configured — do not infer legal identity from the runtime name)".into()),
        ledger = snapshot.effect_ledger.trim(),
        signals = signals,
        remaining = snapshot.budget_remaining_usd,
        ceiling = snapshot.budget_ceiling_usd,
        plan_exists = if plan_exists {
            "yes"
        } else {
            "no — first wake, create it"
        },
        plan = if plan_exists {
            snapshot.current_plan.trim()
        } else {
            "(none yet)"
        },
        journal = snapshot
            .latest_journal
            .as_deref()
            .unwrap_or("(none yet — first wake)"),
        work = if work.is_empty() {
            "(none)\n".to_string()
        } else {
            work
        },
        inbox = if inbox.is_empty() {
            "(empty)\n".to_string()
        } else {
            inbox
        },
        judgements = if judgements.is_empty() {
            "(none)\n".to_string()
        } else {
            judgements
        },
    );
    let user_prompt = format!(
        "# This wake\n{}\n\n# Owner input [authoritative in source; classify before applying]\n{}\n\
         Work this turn using the actor contract and current company state in your system context. \
         If the owner wrote, interpret and reply through the stated conversation contract. Stop when \
         this Exec wake's coordination or bounded executive work is done.",
        snapshot.wake_reason.trim(),
        if owner_input.is_empty() {
            "(none)"
        } else {
            owner_input.trim_end()
        },
    );
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"system\0");
    hasher.update(system_prompt.as_bytes());
    hasher.update(b"\0user\0");
    hasher.update(user_prompt.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    ContextPackage {
        system_prompt,
        user_prompt,
        digest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use restless_orgintel::{
        OwnerBrief, OwnerBriefKind, OwnerHandoffCategory, OwnerHandoffRow, OwnerHandoffState,
        WorkStatus,
    };

    fn snapshot() -> ContextSnapshot {
        ContextSnapshot {
            company: "probe".into(),
            operating_rules: "1. Claims are not observations.".into(),
            mission: "make the thing".into(),
            legal_identity: None,
            current_plan: "# plan\nstep 1".into(),
            latest_journal: Some("== 0001.md ==\ndid step 0".into()),
            open_work: vec![WorkRow {
                id: uuid::Uuid::nil(),
                goal_id: None,
                owner_id: "exec".into(),
                title: "milestone: probe".into(),
                outcome: "make the thing".into(),
                status: WorkStatus::Active,
                resolution: String::new(),
                priority: 0,
                expected_artifact: String::new(),
                repo: None,
                base_ref: None,
                integration_branch: None,
                worktree: None,
                revision: 1,
                attempt_limit: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }],
            inbox: vec![],
            owed_judgements: vec![],
            wake_reason: "owner-requested wake".into(),
            budget_remaining_usd: 7.5,
            budget_ceiling_usd: 10.0,
            effect_ledger: "customer-contact.email 3 · GBP 27.00 moved".into(),
            org_signals: vec!["\"ship the thing\" is blocked and waiting on someone".into()],
        }
    }

    #[test]
    fn same_snapshot_same_digest_and_new_mail_changes_it() {
        let first = assemble(&snapshot());
        let second = assemble(&snapshot());
        assert_eq!(
            first.digest, second.digest,
            "assembly must be deterministic"
        );
        assert_eq!(first.system_prompt, second.system_prompt);
        assert_eq!(first.user_prompt, second.user_prompt);

        let mut with_mail = snapshot();
        with_mail.inbox.push(MessageRow {
            id: 1,
            from_actor: "owner".into(),
            to_actor: Some("exec".into()),
            body: "prioritise the red one".into(),
            created_at: chrono::Utc::now(),
            read_at: None,
        });
        let third = assemble(&with_mail);
        assert_ne!(
            first.digest, third.digest,
            "new state must change the digest"
        );
        assert!(third.user_prompt.contains("prioritise the red one"));
        assert!(third.user_prompt.contains("owner message 1"));
        assert!(!third.system_prompt.contains("prioritise the red one"));
        assert!(third.system_prompt.contains("You are the Exec of probe"));
        assert!(!third.user_prompt.contains("You are the Exec of probe"));
    }

    #[test]
    fn exec_sees_authored_meaning_and_terminal_resolution_semantics() {
        let mut with_judgement = snapshot();
        with_judgement.owed_judgements.push(OwnerHandoffRow {
            id: uuid::Uuid::nil(),
            work_id: uuid::Uuid::nil(),
            attempt_id: None,
            requested_by: "offer-lead".into(),
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "Approve the prepared launch".into(),
            prepared_state: "The candidate is independently accepted.".into(),
            resume_condition: "The owner records a decision.".into(),
            state: OwnerHandoffState::Pending,
            resolution: String::new(),
            assigned_to: Some("exec".into()),
            escalated_from: Some("offer-lead".into()),
            escalated_at: Some(chrono::Utc::now()),
            owner_brief: Some(OwnerBrief {
                kind: OwnerBriefKind::OutcomeReview,
                headline: "The centre offer is ready to publish".into(),
                situation: "Independent review is complete.".into(),
                impact: "Approval releases the prepared launch.".into(),
                recommendation: "Approve the release.".into(),
                no_action: "The current site remains live.".into(),
                uncertainty: None,
                deadline: None,
            }),
            briefed_by: Some("offer-lead".into()),
            briefed_at: Some(chrono::Utc::now()),
            brief_source_fingerprint: Some("current".into()),
            created_at: chrono::Utc::now(),
            resolved_at: None,
        });

        let package = assemble(&with_judgement);
        assert!(package
            .system_prompt
            .contains("current owner brief by offer-lead (OutcomeReview)"));
        assert!(package
            .system_prompt
            .contains("headline: The centre offer is ready to publish"));
        assert!(package.system_prompt.contains(
            "If your answer says the item is ready for, still needs, or awaits an owner decision"
        ));
        assert!(package
            .system_prompt
            .contains("Never substitute Exec approval when the remaining action explicitly requires owner authority"));
    }

    #[test]
    fn missing_capability_guidance_keeps_sourcing_in_ordinary_work() {
        let package = assemble(&snapshot());
        assert!(package.system_prompt.contains("buy an input, rent a tool"));
        assert!(package
            .system_prompt
            .contains("A provider or counterparty is not an OrgIntel Actor"));
        assert!(package.system_prompt.contains("Use ordinary Work"));
        assert!(package.system_prompt.contains("`requires`/`revises` edges"));
        assert!(package.system_prompt.contains("The decision"));
        assert!(package.system_prompt.contains("grants no permission"));
        assert!(!package
            .system_prompt
            .contains("Only when a genuinely different capability is missing, commission it"));
    }

    #[test]
    fn exec_system_contract_requires_real_delegation_for_staff_work() {
        let package = assemble(&snapshot());
        assert!(package
            .system_prompt
            .contains("choose the posture that best serves the outcome"));
        assert!(package
            .system_prompt
            .contains("normally commission one accountable lead"));
        assert!(package
            .system_prompt
            .contains("do not merely narrate delegation: make it true in the Work graph"));
    }
}
