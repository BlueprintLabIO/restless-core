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
    pub mission: String,
    pub current_plan: String,
    /// Filename + content of the most recent journal entry, if any.
    pub latest_journal: Option<String>,
    pub open_commitments: Vec<CommitmentRow>,
    pub inbox: Vec<MessageRow>,
    pub wake_reason: String,
}

/// The assembled prompt and its content digest (sha256, hex).
pub struct ContextPackage {
    pub text: String,
    pub digest: String,
}

/// Pure: same snapshot in, same package out. No clock, no randomness, no IO.
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

    let plan_exists = !snapshot.current_plan.trim().is_empty();
    let text = format!(
        "You are the Exec of {name} — the singleton chief executive of this autonomous company.\n\
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
         You may hand work to staff — at most 2 run concurrently. Ask in your termination \
         envelope with an extra field:\n\
           \"spawn\": [{{\"name\": \"builder\", \"task\": \"what and why, in detail\", \
         \"repo\": \"<name under /company/repos>\"}}]\n\
         (\"repo\" only when the task produces code: each code-producing staff member gets a \
         dedicated git worktree at /company/worktrees/<name>, branch staff/<name>, and works \
         only there.) One staff member runs one supervised task. Its completion, blockage, or \
         crash wakes you; after a crash the worktree is intact — resume or reassign is your \
         call. Staff tasks already open appear in your commitments below.\n\n\
         # Current plan [working hypothesis]\n{plan}\n\n\
         # Latest journal entry [historical memory]\n{journal}\n\n\
         # Open commitments [internal decision]\n{commitments}\
         # Inbox\n{inbox}\
         # This wake [owner directive]\n{reason}\n\n\
         Work this turn. Use the tools. Write files. Stop when the turn's work is done.",
        name = snapshot.company,
        mission = snapshot.mission,
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
