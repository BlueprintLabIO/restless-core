/// Ordinary conversation is a different surface from Attention: the owner
/// should receive the answer, not the actor's operating transcript.
pub const CONVERSE_WITH_OWNER: &str = include_str!("../prompts/converse-with-owner.md");
/// How Exec meets a new owner: a short interview, then a read-back that becomes the charter.
pub const WELCOME_THE_OWNER: &str = include_str!("../prompts/welcome-the-owner.md");

/// What a creator owes at a checkpoint: one honest progress estimate per Work it created. Sent only
/// in a wake where estimates are owed, so a quiet wake carries none of it.
pub const ESTIMATE_PROGRESS: &str = include_str!("../prompts/estimate-progress.md");

/// Company doctrine that applies only to some turns lives in built-in skills, shipped in the
/// company image at `/opt/restless/skills` (the Dockerfile copies `crates/restless-engine/skills`).
/// Every model call resends the fixed prompt, so ~22 KB of situational doctrine there was paid on
/// every call of every wake; a skill costs one `restless skill use` on the turn that needs it.
/// Name and full `SKILL.md`, for checks that read what agents can be told.
pub const DOCTRINE_SKILLS: [(&str, &str); 7] = [
    (
        "writing-for-the-owner",
        include_str!("../skills/writing-for-the-owner/SKILL.md"),
    ),
    (
        "owner-briefs",
        include_str!("../skills/owner-briefs/SKILL.md"),
    ),
    (
        "running-the-business",
        include_str!("../skills/running-the-business/SKILL.md"),
    ),
    (
        "align-and-learn",
        include_str!("../skills/align-and-learn/SKILL.md"),
    ),
    (
        "sourcing-capabilities",
        include_str!("../skills/sourcing-capabilities/SKILL.md"),
    ),
    (
        "people-and-teams",
        include_str!("../skills/people-and-teams/SKILL.md"),
    ),
    (
        "asking-the-owner",
        include_str!("../skills/asking-the-owner/SKILL.md"),
    ),
];

/// The few lines of that doctrine that must hold on every turn, and when to load the rest.
const LOAD_RULE: &str = "Load a company skill with `restless skill use <name>` before the step it covers, once per session.";
const WRITING_POINTER: &str = "- `writing-for-the-owner`: before writing a Work title or outcome, a resolution, an artifact or gate label, or a document a person will read. In short: open with one or two plain sentences the owner can read, then the exact contract unchanged; no ids, paths or commands in what a person reads.";
const BRIEFS_POINTER: &str = "- `owner-briefs`: before raising anything to the owner (`restless work handoff`, `prepare-owner-brief`, `escalate-handoff`). First finish every machine-doable step; then recommend one course and ask for the smallest exact action.";
const SOURCING_POINTER: &str = "- `sourcing-capabilities`: when an outcome needs a capability, app, account, service or provider the company does not have. New Staff is one option, not the default answer.";
const ALIGN_POINTER: &str = "- `align-and-learn`: before commissioning anything customers will see (a site, deck, post, email or copy), and whenever someone corrects the work or states a lasting preference.";
const READ_BACK: &str = "Always: before saying something is done, saved, removed or changed, read it back where the owner sees it (`restless work graph`, `restless charter show`, `restless identity show`, `restless people`) and report what you read, not what you meant to do. When someone attaches an image, open it with your file reader before answering.";

/// Exec's doctrine section: pointers plus the every-turn invariants.
pub fn exec_doctrine() -> String {
    format!(
        "# Company doctrine [load the skill when it applies]\n{LOAD_RULE}\n{WRITING_POINTER}\n{BRIEFS_POINTER}\n{ALIGN_POINTER}\n\
         - `running-the-business`: before proposing an improvement, adding Staff, choosing a model, opening or closing a Goal. A proposal is not a change: charter, mission or identity, standing teams, ongoing cost, tools, access and outside actions need the owner's yes. For an owner goal within the spend limit, act then tell: start work, add follow-ups and schedules, say how to undo.\n\
         {SOURCING_POINTER}\n{READ_BACK}"
    )
}

/// A lead's doctrine section: the pointers a lead acts on.
pub fn lead_doctrine() -> String {
    format!(
        "# Company doctrine [load the skill when it applies]\n{LOAD_RULE}\n{WRITING_POINTER}\n{BRIEFS_POINTER}\n{ALIGN_POINTER}\n{SOURCING_POINTER}\n{READ_BACK}"
    )
}

/// Every Staff turn: where to look when a capability is missing.
pub fn staff_doctrine() -> String {
    format!("# Company doctrine [load the skill when it applies]\n{LOAD_RULE}\n{SOURCING_POINTER}")
}
