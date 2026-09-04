//! restless — one surface, two consumers (sprint 01 T10). The owner drives
//! the company from the host over the daemon's unix socket; agents inside
//! the company container drive coordination through the same binary over
//! TCP (unix sockets do not survive the Docker Desktop file share — probed,
//! not guessed). The CLI is a dumb client: the trust boundary is the
//! daemon's listeners, not this binary (§6.1).
//!
//! Environment defaults (so agents can just type `restless work list`):
//!   RESTLESS_COMPANY      — whose coordination state to touch
//!   RESTLESS_ACTOR        — who "message send" is from
//!   RESTLESS_COORDINATOR  — local host:port or hosted wss:// coordination URL
//!   RESTLESS_OWNER_URL    — loopback owner gateway used by `chat` and probed by `doctor`
//!   RESTLESS_COCKPIT_URL  — optional dev cockpit origin probed by `doctor`

mod appliance;
mod chat;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};

const HOSTED_COORDINATION_PATH: &str = "/internal/v1/coordination";
const MAX_COORDINATION_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Parser)]
#[command(name = "restless", about = "Restless owner surface")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Install and operate the dependable local Restless appliance.
    Appliance {
        #[command(subcommand)]
        command: ApplianceCommand,
    },
    /// Open the owner Cockpit for the selected machine profile.
    Open,
    /// Create and configure companies without hand-editing daemon state.
    Company {
        #[command(subcommand)]
        command: CompanyCommand,
    },
    /// Configure and probe secret references. Secret values remain in their backend.
    Credential {
        #[command(subcommand)]
        command: CredentialCommand,
    },
    /// External capabilities installed by Exec and attached to selected actor sessions.
    ConnectedTool {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY", global = true)]
        company: Option<String>,
        #[command(subcommand)]
        command: ConnectedToolCommand,
    },
    /// Authority-owned legal identity safe for ordinary company use.
    Legal {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY", global = true)]
        company: Option<String>,
        #[command(subcommand)]
        command: LegalCommand,
    },
    /// Bounded operating-money observation, preparation and provider submission.
    Finance {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY", global = true)]
        company: Option<String>,
        #[command(subcommand)]
        command: FinanceCommand,
    },
    /// Publish one immutable Work/Attempt artifact through a bounded demo or game-server profile.
    Publish {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY", global = true)]
        company: Option<String>,
        #[command(subcommand)]
        command: PublishCommand,
    },
    /// Bring a company environment up (create if absent, then start).
    Up {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Clone a live company's mission and config into this one as a
        /// throwaway (S04-T1). Target name must end in `_test`; every real
        /// provider, credential, standing approval and the sender address are
        /// stripped. Install a deterministic fake CLI to exercise effects.
        #[arg(long)]
        from: Option<String>,
        /// Rebuild the company image from this Restless source tree and
        /// replace an outdated container while preserving its volume.
        #[arg(long)]
        reconcile: bool,
    },
    /// Stop a company environment. The volume — files, Git, browser — survives.
    Down {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Remove container, volume, OrgIntel schema AND spend spool.
        /// Throwaway companies only — a live company's history is evidence.
        #[arg(long)]
        destroy: bool,
    },
    /// Company environment lifecycle status.
    Status {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Probe the complete local owner path and report an exact repair action.
    Doctor {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Ensure and inspect the company's OrgIntel schema.
    OrgintelInit {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Source-owned company truth and expression evidence.
    Identity {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY", global = true)]
        company: Option<String>,
        #[command(subcommand)]
        command: IdentityCommand,
    },
    /// Owner attention, projected from its source planes.
    Attention {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Render the compact owner queue and the existing source-owned typed
        /// commands. Omit for the full JSON projection.
        #[arg(long)]
        summary: bool,
    },
    /// Persistent browser controller coordination and health.
    Browser {
        #[command(subcommand)]
        command: BrowserCommand,
    },
    /// Wake the company's Exec for one turn (rehydrate → work → decide).
    Wake {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Why this wake is happening; defaults to an owner-requested wake.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Issue a directive to the Exec — also how a blocked judgement request
    /// is answered. Wakes the company.
    Tell {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        body: String,
    },
    /// Hold a full-screen owner conversation through the same gateway as the Cockpit.
    Chat {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Durable actor to talk with. Exec is the ordinary owner conversation.
        #[arg(long, short = 'a', default_value = "exec")]
        actor: String,
    },
    /// Stream the company's operational event stream (snapshot, then live).
    Watch {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Drop into a shell on the company computer.
    Attach {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Run an ordinary command on the company computer. Omit it for an
        /// interactive shell. This is the unbounded runtime door: do not grow
        /// one Restless verb per Linux command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Inspect, commission, or explicitly retire durable company actors.
    People {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY", global = true)]
        company: Option<String>,
        /// Include explicitly retired actors in the list.
        #[arg(long)]
        include_retired: bool,
        #[command(subcommand)]
        command: Option<PeopleCommand>,
    },
    /// Teams and the leads accountable for them. A lead absorbs coordination and
    /// judgement below the Exec so they stop reaching the owner (S06-T4).
    Teams {
        #[command(subcommand)]
        command: TeamCommand,
    },
    /// Durable time facts. Exact and weekday schedules create opportunities to inspect; they run no command.
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommand,
    },
    /// Judgement an actor owes — a team lead's queue, the same shape as the
    /// owner's `attention` (S06-T5).
    Judgement {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// The actor whose queue to read.
        #[arg(long = "as")]
        as_actor: String,
    },
    /// What the company's receipts actually record — the strongest evidence
    /// the system holds about what it did to the world.
    Receipts {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Only this governance class, e.g. `customer-contact.email`.
        #[arg(long = "class")]
        effect_class: Option<String>,
        #[arg(long, default_value = "50")]
        limit: i64,
    },
    /// Spend against the ceiling, broken down by actor and model.
    Spend {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Exact decision telemetry projected from Attempts, gates, model requests,
    /// messages and Runtime events. Unavailable measurements remain null.
    Telemetry {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Preview or apply an audited correction to exact duplicate spend records.
    /// Preview is the default; mutation requires both owner authority and --apply.
    SpendCorrect {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Stable idempotency/audit UUID chosen before the correction is attempted.
        #[arg(long = "correction-id")]
        correction_id: String,
        /// Exact duplicated request UUID. Repeat for every record being removed.
        #[arg(long = "request", required = true)]
        request_ids: Vec<String>,
        /// Exact negative cost of the referenced records, in micro-USD.
        #[arg(long, allow_hyphen_values = true)]
        delta_micro_usd: i64,
        /// Why these exact records are duplicates.
        #[arg(long)]
        reason: String,
        /// Append the correction. Omit this flag for a read-only preview.
        #[arg(long)]
        apply: bool,
    },
    /// Inspect and create Goals, or attach existing Work to one.
    #[command(alias = "goals")]
    Goal {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY", global = true)]
        company: Option<String>,
        #[command(subcommand)]
        command: Option<GoalCommand>,
    },
    /// Inspect and change the one canonical Work graph.
    Work {
        #[command(subcommand)]
        command: WorkCommand,
    },
    /// Recent events from the operational stream (newest first).
    Events {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long, default_value = "50")]
        limit: i64,
    },
    /// Read unread mail. Your own read consumes it; inspecting with --as does not.
    Inbox {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long = "as")]
        as_actor: Option<String>,
    },
    /// Send mail between actors (omit --to for the owner).
    Message {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long, env = "RESTLESS_ACTOR")]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        /// Link this message to exact Work. With --to, owner → Work lead;
        /// without --to, the accountable lead → owner.
        #[arg(long)]
        work: Option<String>,
        body: String,
    },
    /// Clear a fail-closed spend poison after inspecting why it happened.
    /// A poison stops a company dead; without this it stops it forever.
    ClearPoison {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Approve a party for real external effects (S03-T5). First contact
    /// through a real provider needs a human yes; this is that yes, and it is
    /// per-party rather than per-send so the owner governs rather than
    /// dispatches (`owner-cockpit` §2.3).
    Approve {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// The address or party identifier being approved.
        #[arg(long)]
        party: String,
        /// Withdraw standing approval instead of granting it.
        #[arg(long)]
        revoke: bool,
        /// Close the current request without granting standing authority.
        #[arg(long, conflicts_with = "revoke")]
        decline: bool,
    },
    /// Run an ordinary command as a governed external effect.
    Effect {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long = "class")]
        effect_class: String,
        #[arg(long)]
        party: Option<String>,
        /// Human-readable reason for this material consequence.
        #[arg(long)]
        purpose: String,
        /// Runtime artifact or attachment URI carried by the effect.
        #[arg(long = "artifact")]
        artifacts: Vec<String>,
        /// Map one child-process env name to a configured secret binding,
        /// e.g. RESEND_API_KEY=resend.production.
        #[arg(long = "secret")]
        secrets: Vec<String>,
        #[arg(long, default_value = "/company")]
        cwd: String,
        /// Idempotency key: a retry with the same key replays the receipt.
        #[arg(long)]
        key: String,
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Resolve an interrupted effect only after a separate status-check
    /// receipt establishes what the external tool observed.
    EffectReconcile {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        key: String,
        #[arg(long)]
        execution: i32,
        /// succeeded or failed.
        #[arg(long)]
        result: String,
        #[arg(long)]
        evidence_receipt: String,
    },
    /// Internal stdin bridge used by the trusted host daemon. It receives
    /// one process envelope and never resolves credentials itself.
    #[command(hide = true)]
    EffectChild,
}

#[derive(Subcommand)]
enum ApplianceCommand {
    /// Stage this CLI, its sibling daemon and built Cockpit, then install the user service.
    Install {
        #[arg(long)]
        daemon: Option<PathBuf>,
        #[arg(long)]
        cockpit: Option<PathBuf>,
        /// Import only company-referenced environment credentials and provider
        /// endpoints into the stable appliance's private credential file.
        #[arg(long)]
        environment: Option<PathBuf>,
    },
    /// Stage and activate a new release, rolling back if readiness fails.
    Upgrade {
        #[arg(long)]
        daemon: Option<PathBuf>,
        #[arg(long)]
        cockpit: Option<PathBuf>,
        /// Refresh the private, filtered stable environment from this dotenv file.
        #[arg(long)]
        environment: Option<PathBuf>,
    },
    /// Activate the previous known-good release.
    Rollback,
    /// Inspect service definitions, lock ownership and owner readiness.
    Status,
    /// Bootstrap or restart the installed user services.
    Start,
    /// Stop the installed user services without touching company state.
    Stop,
    /// Deliver one bounded schedule-reconciliation hint to the running daemon.
    WakeDue {
        #[arg(long, default_value = "manual")]
        adapter: String,
    },
    /// Remove services and owned machine caches while retaining company data.
    Uninstall,
}

#[derive(Subcommand)]
enum PublishCommand {
    /// Bind a versioned service manifest to an exact immutable source artifact.
    Candidate {
        #[arg(long)]
        source_artifact: String,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long, env = "RESTLESS_ACTOR")]
        actor: Option<String>,
    },
    /// Request an exact audience, expiry and resource envelope; no provider runs yet.
    Request {
        #[arg(long)]
        candidate_artifact: String,
        #[arg(long, value_parser = ["owner-only", "named-invitees", "public"])]
        audience: String,
        #[arg(long)]
        start_deadline: String,
        #[arg(long)]
        expires_at: String,
        #[arg(long, default_value = "500")]
        cpu_millis: u32,
        #[arg(long, default_value = "512")]
        memory_mib: u32,
        #[arg(long, default_value = "512")]
        ephemeral_storage_mib: u32,
        #[arg(long, default_value = "32")]
        max_connections: u32,
        #[arg(long)]
        key: String,
        #[arg(long, env = "RESTLESS_ACTOR")]
        actor: Option<String>,
    },
    /// Owner authorization for the exact request consequences.
    Authorize {
        #[arg(long)]
        publication: String,
    },
    /// Mint a signed, scoped and expiring invitation.
    Invite {
        #[arg(long)]
        publication: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        expires_at: String,
    },
    /// Revoke one invitation immediately.
    Revoke {
        #[arg(long)]
        invitation: String,
    },
    /// Capture current provider activity as an Authority observation.
    Observe {
        #[arg(long)]
        publication: String,
    },
    /// Recover an authorized provider after daemon/provider interruption.
    Reconcile {
        #[arg(long)]
        publication: String,
    },
    /// Stop the provider and prove process, route, invitation and temp cleanup.
    Stop {
        #[arg(long)]
        publication: String,
        #[arg(long)]
        reason: String,
    },
    /// Show one publication and all of its source-owned records.
    Show {
        #[arg(long)]
        publication: String,
    },
    /// List all publication records for the company.
    List,
}

#[derive(Subcommand)]
#[allow(
    clippy::large_enum_variant,
    reason = "clap owns this one-shot CLI value; boxing its argument variants adds indirection without reducing a resident data structure"
)]
enum IdentityCommand {
    /// Show the effective release, proposals, evidence, lineage and bindings.
    Show,
    /// Add one attributed evidence statement. This does not make it effective.
    Evidence {
        #[arg(long)]
        pillar: String,
        #[arg(long = "kind")]
        statement_kind: String,
        #[arg(long)]
        claim: String,
        #[arg(long)]
        statement: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        authority: String,
        #[arg(long, default_value = "company")]
        scope: String,
        #[arg(long)]
        locator: String,
        #[arg(long, default_value = "neutral")]
        polarity: String,
        #[arg(long, default_value = "active")]
        status: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        audience: Option<String>,
        #[arg(long)]
        supersedes: Option<String>,
        #[arg(long)]
        expires_at: Option<String>,
        #[arg(long)]
        indefinite: bool,
    },
    /// Propose a complete release from exact evidence ids.
    Propose {
        #[arg(long)]
        reason: String,
        #[arg(long = "evidence", required = true)]
        evidence_ids: Vec<String>,
    },
    /// Compile one deterministic bounded brief for a concrete outcome.
    Brief {
        #[arg(long)]
        outcome: String,
        #[arg(long)]
        channel: String,
        #[arg(long)]
        audience: String,
        #[arg(long)]
        release: Option<String>,
        #[arg(long, default_value = "8192")]
        max_bytes: usize,
    },
    /// Add typed, attributed Voice evidence. It remains ineffective until an owner release.
    VoiceEvidence {
        #[arg(long = "kind")]
        voice_kind: String,
        #[arg(long)]
        claim: String,
        #[arg(long)]
        statement: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        authority: String,
        #[arg(long, default_value = "company")]
        scope: String,
        #[arg(long)]
        locator: String,
        #[arg(long)]
        judgement: String,
        #[arg(long, default_value = "positive")]
        polarity: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        audience: Option<String>,
        #[arg(long)]
        named_author: Option<String>,
        #[arg(long)]
        supersedes: Option<String>,
    },
    /// Set the human communication situation for one voice-producing Work.
    VoiceBind {
        #[arg(long)]
        work: String,
        #[arg(long)]
        channel: String,
        #[arg(long)]
        author: String,
        #[arg(long)]
        audience: String,
        #[arg(long)]
        reader: String,
        #[arg(long)]
        understanding: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        proof: String,
        #[arg(long)]
        consequence: String,
    },
    /// Inspect the deterministic voice contract already bound to one Work.
    VoiceBrief {
        #[arg(long)]
        work: String,
        #[arg(long, default_value = "8192")]
        max_bytes: usize,
    },
    /// Bind representative native-render checks to an exact artifact.
    VoiceRender {
        #[arg(long)]
        artifact: String,
        #[arg(long)]
        channel: String,
        #[arg(long)]
        renderer: String,
        #[arg(long)]
        renderer_version: String,
        /// JSON object of native semantic checks and observed results.
        #[arg(long)]
        checks: String,
    },
    /// Record a blinded copy-desk decision on one native render.
    VoiceReview {
        #[arg(long)]
        render: String,
        #[arg(long)]
        verdict: String,
        #[arg(long, default_value = "")]
        factual: String,
        #[arg(long, default_value = "")]
        abstraction: String,
        #[arg(long, default_value = "")]
        repetition: String,
        #[arg(long, default_value = "")]
        channel: String,
        #[arg(long, default_value = "")]
        authorship: String,
        #[arg(long, default_value = "")]
        concepts_removed: String,
    },
    /// Turn an exact owner edit into a scoped proposal; typo and fact fixes create no Voice rule.
    VoiceLearn {
        #[arg(long)]
        before: String,
        #[arg(long)]
        after: String,
        #[arg(long = "kind")]
        learning_kind: String,
        #[arg(long)]
        claim: String,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        decision: String,
        #[arg(long, default_value = "company")]
        scope: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        locator: String,
        #[arg(long)]
        named_author: Option<String>,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        audience: Option<String>,
    },
    /// Add typed Visual Language evidence or one inspectable registry capability.
    VisualEvidence {
        #[arg(long = "kind")]
        visual_kind: String,
        #[arg(long)]
        claim: String,
        #[arg(long)]
        statement: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        authority: String,
        #[arg(long, default_value = "company")]
        scope: String,
        #[arg(long)]
        locator: String,
        #[arg(long)]
        purpose: String,
        #[arg(long)]
        rationale: String,
        #[arg(long)]
        accessibility: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        reduced_motion: Option<String>,
        #[arg(long)]
        product_truth: Option<String>,
        #[arg(long)]
        origin: Option<String>,
        #[arg(long)]
        licence: Option<String>,
        #[arg(long)]
        framework: Option<String>,
        #[arg(long)]
        adaptation: Option<String>,
        #[arg(long, default_value = "[]")]
        dependencies: String,
        #[arg(long)]
        semantic_role: Option<String>,
        #[arg(long)]
        value: Option<String>,
        #[arg(long, default_value = "positive")]
        polarity: String,
    },
    /// Bind channel art direction and product-representation truth to one Work.
    VisualBind {
        #[arg(long)]
        work: String,
        #[arg(long)]
        channel: String,
        #[arg(long)]
        audience: String,
        #[arg(long)]
        outcome: String,
        #[arg(long)]
        hierarchy: String,
        #[arg(long)]
        proof: String,
        #[arg(long)]
        density: String,
        #[arg(long)]
        imagery: String,
        #[arg(long)]
        motion: String,
        #[arg(long)]
        representation: String,
        #[arg(long)]
        product_truth: Option<String>,
        #[arg(long)]
        departure: Option<String>,
    },
    /// Inspect the deterministic Visual Language direction for one Work.
    VisualBrief {
        #[arg(long)]
        work: String,
        #[arg(long, default_value = "10240")]
        max_bytes: usize,
    },
    /// Record the exact released primitive version actually selected for Work.
    VisualUse {
        #[arg(long)]
        work: String,
        #[arg(long)]
        evidence: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        purpose: String,
    },
    /// Bind native viewport, motion and accessibility checks to an exact artifact.
    VisualRender {
        #[arg(long)]
        work: String,
        #[arg(long)]
        artifact: String,
        #[arg(long)]
        channel: String,
        #[arg(long)]
        renderer: String,
        #[arg(long)]
        renderer_version: String,
        #[arg(long)]
        width: i32,
        #[arg(long)]
        height: i32,
        #[arg(long)]
        motion_state: String,
        #[arg(long)]
        checks: String,
    },
    /// Record independent art direction, optionally against a restrained control.
    VisualReview {
        #[arg(long)]
        render: String,
        #[arg(long)]
        control: Option<String>,
        #[arg(long)]
        verdict: String,
        #[arg(long, default_value = "")]
        identity: String,
        #[arg(long, default_value = "")]
        hierarchy: String,
        #[arg(long, default_value = "")]
        density: String,
        #[arg(long, default_value = "")]
        proof: String,
        #[arg(long, default_value = "")]
        product_fidelity: String,
        #[arg(long, default_value = "")]
        motion: String,
        #[arg(long, default_value = "")]
        defects: String,
        #[arg(long, default_value = "")]
        departure: String,
    },
    /// Add observed conduct with consequence, counterexample and boundary.
    CultureEvidence {
        #[arg(long = "kind")]
        culture_kind: String,
        #[arg(long)]
        case_kind: Option<String>,
        #[arg(long)]
        claim: String,
        #[arg(long)]
        statement: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        authority: String,
        #[arg(long, default_value = "company")]
        scope: String,
        #[arg(long)]
        locator: String,
        #[arg(long)]
        situation: String,
        #[arg(long)]
        consequence: String,
        #[arg(long)]
        actors: String,
        #[arg(long)]
        decision_authority: String,
        #[arg(long)]
        conduct: String,
        #[arg(long)]
        observed_outcome: String,
        #[arg(long)]
        confidence: String,
        #[arg(long)]
        counterexample: String,
        #[arg(long)]
        boundary: String,
        #[arg(long)]
        implication: String,
        #[arg(long, default_value = "company")]
        actor_scope: String,
    },
    /// Bind a consequence-relevant cultural posture to one Work.
    CultureBind {
        #[arg(long)]
        work: String,
        #[arg(long)]
        case_kind: String,
        #[arg(long)]
        actor: String,
        #[arg(long)]
        actor_role: String,
        #[arg(long)]
        team: String,
        #[arg(long)]
        consequence: String,
        #[arg(long)]
        decision_boundary: String,
    },
    CultureBrief {
        #[arg(long)]
        work: String,
        #[arg(long, default_value = "8192")]
        max_bytes: usize,
    },
    /// Bind an exact decision or communication artifact to a culture case.
    CultureCase {
        #[arg(long)]
        work: String,
        #[arg(long)]
        artifact: String,
        #[arg(long)]
        case_kind: String,
        #[arg(long)]
        decision: String,
        #[arg(long, default_value = "[]")]
        alternatives: String,
        #[arg(long)]
        unknowns: String,
        #[arg(long)]
        correction_of: Option<String>,
        #[arg(long, default_value = "")]
        correction_account: String,
        #[arg(long, default_value = "")]
        customer_action: String,
        #[arg(long)]
        checks: String,
    },
    CultureReview {
        #[arg(long)]
        record: String,
        #[arg(long)]
        verdict: String,
        #[arg(long, default_value = "")]
        conduct: String,
        #[arg(long, default_value = "")]
        dissent: String,
        #[arg(long, default_value = "")]
        uncertainty: String,
        #[arg(long, default_value = "")]
        correction: String,
        #[arg(long, default_value = "")]
        authority: String,
        #[arg(long, default_value = "")]
        customer_or_hiring: String,
        #[arg(long, default_value_t = false)]
        slogan_recitation: bool,
    },
}

#[derive(serde::Deserialize)]
struct EffectChildEnvelope {
    argv: Vec<String>,
    env: std::collections::BTreeMap<String, String>,
}

#[derive(Subcommand)]
enum PeopleCommand {
    /// Commission one stable specialist after inspecting the current People list.
    Create {
        /// Stable organisational id; do not encode a Work revision in it.
        #[arg(long)]
        id: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        display: String,
        #[arg(long)]
        model: Option<String>,
        /// What difference this specialist buys over the actors already listed.
        #[arg(long)]
        reason: String,
    },
    /// Change an actor's next-wake model preference explicitly.
    Model {
        #[arg(long)]
        actor: String,
        #[arg(long)]
        model: String,
        /// Why this model is a better fit than the current preference.
        #[arg(long)]
        reason: String,
    },
    /// Retire an unused actor without deleting its Work or attribution.
    Retire {
        #[arg(long)]
        actor: String,
        #[arg(long)]
        reason: String,
    },
}

#[derive(Subcommand)]
enum TeamCommand {
    /// Live teams with their leads and members.
    List {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Form a team around an accountable lead. The lead joins its own team.
    Create {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        name: String,
        /// The actor accountable for this team.
        #[arg(long)]
        lead: String,
        /// Why this team exists and what it is accountable for.
        #[arg(long)]
        brief: String,
        /// Effective outcome ambition. Omit only to inherit company policy.
        #[arg(long)]
        standard: Option<String>,
        /// company_default, owner_override, or owner_language.
        #[arg(long)]
        standard_source: Option<String>,
        /// Owner message supporting an override or language interpretation.
        #[arg(long)]
        source_message: Option<i64>,
    },
    /// Rename a team or revise its outcome charter. Owner/Exec only.
    Update {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Team name or id.
        #[arg(long)]
        team: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        brief: Option<String>,
        #[arg(long)]
        reason: String,
    },
    /// Move an actor into a team, or out of every team with `--team none`.
    Assign {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        actor: String,
        /// Team name or id, or `none` to unassign.
        #[arg(long)]
        team: String,
        /// What capability, capacity, or repair this roster change buys.
        #[arg(long)]
        reason: String,
    },
    /// Replace a team's accountable lead.
    Lead {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Team name or id.
        #[arg(long)]
        team: String,
        #[arg(long)]
        actor: String,
        #[arg(long)]
        reason: String,
    },
    /// Disband a team. Members become unassigned and any judgement the team
    /// still owed falls through to the Exec, recorded rather than dropped.
    Disband {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Team name or id.
        #[arg(long)]
        team: String,
        #[arg(long)]
        reason: String,
    },
}

#[derive(Subcommand)]
enum ScheduleCommand {
    /// Inspect pending schedules, or include already fired/cancelled history.
    List {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Limit the view to one actor. Runtime actors may inspect only themselves.
        #[arg(long = "as")]
        as_actor: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Inspect fired and skipped occurrences for one schedule.
    History {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Schedule UUID.
        #[arg(long)]
        schedule: String,
        #[arg(long, default_value_t = 20)]
        limit: i64,
    },
    /// Recover one recorded skipped occurrence with one idempotent actor wake.
    Recover {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Schedule UUID.
        #[arg(long)]
        schedule: String,
        /// Exact skipped occurrence time in RFC3339 form.
        #[arg(long)]
        scheduled_for: String,
        /// The accountable actor whose skipped occurrence is being recovered.
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: String,
        /// Attribution for the person or system requesting recovery.
        #[arg(long, default_value = "owner")]
        requested_by: String,
        /// Why this occurrence should be recovered now.
        #[arg(long)]
        reason: String,
    },
    /// Retry a failed recovery wake with an explicit idempotency key.
    RetryRecovery {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        schedule: String,
        #[arg(long)]
        scheduled_for: String,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: String,
        /// Exact prior recovery message that was reconciled as failed.
        #[arg(long)]
        prior_message: i64,
        /// Stable key for this one retry; repeating it is a read, not a new wake.
        #[arg(long)]
        key: String,
        #[arg(long, default_value = "owner")]
        requested_by: String,
        #[arg(long)]
        reason: String,
    },
    /// Add a genuinely time-driven wake: one exact instant, or one weekday local-time cadence.
    Add {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// The accountable actor to wake. Defaults to the current Runtime actor.
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
        /// RFC3339 instant, for example 2026-08-28T09:30:00Z. Conflicts with --weekdays.
        #[arg(long)]
        at: Option<String>,
        /// Wake on weekdays at --at-local in --timezone. This wakes judgement; it runs no command.
        #[arg(long)]
        weekdays: bool,
        /// Local wall-clock time in HH:MM form, for example 09:00.
        #[arg(long)]
        at_local: Option<String>,
        /// IANA timezone, for example Australia/Sydney.
        #[arg(long)]
        timezone: Option<String>,
        /// What to do if the daemon was down: skip, skip-if-late, catch-up, or coalesce-latest.
        #[arg(long)]
        on_missed: Option<String>,
        /// Catch-up window for --on-missed catch-up. After this many minutes, skip.
        #[arg(long)]
        catch_up_within_minutes: Option<i64>,
        /// local-mac, or always-on. Always-on work waits for a capable runner.
        #[arg(long, default_value = "local-mac")]
        execution: String,
        #[arg(long)]
        reason: String,
        /// Optional Work waiting on this exact time condition.
        #[arg(long)]
        work: Option<String>,
    },
    /// Change how one live recurring schedule handles a missed clock time.
    Policy {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Schedule UUID.
        #[arg(long)]
        schedule: String,
        /// The accountable actor whose schedule is being changed.
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: String,
        /// skip, skip-if-late, catch-up, or coalesce-latest.
        #[arg(long)]
        on_missed: String,
        /// Required for every bounded policy; the stale wake is skipped after this window.
        #[arg(long)]
        catch_up_within_minutes: Option<i64>,
    },
    /// Stop one pending exact or recurring schedule. Fired history remains visible with --all.
    Cancel {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        /// Schedule UUID.
        #[arg(long)]
        schedule: String,
        /// The accountable actor whose schedule is being stopped.
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: String,
        #[arg(long)]
        reason: String,
    },
}

#[derive(Subcommand)]
enum WorkCommand {
    List {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Read the complete graph projection, including Attempts and evidence.
    Graph {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    Attempts {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        work: Option<String>,
    },
    Add {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        owner: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        model: Option<String>,
        /// coherent-single-worker or locally-closing-parallel-unit. Exec may
        /// use the coherent route only when the team has exactly one worker;
        /// every other production commission belongs to the accountable lead.
        #[arg(long, default_value = "coherent-single-worker")]
        topology: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        outcome: String,
        #[arg(long, default_value_t = 0)]
        priority: i16,
        #[arg(long, default_value = "")]
        expected_artifact: String,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        base_ref: Option<String>,
        /// Existing checked-out shared branch to fast-forward after this
        /// final accepted Work passes. Dependencies carry intermediate commits.
        #[arg(long)]
        integration_branch: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        attempt_limit: Option<i32>,
        /// Require one prepared native ReviewTarget and a recorded
        /// `review-target-live-probe` gate before this Work reaches owner
        /// outcome review. It never accepts the outcome automatically.
        #[arg(long)]
        owner_review: bool,
        /// Existing Goal this Work serves.
        #[arg(long)]
        goal: Option<String>,
        /// Authenticated external message that caused this Work. The
        /// accountable lead may attach it once; creation and linkage commit
        /// atomically so redelivery cannot commission duplicate Work.
        #[arg(long)]
        source_message: Option<i64>,
        /// Existing Work this node requires. Repeat for more than one. These
        /// edges are committed atomically with the node so it cannot start
        /// against a half-built graph.
        #[arg(long)]
        requires: Vec<String>,
        /// Existing producer Work this reviewer may revise. Repeat for more
        /// than one; committed atomically with the node.
        #[arg(long)]
        revises: Vec<String>,
        /// Deterministic acceptance gate, as JSON with `name` and argv
        /// `command`. Repeat for more than one. Gates run in the current
        /// Attempt workspace and commit atomically with the Work node.
        #[arg(long)]
        gate: Vec<String>,
        /// Company Constitution situation JSON committed atomically with Work.
        /// The object may contain voice, visual and/or culture contracts.
        #[arg(long)]
        constitution_contracts: Option<String>,
    },
    /// Move unsettled Work to another durable actor.
    Assign {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        work: String,
        #[arg(long)]
        owner: String,
        #[arg(long)]
        reason: String,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    Edge {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        kind: String,
        /// Remove this edge instead of adding it.
        #[arg(long)]
        remove: bool,
        /// Why the dependency is wrong. Required with --remove.
        #[arg(long)]
        reason: Option<String>,
        /// Owner, Exec, or the accountable lead making the repair.
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    Artifact {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        work: String,
        #[arg(long)]
        attempt: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        uri: String,
        #[arg(long)]
        digest: Option<String>,
        #[arg(long)]
        source_commit: Option<String>,
        #[arg(long, default_value = "output")]
        label: String,
        #[arg(long, default_value = "")]
        note: String,
    },
    /// Retire a stale artifact reference without deleting its history.
    RetireArtifact {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        artifact: String,
        #[arg(long)]
        reason: String,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    Gate {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        work: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        cwd: String,
        #[arg(long, default_value = "cumulative")]
        stage: String,
        #[arg(long, default_value_t = 900)]
        timeout_seconds: i32,
        /// Scarce Runtime resource to lease: port or display. Repeatable.
        #[arg(long = "resource")]
        resources: Vec<String>,
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Retire a mistaken deterministic gate while preserving its historical
    /// runs. Declare any replacement before resuming the blocked Work.
    RetireGate {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        gate: String,
        #[arg(long)]
        reason: String,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    Handoff {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        work: String,
        #[arg(long)]
        attempt: Option<String>,
        #[arg(long)]
        /// identity, captcha, mfa, legal-attestation, payment-confirmation, or owner-judgement
        category: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        prepared: String,
        #[arg(long)]
        resume_when: String,
    },
    /// Replace stale prepared evidence on an outstanding handoff without
    /// resolving it or creating a second owner request.
    RefreshHandoff {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        handoff: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        prepared: String,
        #[arg(long)]
        resume_when: String,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    /// Author the stable owner-altitude meaning of the current handoff source
    /// snapshot. This prepares attention; it does not admit or resolve it.
    PrepareOwnerBrief {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        handoff: String,
        /// outcome_review, decision, blocker, opportunity, contradiction, human_step.
        #[arg(long)]
        kind: String,
        #[arg(long)]
        headline: String,
        #[arg(long)]
        situation: String,
        #[arg(long)]
        impact: String,
        #[arg(long)]
        recommendation: String,
        #[arg(long)]
        no_action: String,
        #[arg(long)]
        uncertainty: Option<String>,
        #[arg(long)]
        deadline: Option<String>,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    /// Record the observed outcome of one prepared owner handoff.
    ResolveHandoff {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        handoff: String,
        /// resolved, declined, or withdrawn.
        #[arg(long)]
        state: String,
        #[arg(long)]
        resolution: String,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    /// Pass a judgement up because it is outside this actor's remit. The chain
    /// is recorded: the owner sees who tried first and why they stopped.
    EscalateHandoff {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        handoff: String,
        /// The actor passing it up. Must be the actor it is assigned to.
        #[arg(long = "as")]
        as_actor: String,
        #[arg(long)]
        reason: String,
    },
    /// Resume a blocked node after changing the failed mechanism.
    Interrupt {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        work: String,
        #[arg(long)]
        reason: String,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    /// Resume a blocked node after changing the failed mechanism.
    Resume {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        work: String,
        #[arg(long)]
        reason: String,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    /// Retire superseded or no-longer-needed Work without deleting its history.
    Abandon {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        work: String,
        #[arg(long)]
        reason: String,
        #[arg(long = "as", env = "RESTLESS_ACTOR")]
        as_actor: Option<String>,
    },
    /// Record the owner's explicit judgement on a prepared outcome.
    Review {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        handoff: String,
        /// accept or request_changes.
        #[arg(long)]
        decision: String,
        /// Exact revision guidance. Required for request_changes.
        #[arg(long, default_value = "")]
        feedback: String,
    },
}

#[derive(Subcommand)]
enum GoalCommand {
    /// List the company's Goals.
    List,
    /// Create one durable desired outcome.
    Add {
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "")]
        body: String,
    },
    /// Attach or reassign existing Work to an existing Goal.
    Attach {
        #[arg(long)]
        work: String,
        #[arg(long)]
        goal: String,
    },
}

#[derive(Subcommand)]
enum CompanyCommand {
    /// Validate and create a company from the canonical TOML interface.
    Create {
        #[arg(long)]
        from_file: PathBuf,
    },
    /// Print canonical TOML; credential references are shown, never values.
    Show {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Set one deterministic configuration key: mission, model, model_failover
    /// (a comma-separated ordered list), worker_runtime, reasoning_effort,
    /// spend_ceiling_usd, or credentials.<binding>. Name is immutable.
    Set {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        key: String,
        value: String,
    },
    /// Remove one named credential binding from company config. This removes
    /// only the reference; deleting backend secret material is a separate
    /// owner operation at that backend.
    Unset {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        key: String,
    },
    /// List configured companies.
    List,
}

#[derive(Subcommand)]
enum CredentialCommand {
    /// Store a named binding's scheme:locator reference, never its value.
    Set {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        binding: String,
        reference: String,
        /// Forward a value from @<file> or `-` (stdin) to a supporting backend.
        /// Secret material is never accepted as an argv value.
        #[arg(long)]
        value: Option<String>,
    },
    /// Promote a daemon bootstrap env: binding into Infisical without exposing
    /// its value to the CLI, command line, or company Runtime.
    Promote {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        binding: String,
        reference: String,
    },
    /// Probe every configured reference as present, absent or invalid.
    Check {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConnectedToolCommand {
    /// List Authority-observed connections. Raw OAuth material is never shown.
    List,
    /// Discover OAuth, prepare one owner consent handoff, and install after observation.
    Install {
        #[arg(long)]
        name: String,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        purpose: String,
        /// The only durable actor whose fresh sessions receive this MCP.
        #[arg(long)]
        actor: String,
        #[arg(long)]
        work: String,
        #[arg(long)]
        attempt: String,
        /// Provider-advertised scope. Repeat to preserve the live observation.
        #[arg(long = "scope", required = true)]
        scopes: Vec<String>,
    },
    /// Repeat provider authorization after expiry or revocation.
    Reconnect {
        #[arg(long)]
        name: String,
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        purpose: String,
        #[arg(long)]
        actor: String,
        #[arg(long)]
        work: String,
        #[arg(long)]
        attempt: String,
        #[arg(long = "scope", required = true)]
        scopes: Vec<String>,
    },
    /// Record the authenticated workspace and exact tools observed by the selected actor.
    Observe {
        #[arg(long)]
        name: String,
        #[arg(long)]
        workspace: String,
        #[arg(long = "tool", required = true)]
        tools: Vec<String>,
    },
    /// Stop attaching this connection to all future sessions.
    Disable {
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand)]
enum LegalCommand {
    /// Show the safe current profile and registry observation.
    Show,
    /// Live-probe the official Australian ABN Lookup service.
    Probe,
    /// Set one owner-confirmed safe legal profile. Restricted KYB documents
    /// are intentionally not accepted by this command.
    Set {
        #[arg(long)]
        legal_name: String,
        #[arg(long)]
        trading_name: Option<String>,
        #[arg(long)]
        entity_type: String,
        #[arg(long)]
        jurisdiction: String,
        #[arg(long)]
        registration_type: String,
        #[arg(long)]
        registration_value: String,
        #[arg(long)]
        business_address: String,
        #[arg(long)]
        invoice_email: Option<String>,
    },
}

#[derive(Subcommand)]
enum FinanceCommand {
    /// Show envelopes, payments, provider metadata and last observed balances.
    Show,
    /// Owner-set deterministic loss envelope for one currency/account.
    SetEnvelope {
        #[arg(long)]
        source_account: String,
        #[arg(long)]
        currency: String,
        #[arg(long = "beneficiary", required = true)]
        beneficiaries: Vec<String>,
        #[arg(long)]
        per_payment_minor: i64,
        #[arg(long)]
        aggregate_minor: i64,
    },
    /// Freeze or explicitly unfreeze new financial effects.
    Freeze {
        #[arg(long)]
        currency: String,
        #[arg(long)]
        unfreeze: bool,
    },
    /// Record the exact live-probed Airwallex account/scopes and owner action.
    ConnectAirwallex {
        #[arg(long, value_parser = ["sandbox", "live"])]
        environment: String,
        /// Exact YYYY-MM-DD API version observed in the Airwallex account.
        #[arg(long)]
        api_version: String,
        #[arg(long)]
        client_id: String,
        #[arg(long)]
        account_ref: String,
        #[arg(long)]
        approval_url: String,
        #[arg(long = "read-scope", required = true)]
        read_scopes: Vec<String>,
        #[arg(long = "submit-scope", required = true)]
        submit_scopes: Vec<String>,
        #[arg(long)]
        approval_workflow_observed: bool,
        #[arg(long, requires = "approval_workflow_observed")]
        observed_at: Option<String>,
    },
    /// Live-observe current provider balances with the read-only key.
    Balances,
    /// Live-probe both scoped connections without moving money.
    Probe,
    /// Atomically reserve one exact payment inside the owner-set envelope.
    Reserve {
        /// OrgIntel Work whose accepted outcome requires this payment.
        #[arg(long)]
        work: String,
        /// Pending payment-confirmation owner handoff for the provider step.
        #[arg(long)]
        handoff: String,
        #[arg(long)]
        source_account: String,
        #[arg(long)]
        beneficiary: String,
        #[arg(long)]
        amount_minor: i64,
        #[arg(long)]
        currency: String,
        #[arg(long)]
        purpose: String,
        #[arg(long = "evidence")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        key: String,
    },
    /// Submit a reserved payment through the host-side adapter. This cannot approve it.
    Submit {
        #[arg(long)]
        key: String,
    },
    /// Re-query authenticated provider state; required after ambiguity and approval.
    Reconcile {
        #[arg(long)]
        key: String,
    },
}

#[derive(Subcommand)]
enum BrowserCommand {
    /// Probe desktop, Chromium, automation and controller state.
    Status {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
    /// Yield the shared visible browser to the owner.
    Request {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long, env = "RESTLESS_ACTOR")]
        session: Option<String>,
    },
    /// Release an agent-held browser controller claim.
    Release {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
    },
}

/// Transport spelling only. The daemon derives real authority from its
/// listener and the signed Runtime/session grant, rather than trusting this
/// environment-derived label.
fn principal() -> &'static str {
    if std::env::var_os("RESTLESS_COORDINATOR").is_some() {
        "company/exec"
    } else {
        "owner"
    }
}

fn is_runtime() -> bool {
    std::env::var_os("RESTLESS_COORDINATOR").is_some()
}

/// A supervised actor receives an ephemeral session grant. An ordinary
/// Runtime shell falls back to its company-scoped bridge grant materialised
/// by the host on up. Neither variable is meaningful on the local Unix owner
/// transport.
fn runtime_capability() -> Option<String> {
    std::env::var("RESTLESS_SESSION_CAPABILITY")
        .ok()
        .or_else(|| std::env::var("RESTLESS_RUNTIME_CAPABILITY").ok())
        .or_else(|| {
            is_runtime()
                .then(|| std::fs::read_to_string("/company/run/restless-bridge.cap").ok())
                .flatten()
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// OrgIntel attribution is not kernel authority, but it must still name the
/// actor that made a coordination change. A missing actor inside the company
/// is the Exec, never the owner; otherwise an unset environment variable would
/// turn an ordinary runtime command into a forged owner override.
fn acting_actor() -> String {
    std::env::var("RESTLESS_ACTOR").unwrap_or_else(|_| {
        if principal() == "company/exec" {
            "exec".to_string()
        } else {
            "owner".to_string()
        }
    })
}

/// A team-lead coordination turn may communicate only through an explicit
/// recipient. In this mode an omitted `--to` is never an owner reply: it is a
/// common command-shape mistake (`restless message list`) that otherwise
/// creates unsolicited owner mail. This is a narrow usability guard, not a
/// new agent-security boundary; the shared company Runtime remains mutable.
fn require_message_recipient_in_coordination_wake(
    coordination_wake: bool,
    recipient: Option<&str>,
) -> Result<()> {
    if coordination_wake && recipient.is_none() {
        bail!(
            "an internal team coordination wake cannot send mail to the owner; use `restless inbox --as <actor>` to inspect mail, or `restless message --to <actor> <body>` for changed information"
        );
    }
    Ok(())
}

fn is_team_coordination_wake() -> bool {
    std::env::var("RESTLESS_COORDINATION_WAKE").is_ok_and(|value| value == "1")
}

fn state_root() -> PathBuf {
    if let Ok(root) = std::env::var("RESTLESS_HOME") {
        return PathBuf::from(root);
    }
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join(".restless")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    // Host commands must name a coherent appliance profile before they can
    // reach a socket, Docker resource or destructive lifecycle operation.
    // Runtime clients authenticate through RESTLESS_COORDINATOR instead and
    // deliberately do not inherit host filesystem identity.
    if !is_runtime() {
        restlessd::appliance::MachineProfile::from_env()?;
    }
    match cli.command {
        Command::Appliance { command } => match command {
            ApplianceCommand::Install {
                daemon,
                cockpit,
                environment,
            } => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&appliance::install(
                        daemon,
                        cockpit,
                        environment,
                        false,
                    )?)?
                );
                Ok(())
            }
            ApplianceCommand::Upgrade {
                daemon,
                cockpit,
                environment,
            } => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&appliance::install(
                        daemon,
                        cockpit,
                        environment,
                        true,
                    )?)?
                );
                Ok(())
            }
            ApplianceCommand::Rollback => {
                println!("{}", serde_json::to_string_pretty(&appliance::rollback()?)?);
                Ok(())
            }
            ApplianceCommand::Status => {
                println!("{}", serde_json::to_string_pretty(&appliance::status()?)?);
                Ok(())
            }
            ApplianceCommand::Start => {
                println!("{}", serde_json::to_string_pretty(&appliance::start()?)?);
                Ok(())
            }
            ApplianceCommand::Stop => {
                println!("{}", serde_json::to_string_pretty(&appliance::stop()?)?);
                Ok(())
            }
            ApplianceCommand::WakeDue { adapter } => {
                if !matches!(adapter.as_str(), "manual" | "launchd" | "systemd") {
                    bail!("wake adapter must be manual|launchd|systemd");
                }
                let request = stamp(serde_json::json!({
                    "cmd": "schedule-wake",
                    "adapter": adapter,
                }));
                print_response(&request_once(&request.to_string())?)
            }
            ApplianceCommand::Uninstall => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&appliance::uninstall()?)?
                );
                Ok(())
            }
        },
        Command::Open => appliance::open_owner(),
        Command::EffectChild => {
            let envelope: EffectChildEnvelope = serde_json::from_reader(std::io::stdin())
                .context("read governed child envelope")?;
            let (program, args) = envelope.argv.split_first().context("empty child argv")?;
            let status = std::process::Command::new(program)
                .args(args)
                .envs(envelope.env)
                .status()
                .with_context(|| format!("run governed child {program:?}"))?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Command::Attach {
            company: name,
            command,
        } => {
            let name = name.context("no company: pass -c or set RESTLESS_COMPANY")?;
            // `attach` is the generic door to the company computer. Docker is
            // the V0 Runtime Bridge implementation, not an owner operation.
            // A supplied command is deliberately passed through unchanged;
            // judgement and productive work stay in ordinary Linux.
            let mut args = vec!["exec".to_string()];
            if command.is_empty() {
                args.push("-it".to_string());
            } else {
                args.push("-i".to_string());
            }
            args.extend([
                "-u".to_string(),
                "company".to_string(),
                "-e".to_string(),
                format!("RESTLESS_COMPANY={name}"),
                "-e".to_string(),
                "RESTLESS_ACTOR=owner".to_string(),
                restlessd::appliance::MachineProfile::from_env()?.docker_container_name(&name),
            ]);
            if command.is_empty() {
                args.push("bash".to_string());
            } else {
                args.extend(command);
            }
            let status = std::process::Command::new("docker")
                .args(&args)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()
                .context("docker exec — is docker running?")?;
            if !status.success() {
                bail!("attach failed (is the company up?)");
            }
            Ok(())
        }
        Command::Watch { company: name } => {
            let name = name.context("no company: pass -c or set RESTLESS_COMPANY")?;
            let request = stamp(serde_json::json!({ "cmd": "watch", "company": name }));
            watch(&request.to_string())
        }
        Command::Chat {
            company: name,
            actor,
        } => {
            let name = name.context("no company: pass -c or set RESTLESS_COMPANY")?;
            chat::run(name, actor)
        }
        Command::Doctor { company } => doctor(company),
        Command::Attention { company, summary } => {
            let request = stamp(serde_json::json!({ "cmd": "attention", "company": company }));
            let response = request_once(&request.to_string())?;
            if summary {
                print_attention_summary(&response)
            } else {
                print_response(&response)
            }
        }
        other => {
            let request = stamp(request_json(other)?);
            let response = request_once(&request.to_string())?;
            print_response(&response)
        }
    }
}

/// Every request carries its principal. One place, so a new command cannot
/// forget to — the daemon rejects an unstamped request rather than defaulting
/// one, which makes forgetting loud instead of silently privileged.
fn stamp(mut request: serde_json::Value) -> serde_json::Value {
    if let Some(object) = request.as_object_mut() {
        object.insert("principal".into(), principal().into());
        if is_runtime() {
            if let Some(capability) = runtime_capability() {
                object.insert("session_capability".into(), capability.into());
            }
        }
    }
    request
}

/// One request/response pair; `watch` and `attach` handle their own I/O.
fn request_json(command: Command) -> Result<serde_json::Value> {
    Ok(match command {
        Command::Publish { company, command } => {
            let company = company.context("no company: pass -c or set RESTLESS_COMPANY")?;
            match command {
                PublishCommand::Candidate {
                    source_artifact,
                    manifest,
                    actor,
                } => {
                    let raw = std::fs::read(&manifest)
                        .with_context(|| format!("read {}", manifest.display()))?;
                    let service_manifest: serde_json::Value = serde_json::from_slice(&raw)
                        .with_context(|| format!("parse {}", manifest.display()))?;
                    serde_json::json!({
                        "cmd": "publish-candidate",
                        "company": company,
                        "actor": actor.unwrap_or_else(|| "owner".into()),
                        "source_artifact_ref_id": source_artifact,
                        "service_manifest": service_manifest,
                    })
                }
                PublishCommand::Request {
                    candidate_artifact,
                    audience,
                    start_deadline,
                    expires_at,
                    cpu_millis,
                    memory_mib,
                    ephemeral_storage_mib,
                    max_connections,
                    key,
                    actor,
                } => serde_json::json!({
                    "cmd": "publish-request",
                    "company": company,
                    "actor": actor.unwrap_or_else(|| "owner".into()),
                    "candidate_artifact_ref_id": candidate_artifact,
                    "publication_audience": audience,
                    "publication_start_deadline": start_deadline,
                    "publication_expires_at": expires_at,
                    "cpu_millis": cpu_millis,
                    "memory_mib": memory_mib,
                    "ephemeral_storage_mib": ephemeral_storage_mib,
                    "max_connections": max_connections,
                    "idempotency_key": key,
                }),
                PublishCommand::Authorize { publication } => serde_json::json!({
                    "cmd": "publish-authorize", "company": company, "publication_id": publication,
                }),
                PublishCommand::Invite {
                    publication,
                    id,
                    subject,
                    expires_at,
                } => serde_json::json!({
                    "cmd": "publish-invite",
                    "company": company,
                    "publication_id": publication,
                    "invitation_id": id,
                    "invitee": subject,
                    "publication_expires_at": expires_at,
                }),
                PublishCommand::Revoke { invitation } => serde_json::json!({
                    "cmd": "publish-revoke", "company": company, "invitation_id": invitation,
                }),
                PublishCommand::Observe { publication } => serde_json::json!({
                    "cmd": "publish-observe", "company": company, "publication_id": publication,
                }),
                PublishCommand::Reconcile { publication } => serde_json::json!({
                    "cmd": "publish-reconcile", "company": company, "publication_id": publication,
                }),
                PublishCommand::Stop {
                    publication,
                    reason,
                } => serde_json::json!({
                    "cmd": "publish-stop",
                    "company": company,
                    "publication_id": publication,
                    "stop_reason": reason,
                }),
                PublishCommand::Show { publication } => serde_json::json!({
                    "cmd": "publish-show", "company": company, "publication_id": publication,
                }),
                PublishCommand::List => serde_json::json!({
                    "cmd": "publish-list", "company": company,
                }),
            }
        }
        Command::Company { command } => match command {
            CompanyCommand::Create { from_file } => {
                let body = std::fs::read_to_string(&from_file)
                    .with_context(|| format!("read {}", from_file.display()))?;
                let value: toml::Value = toml::from_str(&body)
                    .with_context(|| format!("parse {}", from_file.display()))?;
                let company = value
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .context("company TOML needs a string name")?;
                serde_json::json!({ "cmd": "company-create", "company": company, "body": body })
            }
            CompanyCommand::Show { company } => {
                serde_json::json!({ "cmd": "company-show", "company": company })
            }
            CompanyCommand::Set {
                company,
                key,
                value,
            } => serde_json::json!({
                "cmd": "company-set", "company": company, "state": key, "body": value,
            }),
            CompanyCommand::Unset { company, key } => serde_json::json!({
                "cmd": "company-unset", "company": company, "state": key,
            }),
            CompanyCommand::List => serde_json::json!({ "cmd": "company-list" }),
        },
        Command::Credential { command } => match command {
            CredentialCommand::Set {
                company,
                binding,
                reference,
                value,
            } => {
                let secret_value = value
                    .map(|source| read_secret_source(&source))
                    .transpose()?;
                serde_json::json!({
                    "cmd": "credential-set", "company": company,
                    "capability": binding, "body": reference,
                    "secret_value": secret_value,
                })
            }
            CredentialCommand::Promote {
                company,
                binding,
                reference,
            } => serde_json::json!({
                "cmd": "credential-promote", "company": company,
                "capability": binding, "body": reference,
            }),
            CredentialCommand::Check { company } => {
                serde_json::json!({ "cmd": "credential-check", "company": company })
            }
        },
        Command::ConnectedTool { company, command } => match command {
            ConnectedToolCommand::List => serde_json::json!({
                "cmd": "connected-tools", "company": company,
            }),
            ConnectedToolCommand::Install {
                name,
                endpoint,
                purpose,
                actor,
                work,
                attempt,
                scopes,
            } => serde_json::json!({
                "cmd": "connected-tool-install", "company": company,
                "tool_name": name, "endpoint": endpoint, "purpose": purpose,
                "assigned_actor": actor, "work_id": work, "attempt_id": attempt,
                "requested_scopes": scopes, "actor": acting_actor(),
            }),
            ConnectedToolCommand::Reconnect {
                name,
                endpoint,
                purpose,
                actor,
                work,
                attempt,
                scopes,
            } => serde_json::json!({
                "cmd": "connected-tool-reconnect", "company": company,
                "tool_name": name, "endpoint": endpoint, "purpose": purpose,
                "assigned_actor": actor, "work_id": work, "attempt_id": attempt,
                "requested_scopes": scopes, "actor": acting_actor(),
            }),
            ConnectedToolCommand::Observe {
                name,
                workspace,
                tools,
            } => serde_json::json!({
                "cmd": "connected-tool-observe", "company": company,
                "tool_name": name, "workspace_reference": workspace,
                "observed_tools": tools, "actor": acting_actor(),
            }),
            ConnectedToolCommand::Disable { name } => serde_json::json!({
                "cmd": "connected-tool-disable", "company": company,
                "tool_name": name, "actor": acting_actor(),
            }),
        },
        Command::Legal { company, command } => match command {
            LegalCommand::Show => serde_json::json!({
                "cmd": "legal-show", "company": company,
            }),
            LegalCommand::Probe => serde_json::json!({
                "cmd": "legal-probe", "company": company,
            }),
            LegalCommand::Set {
                legal_name,
                trading_name,
                entity_type,
                jurisdiction,
                registration_type,
                registration_value,
                business_address,
                invoice_email,
            } => serde_json::json!({
                "cmd": "legal-set", "company": company,
                "body": serde_json::json!({
                    "legal_name": legal_name,
                    "trading_name": trading_name,
                    "entity_type": entity_type,
                    "jurisdiction": jurisdiction,
                    "registration_identifier": {
                        "kind": registration_type,
                        "value": registration_value,
                    },
                    "approved_business_address": business_address,
                    "invoice_email": invoice_email,
                }).to_string(),
            }),
        },
        Command::Finance { company, command } => match command {
            FinanceCommand::Show => serde_json::json!({
                "cmd": "finance-show", "company": company,
            }),
            FinanceCommand::SetEnvelope {
                source_account,
                currency,
                beneficiaries,
                per_payment_minor,
                aggregate_minor,
            } => serde_json::json!({
                "cmd": "finance-envelope-set", "company": company,
                "body": serde_json::json!({
                    "source_account_ref": source_account,
                    "currency": currency,
                    "beneficiary_refs": beneficiaries,
                    "per_payment_limit_minor": per_payment_minor,
                    "aggregate_limit_minor": aggregate_minor,
                    "frozen": false,
                }).to_string(),
            }),
            FinanceCommand::Freeze { currency, unfreeze } => serde_json::json!({
                "cmd": "finance-freeze", "company": company,
                "state": currency, "apply": !unfreeze,
            }),
            FinanceCommand::ConnectAirwallex {
                environment,
                api_version,
                client_id,
                account_ref,
                approval_url,
                read_scopes,
                submit_scopes,
                approval_workflow_observed,
                observed_at,
            } => serde_json::json!({
                "cmd": "finance-connect-airwallex", "company": company,
                "body": serde_json::json!({
                    "environment": environment,
                    "api_version": api_version,
                    "client_id": client_id,
                    "account_ref": account_ref,
                    "approval_url": approval_url,
                    "read_scopes": read_scopes,
                    "submit_scopes": submit_scopes,
                    "approval_workflow_observed": approval_workflow_observed,
                    "observed_at": observed_at,
                }).to_string(),
            }),
            FinanceCommand::Balances => serde_json::json!({
                "cmd": "finance-balances", "company": company,
            }),
            FinanceCommand::Probe => serde_json::json!({
                "cmd": "finance-probe", "company": company,
            }),
            FinanceCommand::Reserve {
                work,
                handoff,
                source_account,
                beneficiary,
                amount_minor,
                currency,
                purpose,
                evidence_refs,
                key,
            } => serde_json::json!({
                "cmd": "finance-reserve", "company": company,
                "body": serde_json::json!({
                    "work_id": work,
                    "owner_handoff_id": handoff,
                    "source_account_ref": source_account,
                    "provider_beneficiary_ref": beneficiary,
                    "amount_minor": amount_minor,
                    "currency": currency,
                    "purpose": purpose,
                    "evidence_refs": evidence_refs,
                    "idempotency_key": key,
                    "requesting_actor": acting_actor(),
                }).to_string(),
            }),
            FinanceCommand::Submit { key } => serde_json::json!({
                "cmd": "finance-submit", "company": company, "key": key,
            }),
            FinanceCommand::Reconcile { key } => serde_json::json!({
                "cmd": "finance-reconcile", "company": company, "key": key,
            }),
        },
        Command::Up {
            company: c,
            from,
            reconcile,
        } => {
            serde_json::json!({
                "cmd": "up",
                "company": c,
                "from_company": from,
                "reconcile": reconcile,
            })
        }
        Command::Down {
            company: c,
            destroy,
        } => {
            serde_json::json!({ "cmd": "down", "company": c, "destroy": destroy })
        }
        Command::Status { company: c } => {
            serde_json::json!({ "cmd": "status", "company": c })
        }
        Command::OrgintelInit { company: c } => {
            serde_json::json!({ "cmd": "orgintel-init", "company": c })
        }
        Command::Identity { company, command } => match command {
            IdentityCommand::Show => serde_json::json!({
                "cmd": "identity-show", "company": company,
            }),
            IdentityCommand::Evidence {
                pillar,
                statement_kind,
                claim,
                statement,
                source,
                authority,
                scope,
                locator,
                polarity,
                status,
                channel,
                audience,
                supersedes,
                expires_at,
                indefinite,
            } => serde_json::json!({
                "cmd": "identity-evidence-add", "company": company,
                "identity_pillar": pillar, "identity_kind": statement_kind,
                "claim_key": claim, "statement": statement,
                "source": source, "identity_authority": authority,
                "scope": scope, "evidence_locator": locator,
                "polarity": polarity, "evidence_status": status,
                "channel": channel, "audience": audience,
                "supersedes": supersedes,
                "exception_expires_at": expires_at,
                "exception_indefinite": indefinite,
                "actor": acting_actor(),
            }),
            IdentityCommand::Propose {
                reason,
                evidence_ids,
            } => serde_json::json!({
                "cmd": "identity-propose", "company": company,
                "reason": reason, "evidence_ids": evidence_ids,
                "actor": acting_actor(),
            }),
            IdentityCommand::Brief {
                outcome,
                channel,
                audience,
                release,
                max_bytes,
            } => serde_json::json!({
                "cmd": "identity-brief", "company": company,
                "body": outcome, "channel": channel, "audience": audience,
                "release_id": release, "max_bytes": max_bytes,
                "actor": acting_actor(),
            }),
            IdentityCommand::VoiceEvidence {
                voice_kind,
                claim,
                statement,
                source,
                authority,
                scope,
                locator,
                judgement,
                polarity,
                channel,
                audience,
                named_author,
                supersedes,
            } => serde_json::json!({
                "cmd": "voice-evidence-add", "company": company,
                "voice_kind": voice_kind, "claim_key": claim, "statement": statement,
                "source": source, "identity_authority": authority, "scope": scope,
                "evidence_locator": locator, "judgement_reason": judgement,
                "polarity": polarity, "channel": channel, "audience": audience,
                "named_author": named_author, "supersedes": supersedes,
                "actor": acting_actor(),
            }),
            IdentityCommand::VoiceBind {
                work,
                channel,
                author,
                audience,
                reader,
                understanding,
                action,
                proof,
                consequence,
            } => serde_json::json!({
                "cmd": "voice-bind", "company": company, "voice_work_id": work,
                "channel": channel, "voice_author": author, "audience": audience, "reader_situation": reader,
                "desired_understanding": understanding, "desired_action": action,
                "proof": proof, "consequence": consequence, "actor": acting_actor(),
            }),
            IdentityCommand::VoiceBrief { work, max_bytes } => serde_json::json!({
                "cmd": "voice-brief", "company": company, "voice_work_id": work,
                "max_bytes": max_bytes, "actor": acting_actor(),
            }),
            IdentityCommand::VoiceRender {
                artifact,
                channel,
                renderer,
                renderer_version,
                checks,
            } => serde_json::json!({
                "cmd": "voice-render", "company": company, "artifact_ref_id": artifact,
                "channel": channel, "renderer": renderer, "renderer_version": renderer_version,
                "semantic_checks": serde_json::from_str::<serde_json::Value>(&checks)
                    .unwrap_or_else(|_| serde_json::Value::String(checks)),
                "actor": acting_actor(),
            }),
            IdentityCommand::VoiceReview {
                render,
                verdict,
                factual,
                abstraction,
                repetition,
                channel,
                authorship,
                concepts_removed,
            } => serde_json::json!({
                "cmd": "voice-review", "company": company, "render_evidence_id": render,
                "review_verdict": verdict, "factual_findings": factual,
                "abstraction_findings": abstraction, "repetition_findings": repetition,
                "channel_findings": channel, "authorship_findings": authorship,
                "concepts_removed": concepts_removed, "actor": acting_actor(),
            }),
            IdentityCommand::VoiceLearn {
                before,
                after,
                learning_kind,
                claim,
                observation,
                decision,
                scope,
                source,
                locator,
                named_author,
                channel,
                audience,
            } => serde_json::json!({
                "cmd": "voice-learn", "company": company,
                "before_artifact_ref_id": before, "after_artifact_ref_id": after,
                "learning_kind": learning_kind, "claim_key": claim,
                "observation": observation, "motivating_decision": decision,
                "scope": scope, "source": source, "evidence_locator": locator,
                "named_author": named_author, "channel": channel, "audience": audience,
                "actor": acting_actor(),
            }),
            IdentityCommand::VisualEvidence {
                visual_kind,
                claim,
                statement,
                source,
                authority,
                scope,
                locator,
                purpose,
                rationale,
                accessibility,
                channel,
                reduced_motion,
                product_truth,
                origin,
                licence,
                framework,
                adaptation,
                dependencies,
                semantic_role,
                value,
                polarity,
            } => serde_json::json!({
                "cmd":"visual-evidence-add", "company":company, "visual_kind":visual_kind,
                "claim_key":claim, "statement":statement, "source":source, "identity_authority":authority,
                "scope":scope, "evidence_locator":locator, "visual_purpose":purpose, "visual_rationale":rationale,
                "accessibility_notes":accessibility, "channel":channel, "reduced_motion_replacement":reduced_motion,
                "product_truth_locator":product_truth, "primitive_origin":origin, "primitive_licence":licence,
                "primitive_framework":framework, "adaptation_status":adaptation,
                "primitive_dependencies":serde_json::from_str::<serde_json::Value>(&dependencies).unwrap_or_else(|_| serde_json::Value::String(dependencies)),
                "semantic_role":semantic_role, "visual_value":value, "polarity":polarity, "actor":acting_actor(),
            }),
            IdentityCommand::VisualBind {
                work,
                channel,
                audience,
                outcome,
                hierarchy,
                proof,
                density,
                imagery,
                motion,
                representation,
                product_truth,
                departure,
            } => serde_json::json!({
                "cmd":"visual-bind", "company":company, "visual_work_id":work, "channel":channel,
                "audience":audience, "body":outcome, "information_hierarchy":hierarchy, "proof":proof,
                "visual_density":density, "imagery_role":imagery, "motion_role":motion,
                "product_representation":representation, "product_truth_locator":product_truth,
                "requested_departure":departure, "actor":acting_actor(),
            }),
            IdentityCommand::VisualBrief { work, max_bytes } => serde_json::json!({
                "cmd":"visual-brief", "company":company, "visual_work_id":work, "max_bytes":max_bytes, "actor":acting_actor(),
            }),
            IdentityCommand::VisualUse {
                work,
                evidence,
                version,
                purpose,
            } => {
                serde_json::json!({"cmd":"visual-use","company":company,"visual_work_id":work,"visual_evidence_id":evidence,"primitive_version":version,"visual_purpose":purpose,"actor":acting_actor()})
            }
            IdentityCommand::VisualRender {
                work,
                artifact,
                channel,
                renderer,
                renderer_version,
                width,
                height,
                motion_state,
                checks,
            } => {
                serde_json::json!({"cmd":"visual-render","company":company,"visual_work_id":work,"artifact_ref_id":artifact,"channel":channel,"renderer":renderer,"renderer_version":renderer_version,"viewport_width":width,"viewport_height":height,"motion_state":motion_state,"semantic_checks":serde_json::from_str::<serde_json::Value>(&checks).unwrap_or_else(|_|serde_json::Value::String(checks)),"actor":acting_actor()})
            }
            IdentityCommand::VisualReview {
                render,
                control,
                verdict,
                identity,
                hierarchy,
                density,
                proof,
                product_fidelity,
                motion,
                defects,
                departure,
            } => {
                serde_json::json!({"cmd":"visual-review","company":company,"render_evidence_id":render,"control_render_evidence_id":control,"review_verdict":verdict,"visual_identity_findings":identity,"hierarchy_findings":hierarchy,"density_findings":density,"proof_findings":proof,"product_fidelity_findings":product_fidelity,"motion_findings":motion,"defect_findings":defects,"departure_decision":departure,"actor":acting_actor()})
            }
            IdentityCommand::CultureEvidence {
                culture_kind,
                case_kind,
                claim,
                statement,
                source,
                authority,
                scope,
                locator,
                situation,
                consequence,
                actors,
                decision_authority,
                conduct,
                observed_outcome,
                confidence,
                counterexample,
                boundary,
                implication,
                actor_scope,
            } => {
                serde_json::json!({"cmd":"culture-evidence-add","company":company,"culture_kind":culture_kind,"culture_case_kind":case_kind,"claim_key":claim,"statement":statement,"source":source,"identity_authority":authority,"scope":scope,"evidence_locator":locator,"culture_situation":situation,"consequence":consequence,"culture_actors":actors,"decision_authority":decision_authority,"observed_conduct":conduct,"observed_outcome":observed_outcome,"culture_confidence":confidence,"counterexample":counterexample,"boundary_conditions":boundary,"operational_implication":implication,"actor_scope":actor_scope,"actor":acting_actor()})
            }
            IdentityCommand::CultureBind {
                work,
                case_kind,
                actor,
                actor_role,
                team,
                consequence,
                decision_boundary,
            } => {
                serde_json::json!({"cmd":"culture-bind","company":company,"culture_work_id":work,"culture_case_kind":case_kind,"culture_actor":actor,"actor_role":actor_role,"team_name":team,"consequence":consequence,"decision_boundary":decision_boundary,"actor":acting_actor()})
            }
            IdentityCommand::CultureBrief { work, max_bytes } => {
                serde_json::json!({"cmd":"culture-brief","company":company,"culture_work_id":work,"max_bytes":max_bytes,"actor":acting_actor()})
            }
            IdentityCommand::CultureCase {
                work,
                artifact,
                case_kind,
                decision,
                alternatives,
                unknowns,
                correction_of,
                correction_account,
                customer_action,
                checks,
            } => {
                serde_json::json!({"cmd":"culture-case","company":company,"culture_work_id":work,"artifact_ref_id":artifact,"culture_case_kind":case_kind,"culture_decision":decision,"culture_alternatives":serde_json::from_str::<serde_json::Value>(&alternatives).unwrap_or_else(|_|serde_json::Value::String(alternatives)),"culture_unknowns":unknowns,"correction_of":correction_of,"correction_account":correction_account,"customer_action":customer_action,"semantic_checks":serde_json::from_str::<serde_json::Value>(&checks).unwrap_or_else(|_|serde_json::Value::String(checks)),"actor":acting_actor()})
            }
            IdentityCommand::CultureReview {
                record,
                verdict,
                conduct,
                dissent,
                uncertainty,
                correction,
                authority,
                customer_or_hiring,
                slogan_recitation,
            } => {
                serde_json::json!({"cmd":"culture-review","company":company,"culture_case_record_id":record,"review_verdict":verdict,"conduct_findings":conduct,"dissent_findings":dissent,"uncertainty_findings":uncertainty,"correction_findings":correction,"authority_findings":authority,"customer_or_hiring_findings":customer_or_hiring,"slogan_recitation_detected":slogan_recitation,"actor":acting_actor()})
            }
        },
        Command::Attention { company: c, .. } => {
            serde_json::json!({ "cmd": "attention", "company": c })
        }
        Command::Browser { command } => match command {
            BrowserCommand::Status { company } => {
                serde_json::json!({ "cmd": "browser-status", "company": company })
            }
            BrowserCommand::Request { company, session } => serde_json::json!({
                "cmd": "browser-request", "company": company,
                "id": session.unwrap_or_else(|| {
                    if is_runtime() {
                        acting_actor()
                    } else {
                        "exec".to_string()
                    }
                }),
            }),
            BrowserCommand::Release { company } => {
                serde_json::json!({ "cmd": "browser-release", "company": company })
            }
        },
        Command::Wake { company: c, reason } => {
            serde_json::json!({ "cmd": "wake", "company": c, "reason": reason })
        }
        Command::Tell { company: c, body } => {
            serde_json::json!({ "cmd": "tell", "company": c, "body": body })
        }
        Command::People {
            company: c,
            include_retired,
            command,
        } => match command {
            None => serde_json::json!({
                "cmd": "people", "company": c, "include_retired": include_retired,
            }),
            Some(PeopleCommand::Create {
                id,
                role,
                display,
                model,
                reason,
            }) => serde_json::json!({
                "cmd": "actor-create", "company": c, "as_actor": id, "role": role,
                "name": display, "model": model, "reason": reason,
                "actor": acting_actor(),
            }),
            Some(PeopleCommand::Model {
                actor,
                model,
                reason,
            }) => serde_json::json!({
                "cmd": "actor-model", "company": c, "as_actor": actor,
                "model": model, "reason": reason, "actor": acting_actor(),
            }),
            Some(PeopleCommand::Retire { actor, reason }) => serde_json::json!({
                "cmd": "actor-retire", "company": c, "as_actor": actor, "reason": reason,
                "actor": acting_actor(),
            }),
        },
        Command::Judgement {
            company: c,
            as_actor,
        } => serde_json::json!({ "cmd": "judgement", "company": c, "as_actor": as_actor }),
        Command::Teams { command } => match command {
            TeamCommand::List { company } => {
                serde_json::json!({ "cmd": "teams", "company": company })
            }
            TeamCommand::Create {
                company,
                name,
                lead,
                brief,
                standard,
                standard_source,
                source_message,
            } => serde_json::json!({
                "cmd": "team-create", "company": company, "name": name, "to": lead,
                "body": brief,
                "outcome_standard": standard,
                "outcome_standard_source": standard_source,
                "source_message_id": source_message,
                "actor": acting_actor(),
            }),
            TeamCommand::Update {
                company,
                team,
                name,
                brief,
                reason,
            } => serde_json::json!({
                "cmd": "team-update", "company": company, "name": team,
                "new_name": name, "body": brief, "reason": reason,
                "actor": acting_actor(),
            }),
            TeamCommand::Assign {
                company,
                actor,
                team,
                reason,
            } => serde_json::json!({
                "cmd": "team-assign", "company": company, "as_actor": actor, "name": team,
                "reason": reason,
                "actor": acting_actor(),
            }),
            TeamCommand::Lead {
                company,
                team,
                actor,
                reason,
            } => serde_json::json!({
                "cmd": "team-lead", "company": company, "name": team, "to": actor,
                "reason": reason,
                "actor": acting_actor(),
            }),
            TeamCommand::Disband {
                company,
                team,
                reason,
            } => serde_json::json!({
                "cmd": "team-disband", "company": company, "name": team, "reason": reason,
                "actor": acting_actor(),
            }),
        },
        Command::Spend { company: c } => serde_json::json!({ "cmd": "spend", "company": c }),
        Command::Telemetry { company: c } => {
            serde_json::json!({ "cmd": "telemetry", "company": c })
        }
        Command::SpendCorrect {
            company,
            correction_id,
            request_ids,
            delta_micro_usd,
            reason,
            apply,
        } => serde_json::json!({
            "cmd": "spend-correct", "company": company, "correction_id": correction_id,
            "request_ids": request_ids, "delta_micro_usd": delta_micro_usd,
            "reason": reason, "apply": apply,
        }),
        Command::Receipts {
            company: c,
            effect_class,
            limit,
        } => serde_json::json!({
            "cmd": "receipts", "company": c, "capability": effect_class, "limit": limit,
        }),
        Command::Goal { company, command } => match command {
            None | Some(GoalCommand::List) => {
                serde_json::json!({ "cmd": "goals", "company": company })
            }
            Some(GoalCommand::Add { title, body }) => serde_json::json!({
                "cmd": "goal-add", "company": company, "title": title, "body": body,
                "actor": acting_actor(),
            }),
            Some(GoalCommand::Attach { work, goal }) => serde_json::json!({
                "cmd": "work-goal", "company": company, "id": work, "goal": goal,
                "actor": acting_actor(),
            }),
        },
        Command::Work { command } => match command {
            WorkCommand::List { company } => {
                serde_json::json!({ "cmd": "work", "company": company })
            }
            WorkCommand::Graph { company } => {
                serde_json::json!({ "cmd": "work-graph", "company": company })
            }
            WorkCommand::Attempts { company, work } => serde_json::json!({
                "cmd": "work-attempts", "company": company, "id": work,
            }),
            WorkCommand::Add {
                company,
                owner,
                role,
                model,
                topology,
                title,
                outcome,
                priority,
                expected_artifact,
                repo,
                base_ref,
                integration_branch,
                worktree,
                attempt_limit,
                owner_review,
                goal,
                source_message,
                requires,
                revises,
                gate,
                constitution_contracts,
            } => {
                let gates = gate
                    .iter()
                    .map(|value| {
                        serde_json::from_str::<serde_json::Value>(value)
                            .with_context(|| format!("invalid --gate JSON {value:?}"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let constitution_contracts = constitution_contracts
                    .as_deref()
                    .map(|value| {
                        serde_json::from_str::<serde_json::Value>(value)
                            .context("invalid --constitution-contracts JSON")
                    })
                    .transpose()?;
                serde_json::json!({
                    "cmd": "work-add", "company": company, "actor": owner, "role": role,
                    "model": model, "producing_topology": topology, "title": title,
                    "body": outcome, "priority": priority,
                    "expected_artifact": expected_artifact, "repo": repo, "base_ref": base_ref,
                    "integration_branch": integration_branch, "worktree": worktree,
                    "attempt_limit": attempt_limit, "owner_review": owner_review, "goal": goal,
                    "source_message_id": source_message,
                    "requires": requires, "revises": revises, "gates": gates,
                    "constitution_contracts": constitution_contracts,
                    "as_actor": acting_actor(),
                })
            }
            WorkCommand::Assign {
                company,
                work,
                owner,
                reason,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-assign", "company": company, "id": work, "to": owner,
                "reason": reason, "actor": as_actor.unwrap_or_else(acting_actor),
            }),
            WorkCommand::Edge {
                company,
                from,
                to,
                kind,
                remove,
                reason,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-edge", "company": company, "from": from, "to": to, "kind": kind,
                "action": if remove { "remove" } else { "add" },
                "reason": reason, "as_actor": as_actor,
            }),
            WorkCommand::Artifact {
                company,
                work,
                attempt,
                kind,
                uri,
                digest,
                source_commit,
                label,
                note,
            } => serde_json::json!({
                "cmd": "work-artifact", "company": company, "id": work, "attempt": attempt,
                "kind": kind, "uri": uri, "digest": digest, "source_commit": source_commit,
                "label": label, "body": note,
                "actor": std::env::var("RESTLESS_ACTOR").unwrap_or_else(|_| "owner".to_string()),
            }),
            WorkCommand::RetireArtifact {
                company,
                artifact,
                reason,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-artifact-retire", "company": company, "id": artifact,
                "reason": reason, "actor": as_actor.unwrap_or_else(acting_actor),
            }),
            WorkCommand::Gate {
                company,
                work,
                name,
                cwd,
                stage,
                timeout_seconds,
                resources,
                command,
            } => serde_json::json!({
                "cmd": "work-gate", "company": company, "id": work, "name": name,
                "cwd": cwd, "argv": command, "stage": stage,
                "timeout_seconds": timeout_seconds, "resources": resources,
                "actor": std::env::var("RESTLESS_ACTOR").unwrap_or_else(|_| "owner".to_string()),
            }),
            WorkCommand::RetireGate {
                company,
                gate,
                reason,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-gate-retire", "company": company, "id": gate,
                "reason": reason,
                "as_actor": as_actor.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                    .unwrap_or_else(|| "owner".to_string()),
            }),
            WorkCommand::Handoff {
                company,
                work,
                attempt,
                category,
                action,
                prepared,
                resume_when,
            } => serde_json::json!({
                "cmd": "work-handoff", "company": company, "id": work, "attempt": attempt,
                "category": category, "action": action, "prepared": prepared,
                "resume_when": resume_when,
                "actor": std::env::var("RESTLESS_ACTOR").unwrap_or_else(|_| "owner".to_string()),
            }),
            WorkCommand::RefreshHandoff {
                company,
                handoff,
                action,
                prepared,
                resume_when,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-handoff-refresh", "company": company, "id": handoff,
                "action": action, "prepared": prepared, "resume_when": resume_when,
                "as_actor": as_actor.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                    .unwrap_or_else(|| "owner".to_string()),
            }),
            WorkCommand::PrepareOwnerBrief {
                company,
                handoff,
                kind,
                headline,
                situation,
                impact,
                recommendation,
                no_action,
                uncertainty,
                deadline,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-handoff-prepare-brief", "company": company, "id": handoff,
                "owner_kind": kind, "headline": headline, "situation": situation,
                "impact": impact, "recommendation": recommendation, "no_action": no_action,
                "uncertainty": uncertainty, "deadline": deadline,
                "as_actor": as_actor.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                    .unwrap_or_else(|| "owner".to_string()),
            }),
            WorkCommand::EscalateHandoff {
                company,
                handoff,
                as_actor,
                reason,
            } => serde_json::json!({
                "cmd": "work-handoff-escalate", "company": company, "id": handoff,
                "as_actor": as_actor, "reason": reason,
            }),
            WorkCommand::ResolveHandoff {
                company,
                handoff,
                state,
                resolution,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-handoff-resolve", "company": company, "id": handoff, "state": state,
                "resolution": resolution,
                "as_actor": as_actor.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                    .unwrap_or_else(|| "owner".to_string()),
            }),
            WorkCommand::Resume {
                company,
                work,
                reason,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-resume", "company": company, "id": work, "reason": reason,
                "as_actor": as_actor.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                    .unwrap_or_else(|| "owner".to_string()),
            }),
            WorkCommand::Interrupt {
                company,
                work,
                reason,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-interrupt", "company": company, "id": work, "reason": reason,
                "as_actor": as_actor.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                    .unwrap_or_else(|| "owner".to_string()),
            }),
            WorkCommand::Abandon {
                company,
                work,
                reason,
                as_actor,
            } => serde_json::json!({
                "cmd": "work-abandon", "company": company, "id": work, "reason": reason,
                "as_actor": as_actor.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                    .unwrap_or_else(|| "owner".to_string()),
            }),
            WorkCommand::Review {
                company,
                handoff,
                decision,
                feedback,
            } => serde_json::json!({
                "cmd": "work-review", "company": company, "id": handoff, "state": decision,
                "resolution": feedback,
            }),
        },
        Command::Schedule { command } => match command {
            ScheduleCommand::List {
                company,
                as_actor,
                all,
            } => serde_json::json!({
                "cmd": "schedule-list", "company": company,
                "as_actor": as_actor.or_else(|| std::env::var("RESTLESS_ACTOR").ok()),
                "include_fired": all,
            }),
            ScheduleCommand::History {
                company,
                schedule,
                limit,
            } => serde_json::json!({
                "cmd": "schedule-history", "company": company,
                "id": schedule, "limit": limit,
            }),
            ScheduleCommand::Recover {
                company,
                schedule,
                scheduled_for,
                as_actor,
                requested_by,
                reason,
            } => serde_json::json!({
                "cmd": "schedule-recover", "company": company,
                "id": schedule, "fire_at": scheduled_for,
                "as_actor": as_actor, "from": requested_by, "reason": reason,
            }),
            ScheduleCommand::RetryRecovery {
                company,
                schedule,
                scheduled_for,
                as_actor,
                prior_message,
                key,
                requested_by,
                reason,
            } => serde_json::json!({
                "cmd": "schedule-retry-recovery", "company": company,
                "id": schedule, "fire_at": scheduled_for,
                "as_actor": as_actor, "prior_message_id": prior_message,
                "retry_key": key, "from": requested_by, "reason": reason,
            }),
            ScheduleCommand::Add {
                company,
                as_actor,
                at,
                weekdays,
                at_local,
                timezone,
                on_missed,
                catch_up_within_minutes,
                execution,
                reason,
                work,
            } => serde_json::json!({
                "cmd": "schedule-add", "company": company,
                "as_actor": as_actor.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                    .unwrap_or_else(|| "exec".to_string()),
                "fire_at": at, "recurrence": weekdays.then_some("weekdays"),
                "local_time": at_local, "timezone": timezone,
                "missed_policy": on_missed,
                "catch_up_grace_seconds": catch_up_within_minutes.map(|minutes| minutes.saturating_mul(60)),
                "execution_requirement": execution,
                "reason": reason, "id": work,
            }),
            ScheduleCommand::Policy {
                company,
                schedule,
                as_actor,
                on_missed,
                catch_up_within_minutes,
            } => serde_json::json!({
                "cmd": "schedule-policy", "company": company, "id": schedule,
                "as_actor": as_actor, "missed_policy": on_missed,
                "catch_up_grace_seconds": catch_up_within_minutes.map(|minutes| minutes.saturating_mul(60)),
            }),
            ScheduleCommand::Cancel {
                company,
                schedule,
                as_actor,
                reason,
            } => serde_json::json!({
                "cmd": "schedule-cancel", "company": company, "id": schedule,
                "as_actor": as_actor, "reason": reason,
            }),
        },
        Command::Events { company: c, limit } => {
            serde_json::json!({ "cmd": "events", "company": c, "limit": limit })
        }
        Command::Inbox {
            company: c,
            as_actor,
        } => {
            // A company actor reading its own inbox is a real observation,
            // not host-side inspection: the daemon can then attach any
            // Work-linked mail it actually received to its one live Attempt.
            // `--as` remains an observer spelling on the host, and stays one
            // inside a container when it names someone else.
            let actor = (principal() == "company/exec").then(acting_actor);
            serde_json::json!({
                "cmd": "inbox", "company": c, "as_actor": as_actor, "actor": actor,
            })
        }
        Command::ClearPoison { company: c } => serde_json::json!({
            "cmd": "clear-poison",
            "company": c,
        }),
        Command::Approve {
            company: c,
            party,
            revoke,
            decline,
        } => serde_json::json!({
            "cmd": if revoke { "revoke" } else if decline { "decline" } else { "approve" },
            "company": c,
            "party": party,
        }),
        Command::Message {
            company: c,
            from,
            to,
            work,
            body,
        } => {
            require_message_recipient_in_coordination_wake(
                is_team_coordination_wake(),
                to.as_deref(),
            )?;
            serde_json::json!({
                "cmd": "message",
                "company": c,
                "from": from.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                    .unwrap_or_else(|| "owner".to_string()),
                "to": to,
                "id": work,
                "body": body,
            })
        }
        Command::Effect {
            company: c,
            effect_class,
            party,
            purpose,
            artifacts,
            secrets,
            cwd,
            key,
            command,
        } => {
            let secret_bindings = secrets
                .into_iter()
                .map(|binding| {
                    binding
                        .split_once('=')
                        .map(|(name, reference)| {
                            (
                                name.to_string(),
                                serde_json::Value::String(reference.to_string()),
                            )
                        })
                        .context("--secret must be ENV_NAME=binding")
                })
                .collect::<Result<serde_json::Map<String, serde_json::Value>>>()?;
            serde_json::json!({
                "cmd": "effect",
                "company": c,
                "effect_class": effect_class,
                "party": party,
                "purpose": purpose,
                "artifacts": artifacts,
                "secret_bindings": secret_bindings,
                "cwd": cwd,
                "argv": command,
                "key": key,
                "actor": std::env::var("RESTLESS_ACTOR").unwrap_or_else(|_| "owner".to_string()),
            })
        }
        Command::EffectReconcile {
            company,
            key,
            execution,
            result,
            evidence_receipt,
        } => serde_json::json!({
            "cmd": "effect-reconcile",
            "company": company,
            "key": key,
            "execution_no": execution,
            "state": result,
            "id": evidence_receipt,
            "actor": std::env::var("RESTLESS_ACTOR").unwrap_or_else(|_| "owner".to_string()),
        }),
        Command::Appliance { .. }
        | Command::Open
        | Command::Doctor { .. }
        | Command::Watch { .. }
        | Command::Attach { .. }
        | Command::Chat { .. }
        | Command::EffectChild => {
            unreachable!("handled above")
        }
    })
}

fn read_secret_source(source: &str) -> Result<String> {
    if source == "-" {
        let mut value = String::new();
        std::io::stdin()
            .read_to_string(&mut value)
            .context("read secret value from stdin")?;
        return Ok(value);
    }
    let path = source
        .strip_prefix('@')
        .filter(|path| !path.is_empty())
        .context("--value accepts only @<file> or - (stdin), never raw secret material")?;
    std::fs::read_to_string(path).with_context(|| format!("read secret value from {path}"))
}

fn print_response(response: &str) -> Result<()> {
    let data = response_data(response)?;
    match data {
        serde_json::Value::String(message) => println!("{message}"),
        other => println!("{}", serde_json::to_string_pretty(&other)?),
    }
    Ok(())
}

/// `attention` is the one unified *read* surface for owner control. The
/// lines below deliberately point at the already-typed source commands;
/// there is no `attention act` endpoint and no universal mutation algebra.
fn print_attention_summary(response: &str) -> Result<()> {
    let data = response_data(response)?;
    println!("{}", render_attention_summary(&data));
    Ok(())
}

fn render_attention_summary(data: &serde_json::Value) -> String {
    let company = data
        .pointer("/company/id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<unknown-company>");
    let display_company = data
        .pointer("/company/name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(company);
    let items = data
        .get("items")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let mut lines = vec![format!("Attention · {display_company}")];
    let source_health = ["authority", "orgintel", "runtime", "browser"]
        .iter()
        .filter_map(|source| {
            data.pointer(&format!("/source_health/{source}"))
                .and_then(serde_json::Value::as_str)
                .map(|status| format!("{source}={status}"))
        })
        .collect::<Vec<_>>();
    if !source_health.is_empty() {
        lines.push(format!("Sources: {}", source_health.join(" · ")));
    }

    if items.is_empty() {
        lines.push("No owner attention is currently projected.".into());
        return lines.join("\n");
    }

    lines.push(format!(
        "{} {} {} your attention.",
        items.len(),
        if items.len() == 1 { "item" } else { "items" },
        if items.len() == 1 { "needs" } else { "need" }
    ));
    for (index, item) in items.iter().enumerate() {
        let category = item["category"].as_str().unwrap_or("attention");
        let title = item["title"].as_str().unwrap_or("Untitled owner request");
        lines.push(String::new());
        lines.push(format!(
            "{}. {} · {title}",
            index + 1,
            category.replace('_', " ")
        ));

        let plane = item
            .pointer("/source/plane")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let kind = item
            .pointer("/source/kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let reference = item
            .pointer("/source/reference")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        lines.push(format!("   Source: {plane} / {kind} / {reference}"));
        if let Some(recommendation) = compact_attention_text(item["recommendation"].as_str()) {
            lines.push(format!("   Recommendation: {recommendation}"));
        }
        if let Some(action) = compact_attention_text(item["requested_action"].as_str()) {
            lines.push(format!("   Requested: {action}"));
        }
        if let Some(wait) = compact_attention_text(item["if_no_action"].as_str()) {
            lines.push(format!("   If you wait: {wait}"));
        }
        if let Some(deadline) = compact_attention_text(item["deadline"].as_str()) {
            lines.push(format!("   Deadline: {deadline}"));
        }

        let actions = item
            .get("actions")
            .and_then(serde_json::Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if !actions.is_empty() {
            lines.push("   Controls:".into());
            for action in actions {
                lines.push(format!(
                    "   - {}",
                    attention_control_instruction(company, item, action)
                ));
            }
        }
    }
    lines.join("\n")
}

fn compact_attention_text(value: Option<&str>) -> Option<String> {
    value
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|text| !text.is_empty())
}

fn attention_control_instruction(
    company: &str,
    item: &serde_json::Value,
    action: &serde_json::Value,
) -> String {
    let label = action["label"].as_str().unwrap_or("Act");
    if let Some(href) = action["href"].as_str().filter(|href| !href.is_empty()) {
        return format!("{label}: open {href} in your normal browser");
    }

    let reference = item
        .pointer("/source/reference")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<unknown-handoff>");
    let party = item
        .pointer("/source/party")
        .and_then(serde_json::Value::as_str);
    let work_id = item["work_id"].as_str();
    let responsible_actor = item
        .pointer("/responsible_actor/id")
        .and_then(serde_json::Value::as_str);
    let command_company = shell_quote(company);
    let command = match action["id"].as_str() {
        Some("grant") => party.map(|party| {
            format!(
                "restless approve -c {command_company} --party {}",
                shell_quote(party)
            )
        }),
        Some("decline") => party.map(|party| {
            format!(
                "restless approve -c {command_company} --party {} --decline",
                shell_quote(party)
            )
        }),
        Some("accept-review") => Some(format!(
            "restless work review -c {command_company} --handoff {} --decision accept",
            shell_quote(reference)
        )),
        Some("request-revision") => Some(format!(
            "restless work review -c {command_company} --handoff {} --decision request_changes --feedback '<your exact feedback>'",
            shell_quote(reference)
        )),
        Some("record-decision") => Some(format!(
            "restless work resolve-handoff -c {command_company} --handoff {} --state resolved --resolution '<your decision>'",
            shell_quote(reference)
        )),
        Some("chat-lead") => responsible_actor.zip(work_id).map(|(actor, work)| {
            format!(
                "restless message -c {command_company} --to {} --work {} '<your message>'",
                shell_quote(actor),
                shell_quote(work)
            )
        }),
        Some("open-outcome") => {
            Some("open the prepared native review target in the Cockpit before deciding".into())
        }
        _ => None,
    };

    command.map_or_else(
        || {
            let consequence = compact_attention_text(action["consequence"].as_str())
                .unwrap_or_else(|| "inspect the source item in the Cockpit".into());
            format!("{label}: {consequence}")
        },
        |command| format!("{label}: {command}"),
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn response_data(response: &str) -> Result<serde_json::Value> {
    let parsed: serde_json::Value = serde_json::from_str(response).context("parse response")?;
    if parsed["ok"].as_bool() == Some(true) {
        Ok(parsed["data"].clone())
    } else {
        bail!("{}", render_error(&parsed["error"]))
    }
}

/// `doctor` is deliberately partly client-side. A diagnostic implemented only
/// as a daemon RPC cannot distinguish a broken daemon from a broken company.
/// The daemon remains authoritative for company state; this client probes the
/// host boundaries needed to reach that state and combines both reports.
fn doctor(company: Option<String>) -> Result<()> {
    let company = company.context("no company: pass -c or set RESTLESS_COMPANY")?;
    let runtime = daemon_request(serde_json::json!({
        "cmd": "doctor",
        "company": company,
    }));

    let orgintel = if runtime.is_ok() {
        daemon_request(serde_json::json!({
            "cmd": "attention",
            "company": company,
        }))
    } else {
        Err(anyhow!("coordinator unavailable"))
    };

    let owner_url =
        std::env::var("RESTLESS_OWNER_URL").unwrap_or_else(|_| "http://127.0.0.1:7788".to_string());
    let cockpit_url = std::env::var("RESTLESS_COCKPIT_URL").unwrap_or_else(|_| owner_url.clone());

    // A Runtime client reaches the host through the authenticated coordinator
    // bridge. Its loopback is the company container, not the owner's machine,
    // so probing the owner gateway there manufactures a false outage and can
    // send Staff into an impossible `restless-dev` repair loop.
    let runtime_client = std::env::var_os("RESTLESS_COORDINATOR").is_some();
    let not_observed_from_runtime = || {
        (
            serde_json::json!({
                "status": "not_observed",
                "detail": "host-only owner boundary; run doctor from the owner machine to probe it",
            }),
            true,
        )
    };
    let (owner_gateway, owner_gateway_ok) = if runtime_client {
        not_observed_from_runtime()
    } else {
        http_check(
            &owner_url,
            "/api/companies",
            &[200, 401],
            "owner API must answer (an unauthenticated 401 is healthy)",
        )
    };
    let (cockpit_shell, cockpit_shell_ok) = if runtime_client {
        not_observed_from_runtime()
    } else {
        http_check(
            &cockpit_url,
            &format!("/{company}"),
            &[200],
            "cockpit shell must render",
        )
    };
    let (cockpit_api, cockpit_api_ok) = if runtime_client {
        not_observed_from_runtime()
    } else {
        http_check(
            &cockpit_url,
            "/api/companies",
            &[200, 401],
            "cockpit same-origin API path must reach the owner gateway",
        )
    };
    let (storage, storage_ok) = storage_check();

    let coordinator_ok = runtime.is_ok();
    let orgintel_ok = orgintel.is_ok();
    let runtime_ok = runtime.as_ref().is_ok_and(runtime_report_is_live);
    let all_live = coordinator_ok
        && orgintel_ok
        && runtime_ok
        && owner_gateway_ok
        && cockpit_shell_ok
        && cockpit_api_ok
        && storage_ok;

    let mut actions = Vec::new();
    if !runtime_client
        && (!coordinator_ok || !owner_gateway_ok || !cockpit_shell_ok || !cockpit_api_ok)
    {
        actions.push(format!("restless-dev {company}"));
    }
    if !orgintel_ok {
        actions.push(format!("restless up -c {company}"));
    }
    if let Ok(report) = &runtime {
        if report.get("volume_exists").is_none() || report.get("volume_mounted").is_none() {
            actions.push(format!(
                "restart restlessd with the current build (stop the old stack, then run restless-dev {company})"
            ));
        }
        if report.get("coordination").is_none() {
            actions.push(format!(
                "restart restlessd with the current build (stop the old stack, then run restless-dev {company})"
            ));
        }
        if let Some(action) = report["action"].as_str() {
            actions.push(action.to_string());
        }
        if report.get("supervisor").is_some()
            && report["supervisor"]["status"].as_str() != Some("available")
        {
            actions.push(format!(
                "restless attach -c {company} -- supervisorctl -c /etc/supervisor/conf.d/restless.conf status"
            ));
        }
    } else {
        actions.push("start restlessd".to_string());
    }
    if !storage_ok {
        actions.push("follow docs/BUILD_STORAGE.md before another full build".to_string());
    }
    actions.sort();
    actions.dedup();

    let coordinator = match &runtime {
        Ok(report) => serde_json::json!({
            "status": "available",
            "runtime": report,
        }),
        Err(error) => serde_json::json!({
            "status": "unavailable",
            "error": format!("{error:#}"),
        }),
    };
    let orgintel = match orgintel {
        Ok(_) => serde_json::json!({ "status": "available" }),
        Err(error) => serde_json::json!({
            "status": "unavailable",
            "error": format!("{error:#}"),
        }),
    };
    let report = serde_json::json!({
        "company": company,
        "status": if all_live { "live" } else { "degraded" },
        "checks": {
            "coordinator": coordinator,
            "orgintel": orgintel,
            "owner_gateway": owner_gateway,
            "cockpit_shell": cockpit_shell,
            "cockpit_api": cockpit_api,
            "storage": storage,
        },
        "actions": actions,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    if all_live {
        Ok(())
    } else {
        bail!("local stack is degraded; run the reported action(s)")
    }
}

fn daemon_request(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = stamp(request);
    let response = request_once(&request.to_string())?;
    response_data(&response)
}

fn runtime_report_is_live(report: &serde_json::Value) -> bool {
    report["container"].as_str() == Some("Running")
        && report["reconciliation"].as_str() == Some("current")
        && report["supervisor"]["status"].as_str() == Some("available")
        && report["browser"]["status"].as_str() == Some("available")
        && report["volume_exists"].as_bool() == Some(true)
        && report["volume_mounted"].as_bool() == Some(true)
        && report["coordination"]["status"].as_str() == Some("available")
}

fn http_check(
    base_url: &str,
    path: &str,
    accepted: &[u16],
    expectation: &str,
) -> (serde_json::Value, bool) {
    match http_status(base_url, path) {
        Ok(code) if accepted.contains(&code) => (
            serde_json::json!({
                "status": "available",
                "url": format!("{}{}", base_url.trim_end_matches('/'), path),
                "http_status": code,
            }),
            true,
        ),
        Ok(code) => (
            serde_json::json!({
                "status": "unavailable",
                "url": format!("{}{}", base_url.trim_end_matches('/'), path),
                "http_status": code,
                "expected": expectation,
            }),
            false,
        ),
        Err(error) => (
            serde_json::json!({
                "status": "unavailable",
                "url": format!("{}{}", base_url.trim_end_matches('/'), path),
                "error": format!("{error:#}"),
                "expected": expectation,
            }),
            false,
        ),
    }
}

fn http_status(base_url: &str, path: &str) -> Result<u16> {
    let rest = base_url
        .strip_prefix("http://")
        .context("local health probes support only http:// URLs")?;
    let (authority, prefix) = rest.split_once('/').unwrap_or((rest, ""));
    let endpoint = if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{authority}:80")
    };
    let mut last_error = None;
    let mut stream = None;
    for address in endpoint
        .to_socket_addrs()
        .with_context(|| format!("resolve {endpoint}"))?
    {
        match TcpStream::connect_timeout(&address, Duration::from_secs(2)) {
            Ok(connected) => {
                stream = Some(connected);
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let mut stream = stream.ok_or_else(|| {
        last_error
            .map(anyhow::Error::from)
            .unwrap_or_else(|| anyhow!("{endpoint} resolved to no addresses"))
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    let prefix = prefix.trim_matches('/');
    let path = path.trim_start_matches('/');
    let request_path = if prefix.is_empty() {
        format!("/{path}")
    } else {
        format!("/{prefix}/{path}")
    };
    stream.write_all(
        format!("GET {request_path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    )?;
    let mut status_line = String::new();
    BufReader::new(stream).read_line(&mut status_line)?;
    status_line
        .split_whitespace()
        .nth(1)
        .context("HTTP response had no status code")?
        .parse::<u16>()
        .context("parse HTTP status code")
}

fn storage_check() -> (serde_json::Value, bool) {
    let output = std::process::Command::new("df")
        .arg("-Pk")
        .arg(state_root())
        .output();
    let available_kib = output
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|stdout| {
            stdout
                .lines()
                .last()
                .and_then(|line| line.split_whitespace().nth(3))
                .and_then(|value| value.parse::<u64>().ok())
        });
    let minimum_kib = 30 * 1024 * 1024;
    match available_kib {
        Some(value) => (
            serde_json::json!({
                "status": if value >= minimum_kib { "available" } else { "low" },
                "available_gib": value / 1024 / 1024,
                "minimum_gib": 30,
            }),
            value >= minimum_kib,
        ),
        None => (
            serde_json::json!({
                "status": "unknown",
                "error": "could not read host disk headroom with df",
            }),
            false,
        ),
    }
}

/// Errors are `{kind, message}` (S04-T10). The kind is shown, not swallowed:
/// an owner who sees `[authority]` knows to change what they are allowed to do,
/// and one who sees `[transport]` knows to check the daemon — the distinction
/// prose cannot carry reliably.
fn render_error(error: &serde_json::Value) -> String {
    let message = error["message"].as_str().unwrap_or("unknown error");
    match error["kind"].as_str() {
        Some(kind) if kind != "error" => format!("[{kind}] {message}"),
        _ => message.to_string(),
    }
}

/// Stream the event log: one rendered line per event until the daemon goes
/// away or the owner interrupts.
fn watch(line: &str) -> Result<()> {
    let mut stream = connect()?;
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut reader = BufReader::new(stream);
    let mut event = String::new();
    loop {
        event.clear();
        if reader.read_line(&mut event)? == 0 {
            return Ok(()); // daemon closed
        }
        let parsed: serde_json::Value = match serde_json::from_str(event.trim()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if parsed["ok"].as_bool() == Some(false) {
            bail!("{}", render_error(&parsed["error"]));
        }
        let at = parsed["created_at"].as_str().unwrap_or("");
        let kind = parsed["kind"].as_str().unwrap_or("?");
        let actor = parsed["actor_id"].as_str().unwrap_or("-");
        println!("{at} {kind:<20} {actor:<12} {}", parsed["body"]);
    }
}

enum Stream {
    Unix(UnixStream),
    Tcp(std::net::TcpStream),
    WebSocket(WebSocketLineStream),
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Unix(stream) => stream.write(buf),
            Stream::Tcp(stream) => stream.write(buf),
            Stream::WebSocket(stream) => stream.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Unix(stream) => stream.flush(),
            Stream::Tcp(stream) => stream.flush(),
            Stream::WebSocket(stream) => stream.flush(),
        }
    }
}

impl std::io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Unix(stream) => stream.read(buf),
            Stream::Tcp(stream) => stream.read(buf),
            Stream::WebSocket(stream) => stream.read(buf),
        }
    }
}

type BlockingWebSocket =
    tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>;

/// Adapt one-JSON-object WebSocket text frames to the CLI's established
/// newline-delimited synchronous stream. The capability stays inside the JSON
/// request; neither the URL nor the upgrade headers carry session authority.
struct WebSocketLineStream {
    socket: BlockingWebSocket,
    incoming: Vec<u8>,
    incoming_offset: usize,
    outgoing: Vec<u8>,
    closed: bool,
}

impl WebSocketLineStream {
    fn new(socket: BlockingWebSocket) -> Self {
        Self {
            socket,
            incoming: Vec::new(),
            incoming_offset: 0,
            outgoing: Vec::new(),
            closed: false,
        }
    }

    fn send_outgoing(&mut self) -> std::io::Result<()> {
        let bytes = std::mem::take(&mut self.outgoing);
        let text = String::from_utf8(bytes).map_err(|_| invalid_websocket_data())?;
        validate_coordination_text(&text)?;
        self.socket
            .send(tungstenite::Message::Text(text.into()))
            .map_err(websocket_io_error)
    }

    fn receive_next(&mut self) -> std::io::Result<bool> {
        loop {
            match self.socket.read() {
                Ok(tungstenite::Message::Text(text)) => {
                    validate_coordination_text(text.as_str())?;
                    self.incoming.clear();
                    self.incoming.extend_from_slice(text.as_bytes());
                    self.incoming.push(b'\n');
                    self.incoming_offset = 0;
                    return Ok(true);
                }
                Ok(tungstenite::Message::Ping(_)) => {
                    // tungstenite queues the standards-required Pong while
                    // reading a Ping. Flush it before waiting for application
                    // data so a quiet watch connection remains healthy.
                    self.socket.flush().map_err(websocket_io_error)?;
                }
                Ok(tungstenite::Message::Pong(_)) => {}
                Ok(tungstenite::Message::Close(_)) => {
                    let _ = self.socket.flush();
                    self.closed = true;
                    return Ok(false);
                }
                Ok(tungstenite::Message::Binary(_)) | Ok(tungstenite::Message::Frame(_)) => {
                    self.closed = true;
                    return Err(invalid_websocket_data());
                }
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    self.closed = true;
                    return Ok(false);
                }
                Err(error) => {
                    self.closed = true;
                    return Err(websocket_io_error(error));
                }
            }
        }
    }
}

impl Write for WebSocketLineStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.closed {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "hosted coordination WebSocket is closed",
            ));
        }
        if buf.contains(&b'\r') {
            return Err(invalid_websocket_data());
        }
        let delimiter = buf.iter().position(|byte| *byte == b'\n');
        let payload = match delimiter {
            None => buf,
            Some(index)
                if index + 1 == buf.len() && !buf[..index].iter().any(|byte| *byte == b'\n') =>
            {
                &buf[..index]
            }
            Some(_) => return Err(invalid_websocket_data()),
        };
        if self.outgoing.len().saturating_add(payload.len()) > MAX_COORDINATION_FRAME_BYTES {
            return Err(frame_too_large());
        }
        self.outgoing.extend_from_slice(payload);
        if delimiter.is_some() {
            if self.outgoing.is_empty() {
                return Err(invalid_websocket_data());
            }
            self.send_outgoing()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.outgoing.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "hosted coordination request is not newline terminated",
            ));
        }
        self.socket.flush().map_err(websocket_io_error)
    }
}

impl Read for WebSocketLineStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            if self.incoming_offset < self.incoming.len() {
                let remaining = &self.incoming[self.incoming_offset..];
                let count = remaining.len().min(buf.len());
                buf[..count].copy_from_slice(&remaining[..count]);
                self.incoming_offset += count;
                if self.incoming_offset == self.incoming.len() {
                    self.incoming.clear();
                    self.incoming_offset = 0;
                }
                return Ok(count);
            }
            if self.closed || !self.receive_next()? {
                return Ok(0);
            }
        }
    }
}

fn validate_coordination_text(text: &str) -> std::io::Result<()> {
    if text.is_empty()
        || text.len() > MAX_COORDINATION_FRAME_BYTES
        || text.bytes().any(|byte| matches!(byte, b'\r' | b'\n'))
        || !serde_json::from_str::<serde_json::Value>(text).is_ok_and(|value| value.is_object())
    {
        return Err(invalid_websocket_data());
    }
    Ok(())
}

fn invalid_websocket_data() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "hosted coordination requires one bounded JSON object per text frame",
    )
}

fn frame_too_large() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "hosted coordination frame exceeds 1 MiB",
    )
}

fn websocket_io_error(error: tungstenite::Error) -> std::io::Error {
    match error {
        tungstenite::Error::Io(error) => error,
        tungstenite::Error::Capacity(_) => frame_too_large(),
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "hosted coordination WebSocket is closed",
            )
        }
        tungstenite::Error::Tls(_) => std::io::Error::new(
            std::io::ErrorKind::ConnectionAborted,
            "hosted coordination TLS failed",
        ),
        _ => invalid_websocket_data(),
    }
}

fn validate_coordination_websocket_url(
    coordinator: &str,
    allow_insecure_local: bool,
) -> Result<url::Url> {
    let url = url::Url::parse(coordinator).context("parse RESTLESS_COORDINATOR WebSocket URL")?;
    let secure = url.scheme() == "wss";
    let insecure_test = url.scheme() == "ws"
        && allow_insecure_local
        && match url.host() {
            Some(url::Host::Ipv4(address)) => address.is_loopback(),
            Some(url::Host::Ipv6(address)) => address.is_loopback(),
            Some(url::Host::Domain(host)) => host == "localhost",
            None => false,
        };
    let has_userinfo = coordinator
        .split_once("://")
        .and_then(|(_, remainder)| remainder.split('/').next())
        .is_some_and(|authority| authority.contains('@'));
    if !secure && !insecure_test {
        bail!("hosted RESTLESS_COORDINATOR must use wss://");
    }
    if url.host().is_none()
        || has_userinfo
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != HOSTED_COORDINATION_PATH
        || url.port() == Some(0)
    {
        bail!(
            "hosted RESTLESS_COORDINATOR must be one exact wss:// host{HOSTED_COORDINATION_PATH} URL without URL authority"
        );
    }
    Ok(url)
}

fn connect_websocket_with_policy(
    coordinator: &str,
    allow_insecure_local: bool,
) -> Result<WebSocketLineStream> {
    let url = validate_coordination_websocket_url(coordinator, allow_insecure_local)?;
    let config = tungstenite::protocol::WebSocketConfig::default()
        .write_buffer_size(0)
        .max_write_buffer_size(MAX_COORDINATION_FRAME_BYTES + 1)
        .max_message_size(Some(MAX_COORDINATION_FRAME_BYTES))
        .max_frame_size(Some(MAX_COORDINATION_FRAME_BYTES));
    let (socket, response) = tungstenite::client::connect_with_config(
        url.as_str(),
        Some(config),
        0, // A coordination capability may never be redirected to another origin.
    )
    .map_err(|error| match error {
        tungstenite::Error::Http(response) => anyhow!(
            "hosted coordination WebSocket upgrade was refused with HTTP {}",
            response.status()
        ),
        tungstenite::Error::Io(error) => {
            anyhow!("hosted coordination WebSocket connection failed: {error}")
        }
        tungstenite::Error::Tls(_) => anyhow!("hosted coordination TLS handshake failed"),
        _ => anyhow!("hosted coordination WebSocket handshake failed"),
    })?;
    if response.status() != tungstenite::http::StatusCode::SWITCHING_PROTOCOLS {
        bail!("hosted coordination WebSocket upgrade was not accepted");
    }
    Ok(WebSocketLineStream::new(socket))
}

fn connect_runtime(coordinator: &str) -> Result<Stream> {
    if coordinator.contains("://") {
        return connect_websocket_with_policy(coordinator, false).map(Stream::WebSocket);
    }
    std::net::TcpStream::connect(coordinator)
        .map(Stream::Tcp)
        .with_context(|| format!("connect {coordinator} — is restlessd running?"))
}

/// Inside a company container the coordinator is TCP (RESTLESS_COORDINATOR
/// is set in the image); hosted Runtimes use one exact TLS WebSocket URL; on
/// the host it is the unix socket.
fn connect() -> Result<Stream> {
    if let Ok(coordinator) = std::env::var("RESTLESS_COORDINATOR") {
        return connect_runtime(&coordinator);
    }
    let sock = restlessd::appliance::MachineProfile::from_env()?.socket_path();
    if let Ok(stream) = UnixStream::connect(&sock) {
        return Ok(Stream::Unix(stream));
    }
    // Starting the account plane is not waking a company (cross-layer contract
    // §1.4.2): the plane holds credentials but performs no work until asked, so
    // starting it is free and side-effect-free. Waking a cell runs agents and
    // spends money, and no owner surface may do that implicitly.
    //
    // Only when *nothing* is running: if a plane is up on another home, the
    // owner meant that one, and silently starting a second is how installations
    // multiply by accident.
    if live_planes().is_empty() {
        if let Err(error) = start_plane(&sock) {
            return Err(error).with_context(|| no_plane_here(&sock));
        }
        if let Ok(stream) = UnixStream::connect(&sock) {
            return Ok(Stream::Unix(stream));
        }
    }
    UnixStream::connect(&sock)
        .map(Stream::Unix)
        .with_context(|| no_plane_here(&sock))
}

/// Start the account plane for this home and wait for its socket.
///
/// This is the fallback for a machine with no supervisor registered — a
/// development checkout, CI, a fresh clone. On an installed machine the
/// platform supervisor has already started it and this never runs.
fn start_plane(sock: &std::path::Path) -> Result<()> {
    let binary = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("restlessd")))
        .filter(|path| path.exists())
        .context("cannot locate the restlessd binary next to this CLI")?;
    eprintln!("starting the account plane ({})…", binary.display());
    std::process::Command::new(&binary)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .with_context(|| format!("start {}", binary.display()))?;
    // The plane probes its database, broker and gateway before it listens, so
    // this is deliberately patient rather than a fixed short sleep.
    for _ in 0..120 {
        if sock.exists() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    anyhow::bail!("the account plane did not begin listening within 60s")
}

/// A plane may run on any `RESTLESS_HOME`, so "is restlessd running?" is a
/// guess the CLI does not have to make. Enumerate the live planes and say
/// exactly how to reach one.
fn no_plane_here(sock: &std::path::Path) -> String {
    let live = live_planes();
    if live.is_empty() {
        return format!(
            "no account plane at {} — start one with `restless-dev` or `restlessd`",
            sock.display()
        );
    }
    let mut message = format!(
        "no account plane at {}, but {} running elsewhere:\n",
        sock.display(),
        if live.len() == 1 {
            "one is".to_string()
        } else {
            format!("{} are", live.len())
        }
    );
    for plane in &live {
        message.push_str(&format!(
            "  RESTLESS_HOME={}  (pid {}{})\n",
            plane.root,
            plane.pid,
            if plane.companies.is_empty() {
                String::new()
            } else {
                format!(", companies: {}", plane.companies.join(", "))
            }
        ));
    }
    message.push_str("re-run with RESTLESS_HOME set to the plane you mean");
    message
}

/// One plane's registry record. Read-only mirror of the daemon's
/// `plane::PlaneRecord`; unknown fields are ignored so an older CLI can still
/// enumerate a newer plane.
#[derive(Debug, serde::Deserialize)]
struct PlaneRecord {
    root: String,
    pid: u32,
    #[serde(default)]
    companies: Vec<String>,
}

/// Planes whose record exists *and* whose process is alive. A record is a
/// claim; a dead pid means the plane died without cleaning up.
fn live_planes() -> Vec<PlaneRecord> {
    let Ok(home) = std::env::var("HOME") else {
        return Vec::new();
    };
    let dir = PathBuf::from(home).join(".restless").join("planes");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut planes = Vec::new();
    for entry in entries.flatten() {
        let Ok(bytes) = std::fs::read(entry.path()) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<PlaneRecord>(&bytes) else {
            continue;
        };
        if process_is_alive(record.pid) {
            planes.push(record);
        }
    }
    planes.sort_by(|a, b| a.root.cmp(&b.root));
    planes
}

/// Signal 0 asks the kernel whether the pid exists without delivering
/// anything. `kill` is used rather than a `/proc` read because this must work
/// on macOS, where the daemon and this CLI usually run.
fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn request_once(line: &str) -> Result<String> {
    let mut stream = connect()?;
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attention_summary_keeps_each_control_on_its_typed_write_path() {
        let projection = serde_json::json!({
            "company": { "id": "demo_test", "name": "Demo test" },
            "source_health": {
                "authority": "available",
                "orgintel": "available",
                "runtime": "available",
                "browser": "available"
            },
            "items": [
                {
                    "category": "approval",
                    "title": "First contact: design@example.test",
                    "source": {
                        "plane": "authority",
                        "kind": "approval_required",
                        "reference": "41",
                        "party": "design@example.test"
                    },
                    "requested_action": "Allow or decline first contact.",
                    "if_no_action": "Nothing is sent.",
                    "actions": [
                        { "id": "grant", "label": "Grant first contact", "consequence": "Allows contact." },
                        { "id": "decline", "label": "Decline", "consequence": "Closes request." }
                    ]
                },
                {
                    "category": "review",
                    "title": "Review the prepared site",
                    "work_id": "0c5a5e28-6b4d-4b92-a39f-0a9c38d53552",
                    "source": {
                        "plane": "orgintel",
                        "kind": "owner_handoff",
                        "reference": "2c5a5e28-6b4d-4b92-a39f-0a9c38d53552"
                    },
                    "responsible_actor": { "id": "site-lead" },
                    "requested_action": "Inspect the outcome.",
                    "if_no_action": "Work remains paused.",
                    "actions": [
                        { "id": "accept-review", "label": "Accept outcome", "consequence": "Completes Work." },
                        { "id": "request-revision", "label": "Request changes", "consequence": "Starts revision." },
                        { "id": "chat-lead", "label": "Talk with lead", "consequence": "Opens conversation." }
                    ]
                }
            ]
        });

        let rendered = render_attention_summary(&projection);

        assert!(rendered.contains("restless approve -c 'demo_test' --party 'design@example.test'"));
        assert!(rendered
            .contains("restless approve -c 'demo_test' --party 'design@example.test' --decline"));
        assert!(rendered.contains(
            "restless work review -c 'demo_test' --handoff '2c5a5e28-6b4d-4b92-a39f-0a9c38d53552' --decision accept"
        ));
        assert!(rendered.contains(
            "restless message -c 'demo_test' --to 'site-lead' --work '0c5a5e28-6b4d-4b92-a39f-0a9c38d53552' '<your message>'"
        ));
        assert!(!rendered.contains("restless attention act"));
    }

    #[test]
    fn internal_coordination_cannot_accidentally_address_the_owner() {
        assert!(require_message_recipient_in_coordination_wake(true, None).is_err());
        assert!(
            require_message_recipient_in_coordination_wake(true, Some("world-builder")).is_ok()
        );
        assert!(require_message_recipient_in_coordination_wake(false, None).is_ok());
    }

    #[test]
    fn runtime_health_fails_closed_when_required_runtime_evidence_is_missing() {
        let old_report = serde_json::json!({
            "container": "Running",
            "reconciliation": "current",
            "supervisor": { "status": "available" },
            "browser": { "status": "available" },
        });
        assert!(!runtime_report_is_live(&old_report));

        let bridge_unprobed = serde_json::json!({
            "container": "Running",
            "reconciliation": "current",
            "supervisor": { "status": "available" },
            "browser": { "status": "available" },
            "volume_exists": true,
            "volume_mounted": true,
        });
        assert!(!runtime_report_is_live(&bridge_unprobed));

        let bridge_degraded = serde_json::json!({
            "container": "Running",
            "reconciliation": "current",
            "supervisor": { "status": "available" },
            "browser": { "status": "available" },
            "volume_exists": true,
            "volume_mounted": true,
            "coordination": { "status": "degraded" },
        });
        assert!(!runtime_report_is_live(&bridge_degraded));

        let live_report = serde_json::json!({
            "container": "Running",
            "reconciliation": "current",
            "supervisor": { "status": "available" },
            "browser": { "status": "available" },
            "volume_exists": true,
            "volume_mounted": true,
            "coordination": { "status": "available" },
        });
        assert!(runtime_report_is_live(&live_report));
    }

    #[test]
    fn cockpit_probe_uses_the_exact_same_origin_path() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe server");
        let address = listener.local_addr().expect("probe address");
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept probe");
            let mut reader = BufReader::new(stream);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).expect("read request");
            assert_eq!(request_line, "GET /api/companies HTTP/1.1\r\n");
            reader
                .get_mut()
                .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n")
                .expect("write response");
        });

        let status =
            http_status(&format!("http://{address}"), "/api/companies").expect("probe status");
        assert_eq!(status, 401);
        server.join().expect("probe server");
    }

    #[test]
    fn hosted_coordination_url_is_exact_tls_and_carries_no_url_authority() {
        let valid = validate_coordination_websocket_url(
            "wss://owner-1.planes.example.test/internal/v1/coordination",
            false,
        )
        .unwrap();
        assert_eq!(valid.scheme(), "wss");
        assert_eq!(valid.path(), HOSTED_COORDINATION_PATH);

        for refused in [
            "ws://owner-1.planes.example.test/internal/v1/coordination",
            "http://owner-1.planes.example.test/internal/v1/coordination",
            "wss://user:secret@owner-1.planes.example.test/internal/v1/coordination",
            "wss://@owner-1.planes.example.test/internal/v1/coordination",
            "wss://owner-1.planes.example.test/internal/v1/coordination?session_capability=secret",
            "wss://owner-1.planes.example.test/internal/v1/coordination#secret",
            "wss://owner-1.planes.example.test/internal/v1/coordination/",
            "wss://owner-1.planes.example.test/internal/v1/runtime-bridge",
            "wss://owner-1.planes.example.test:0/internal/v1/coordination",
        ] {
            assert!(
                validate_coordination_websocket_url(refused, false).is_err(),
                "accepted unsafe coordinator URL {refused}"
            );
        }

        assert!(validate_coordination_websocket_url(
            "ws://127.0.0.1:17791/internal/v1/coordination",
            true
        )
        .is_ok());
        assert!(validate_coordination_websocket_url(
            "ws://owner-1.planes.example.test/internal/v1/coordination",
            true
        )
        .is_err());
    }

    #[test]
    fn hosted_coordination_preserves_json_lines_and_streaming_watch_frames() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let callback =
                |request: &tungstenite::handshake::server::Request,
                 response: tungstenite::handshake::server::Response| {
                    assert_eq!(request.uri().path(), HOSTED_COORDINATION_PATH);
                    assert!(request.uri().query().is_none());
                    assert!(!request.headers().contains_key("authorization"));
                    assert!(!request.headers().contains_key("cookie"));
                    Ok(response)
                };
            let mut socket = tungstenite::accept_hdr(stream, callback).unwrap();
            let request = socket.read().unwrap().into_text().unwrap();
            assert!(!request.contains(['\r', '\n']));
            let request: serde_json::Value = serde_json::from_str(request.as_str()).unwrap();
            assert_eq!(request["cmd"], "watch");
            assert_eq!(request["session_capability"], "signed-in-frame");

            socket
                .send(tungstenite::Message::Ping(b"still-here".to_vec().into()))
                .unwrap();
            socket
                .send(tungstenite::Message::Text(
                    serde_json::json!({"kind":"first"}).to_string().into(),
                ))
                .unwrap();
            socket
                .send(tungstenite::Message::Text(
                    serde_json::json!({"kind":"second"}).to_string().into(),
                ))
                .unwrap();
            assert!(matches!(
                socket.read().unwrap(),
                tungstenite::Message::Pong(_)
            ));
        });

        let mut stream = connect_websocket_with_policy(
            &format!("ws://{address}{HOSTED_COORDINATION_PATH}"),
            true,
        )
        .unwrap();
        let request = serde_json::json!({
            "cmd": "watch",
            "session_capability": "signed-in-frame"
        })
        .to_string();
        stream.write_all(request.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();

        let mut reader = BufReader::new(stream);
        let mut first = String::new();
        let mut second = String::new();
        reader.read_line(&mut first).unwrap();
        reader.read_line(&mut second).unwrap();
        assert_eq!(first, "{\"kind\":\"first\"}\n");
        assert_eq!(second, "{\"kind\":\"second\"}\n");
        drop(reader);
        server.join().unwrap();
    }

    fn websocket_with_server_message(
        message: tungstenite::Message,
    ) -> (WebSocketLineStream, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            let _ = socket.send(message);
        });
        let client = connect_websocket_with_policy(
            &format!("ws://{address}{HOSTED_COORDINATION_PATH}"),
            true,
        )
        .unwrap();
        (client, server)
    }

    fn assert_server_frame_rejected(message: tungstenite::Message) {
        let (mut stream, server) = websocket_with_server_message(message);
        let mut output = [0_u8; 32];
        let error = stream.read(&mut output).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        drop(stream);
        server.join().unwrap();
    }

    #[test]
    fn hosted_coordination_rejects_binary_multiline_malformed_and_oversized_frames() {
        assert_server_frame_rejected(tungstenite::Message::Binary(b"{}".to_vec().into()));
        assert_server_frame_rejected(tungstenite::Message::Text(
            "{\"one\":1}\n{\"two\":2}".into(),
        ));
        assert_server_frame_rejected(tungstenite::Message::Text("not-json".into()));
        assert_server_frame_rejected(tungstenite::Message::Text(
            format!(
                "{{\"value\":\"{}\"}}",
                "x".repeat(MAX_COORDINATION_FRAME_BYTES)
            )
            .into(),
        ));
    }

    #[test]
    fn hosted_coordination_does_not_follow_upgrade_redirects() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
            }
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 302 Found\r\nLocation: ws://{address}{HOSTED_COORDINATION_PATH}\r\nContent-Length: 0\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .unwrap();
        });
        let result = connect_websocket_with_policy(
            &format!("ws://{address}{HOSTED_COORDINATION_PATH}"),
            true,
        );
        let error = match result {
            Ok(_) => panic!("followed a coordination redirect"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("HTTP 302"));
        server.join().unwrap();
    }

    #[test]
    fn local_runtime_host_port_remains_raw_tcp() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || listener.accept().unwrap());
        let stream = connect_runtime(&address.to_string()).unwrap();
        assert!(matches!(&stream, Stream::Tcp(_)));
        drop(stream);
        server.join().unwrap();
    }
}
