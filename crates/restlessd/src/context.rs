//! Context assembly on wake (sprint 01 T7): a PURE function from a
//! read-only state snapshot to the Exec's rehydration prompt plus a digest.
//! The digest makes the Exec's worldview reproducible — two wakes over the
//! same snapshot must see byte-identical context, and any new state (a
//! message, a commitment transition) must change it.
//!
//! Sources are labelled by trust (ARCHITECTURE.md §9.5): the owner mandate
//! is read-only and authoritative; the plan is the Exec's own editable
//! working hypothesis; the journal is historical memory; commitments and
//! internal messages are internal decisions. No untrusted external content
//! exists this sprint — nothing ingests it yet.

use restless_orgintel::{CommitmentRow, MessageRow};
use sha2::Digest as _;

/// Read-only inputs to one wake's context. Gathering this is the only IO;
/// `assemble` itself is pure.
pub struct ContextSnapshot {
    pub company: String,
    /// Standing rules for every agent (docs/CONSTITUTION.md, installed at
    /// `$RESTLESS_HOME/constitution.md`). Layer 1 of four — see `assemble`.
    pub constitution: String,
    pub mission: String,
    pub current_plan: String,
    /// Filename + content of the most recent journal entry, if any.
    pub latest_journal: Option<String>,
    pub open_commitments: Vec<CommitmentRow>,
    pub inbox: Vec<MessageRow>,
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
    /// Capabilities this company's effect surface actually offers. Small and
    /// enumerable, so it is carried rather than pointed at — one company burned
    /// 57 tool calls guessing ~95 names against a surface of three.
    pub capabilities: Vec<String>,
}

/// The assembled prompt and its content digest (sha256, hex).
pub struct ContextPackage {
    pub text: String,
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
/// 1. **Constitution** — standing rules for every agent everywhere. Changes
///    rarely, changes the meaning of everything below it.
/// 2. **Mission** — owner-authored, what this company is for.
/// 3. **State** — plan, journal, commitments, inbox, wake reason, budget. What
///    is true right now and nowhere else discoverable.
/// 4. **Pointers** — one line each for surfaces the agent can interrogate
///    itself. Never inline what a tool call answers better.
///
/// The exception that proves the rule is `capabilities`: it is a payload
/// rather than a pointer only because it is tiny, enumerable, and its absence
/// caused an agent to brute-force ~95 guesses. When a surface grows past
/// enumerable, it becomes a pointer.
pub fn assemble(snapshot: &ContextSnapshot) -> ContextPackage {
    let mut commitments = String::new();
    for c in &snapshot.open_commitments {
        commitments.push_str(&format!(
            "- [{}] {} (owner: {}): {}\n",
            format!("{:?}", c.state).to_lowercase(),
            c.title,
            c.owner_id,
            c.body
        ));
    }
    let mut inbox = String::new();
    for message in &snapshot.inbox {
        // Owner messages carry owner authority; anything else is internal.
        let trust = if message.from_actor == "owner" {
            "owner directive"
        } else {
            "internal decision"
        };
        inbox.push_str(&format!("- [{trust}] from {}: {}\n", message.from_actor, message.body));
    }

    let capabilities = if snapshot.capabilities.is_empty() {
        "(none configured — any external effect will fail until the owner adds one)".to_string()
    } else {
        snapshot.capabilities.join(", ")
    };

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
    let text = format!(
        "# Constitution [authoritative — applies to every agent, always]\n{constitution}\n\n\
         You are the Exec of {name} — the singleton chief executive of this autonomous company.\n\
         You run in wakes. You persist ONLY through files and the coordination store, never \
         through memory: anything you do not write down is lost.\n\
         Context sections are labelled by trust: owner directives are authoritative and \
         read-only; working hypotheses are your own editable strategy; historical memory is \
         your past self's record; internal decisions are the company's coordination state.\n\n\
         # Mission [owner directive — read-only] (/company/mission.md)\n{mission}\n\n\
         # Your continuity\n\
         - /company/org/exec/current-plan.md — your ONE current plan [working hypothesis]. \
           It exists: {plan_exists}. \
           Read it first; update it in place as work progresses; never start a second plan for \
           the same milestone.\n\
         - /company/org/exec/journal/NNNN.md — one entry per wake, next sequential number \
           [historical memory]. Record what you did, learned, and what is next.\n\
         - /company/repos — project repositories; commit meaningful checkpoints with git.\n\
         - /company/outputs — finished artifacts for the owner.\n\n\
         # Staff [internal decision]\n\
         `restless spawn --name <name> [--repo <name under /company/repos>] \"<task>\"` hands one \
         task to one supervised staff member. Call it the moment you decide to delegate — it is a \
         tool like any other, and the reply tells you immediately if it was refused. At most 2 run \
         at once.\n\
         Give a brief detailed enough to work unsupervised: the outcome, the constraints, and how \
         you will know it is done. A code task with `--repo` gets its own git worktree at \
         /company/worktrees/<name> on branch staff/<name>, and works only there — so two staff on \
         disjoint files merge cleanly. Their completion, blockage, or crash wakes you; after a \
         crash the worktree is intact, and resuming or reassigning is your call. Staff tasks \
         already open appear in your commitments below.\n\n\
         # Affecting the world [internal decision]\n\
         `restless effect --key <idempotency-key> --args '<json>' <capability>` is the ONLY way to \
         reach outside this company. Capabilities available to you: {capabilities}. Asking for one \
         that does not exist tells you what does — you never have to guess. Every effect returns a \
         receipt; a repeated key returns the stored receipt instead of acting twice.\n\n\
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
         # Open commitments [internal decision]\n{commitments}\
         # Inbox\n{inbox}\
         # This wake [owner directive]\n{reason}\n\n\
         Work this turn. Use the tools. Write files. Stop when the turn's work is done.",
        constitution = snapshot.constitution.trim(),
        name = snapshot.company,
        mission = snapshot.mission,
        capabilities = capabilities,
        ledger = snapshot.effect_ledger.trim(),
        signals = signals,
        remaining = snapshot.budget_remaining_usd,
        ceiling = snapshot.budget_ceiling_usd,
        plan_exists = if plan_exists { "yes" } else { "no — first wake, create it" },
        plan = if plan_exists { snapshot.current_plan.trim() } else { "(none yet)" },
        journal = snapshot.latest_journal.as_deref().unwrap_or("(none yet — first wake)"),
        commitments = if commitments.is_empty() { "(none)\n".to_string() } else { commitments },
        inbox = if inbox.is_empty() { "(empty)\n".to_string() } else { inbox },
        reason = snapshot.wake_reason,
    );
    let digest = format!("{:x}", sha2::Sha256::digest(text.as_bytes()));
    ContextPackage { text, digest }
}

#[cfg(test)]
mod tests {
    use super::*;
    use restless_orgintel::CommitmentState;

    fn snapshot() -> ContextSnapshot {
        ContextSnapshot {
            company: "probe".into(),
            constitution: "1. Claims are not observations.".into(),
            mission: "make the thing".into(),
            current_plan: "# plan\nstep 1".into(),
            latest_journal: Some("== 0001.md ==\ndid step 0".into()),
            open_commitments: vec![CommitmentRow {
                id: uuid::Uuid::nil(),
                goal_id: None,
                owner_id: "exec".into(),
                title: "milestone: probe".into(),
                body: "make the thing".into(),
                state: CommitmentState::Active,
                resolution: String::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }],
            inbox: vec![],
            wake_reason: "owner-requested wake".into(),
            budget_remaining_usd: 7.5,
            budget_ceiling_usd: 10.0,
            capabilities: vec!["email.send".into(), "web.deploy".into()],
            effect_ledger: "email.send 3 · GBP 27.00 moved".into(),
            org_signals: vec!["\"ship the thing\" is blocked and waiting on someone".into()],
        }
    }

    #[test]
    fn same_snapshot_same_digest_and_new_mail_changes_it() {
        let first = assemble(&snapshot());
        let second = assemble(&snapshot());
        assert_eq!(first.digest, second.digest, "assembly must be deterministic");
        assert_eq!(first.text, second.text);

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
        assert_ne!(first.digest, third.digest, "new state must change the digest");
        assert!(third.text.contains("prioritise the red one"));
        assert!(third.text.contains("[owner directive] from owner"));
    }
}
