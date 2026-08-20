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
//!   RESTLESS_COORDINATOR  — host:port; when set, TCP instead of unix socket
//!   RESTLESS_OWNER_URL    — owner gateway probed by `doctor`
//!   RESTLESS_COCKPIT_URL  — optional dev cockpit origin probed by `doctor`

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "restless", about = "Restless owner surface")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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
    /// Outstanding owner work, projected from Authority and OrgIntel.
    Attention {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
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
    /// Read unread mail — yours, or an actor's with --as. Reading marks read.
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
        #[arg(long)]
        integration_branch: Option<String>,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        attempt_limit: Option<i32>,
        /// Existing Goal this Work serves.
        #[arg(long)]
        goal: Option<String>,
        /// Existing Work this node requires. Repeat for more than one. These
        /// edges are committed atomically with the node so it cannot start
        /// against a half-built graph.
        #[arg(long)]
        requires: Vec<String>,
        /// Existing producer Work this reviewer may revise. Repeat for more
        /// than one; committed atomically with the node.
        #[arg(long)]
        revises: Vec<String>,
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
    Gate {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
        #[arg(long)]
        work: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        cwd: String,
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
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
    /// (a comma-separated ordered list),
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
    /// Probe every configured reference as present, absent or invalid.
    Check {
        #[arg(long, short = 'c', env = "RESTLESS_COMPANY")]
        company: Option<String>,
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

/// S04-T10. Who this invocation is, at the authority boundary.
///
/// Inside a company container `RESTLESS_COORDINATOR` is set by the image, and
/// that is already how the CLI knows where to connect — so it is also how it
/// knows what it is. On the host, the caller is the owner.
///
/// This is a claim, not a proof: on a single-operator host the container is
/// still trusted to say what it is. What changes is that it now *says* it, the
/// daemon acts on it, and the audit record carries it. Hardening the claim is
/// the Authority Kernel's job.
fn principal() -> &'static str {
    if std::env::var_os("RESTLESS_COORDINATOR").is_some() {
        "company/exec"
    } else {
        "owner"
    }
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

fn state_root() -> PathBuf {
    if let Ok(root) = std::env::var("RESTLESS_HOME") {
        return PathBuf::from(root);
    }
    let home = std::env::var("HOME").expect("HOME");
    PathBuf::from(home).join(".restless")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
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
                format!("restless-co-{name}"),
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
        Command::Doctor { company } => doctor(company),
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
    }
    request
}

/// One request/response pair; `watch` and `attach` handle their own I/O.
fn request_json(command: Command) -> Result<serde_json::Value> {
    Ok(match command {
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
            CredentialCommand::Check { company } => {
                serde_json::json!({ "cmd": "credential-check", "company": company })
            }
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
                "actor": acting_actor(),
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
        Command::Attention { company: c } => {
            serde_json::json!({ "cmd": "attention", "company": c })
        }
        Command::Browser { command } => match command {
            BrowserCommand::Status { company } => {
                serde_json::json!({ "cmd": "browser-status", "company": company })
            }
            BrowserCommand::Request { company, session } => serde_json::json!({
                "cmd": "browser-request", "company": company,
                "id": session.unwrap_or_else(|| "exec".to_string()),
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
            } => serde_json::json!({
                "cmd": "team-create", "company": company, "name": name, "to": lead,
                "body": brief,
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
                title,
                outcome,
                priority,
                expected_artifact,
                repo,
                base_ref,
                integration_branch,
                worktree,
                attempt_limit,
                goal,
                requires,
                revises,
            } => serde_json::json!({
                "cmd": "work-add", "company": company, "actor": owner, "role": role,
                "model": model, "title": title, "body": outcome, "priority": priority,
                "expected_artifact": expected_artifact, "repo": repo, "base_ref": base_ref,
                "integration_branch": integration_branch, "worktree": worktree,
                "attempt_limit": attempt_limit, "goal": goal,
                "requires": requires, "revises": revises,
            }),
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
            WorkCommand::Gate {
                company,
                work,
                name,
                cwd,
                command,
            } => serde_json::json!({
                "cmd": "work-gate", "company": company, "id": work, "name": name,
                "cwd": cwd, "argv": command,
                "actor": std::env::var("RESTLESS_ACTOR").unwrap_or_else(|_| "owner".to_string()),
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
        Command::Events { company: c, limit } => {
            serde_json::json!({ "cmd": "events", "company": c, "limit": limit })
        }
        Command::Inbox {
            company: c,
            as_actor,
        } => {
            serde_json::json!({ "cmd": "inbox", "company": c, "as_actor": as_actor })
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
        } => serde_json::json!({
            "cmd": "message",
            "company": c,
            "from": from.or_else(|| std::env::var("RESTLESS_ACTOR").ok())
                .unwrap_or_else(|| "owner".to_string()),
            "to": to,
            "id": work,
            "body": body,
        }),
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
        Command::Doctor { .. }
        | Command::Watch { .. }
        | Command::Attach { .. }
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

    let (owner_gateway, owner_gateway_ok) = http_check(
        &owner_url,
        "/api/companies",
        &[200, 401],
        "owner API must answer (an unauthenticated 401 is healthy)",
    );
    let (cockpit_shell, cockpit_shell_ok) = http_check(
        &cockpit_url,
        &format!("/{company}"),
        &[200],
        "cockpit shell must render",
    );
    let (cockpit_api, cockpit_api_ok) = http_check(
        &cockpit_url,
        "/api/companies",
        &[200, 401],
        "cockpit same-origin API path must reach the owner gateway",
    );
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
    if !coordinator_ok || !owner_gateway_ok || !cockpit_shell_ok || !cockpit_api_ok {
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
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Unix(stream) => stream.write(buf),
            Stream::Tcp(stream) => stream.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Unix(stream) => stream.flush(),
            Stream::Tcp(stream) => stream.flush(),
        }
    }
}

impl std::io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Unix(stream) => stream.read(buf),
            Stream::Tcp(stream) => stream.read(buf),
        }
    }
}

/// Inside a company container the coordinator is TCP (RESTLESS_COORDINATOR
/// is set in the image); on the host it is the unix socket.
fn connect() -> Result<Stream> {
    if let Ok(coordinator) = std::env::var("RESTLESS_COORDINATOR") {
        return std::net::TcpStream::connect(&coordinator)
            .map(Stream::Tcp)
            .with_context(|| format!("connect {coordinator} — is restlessd running?"));
    }
    let sock = state_root().join("restlessd.sock");
    UnixStream::connect(&sock)
        .map(Stream::Unix)
        .with_context(|| format!("connect {} — is restlessd running?", sock.display()))
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
    fn runtime_health_fails_closed_when_volume_evidence_is_missing() {
        let old_report = serde_json::json!({
            "container": "Running",
            "reconciliation": "current",
            "supervisor": { "status": "available" },
            "browser": { "status": "available" },
        });
        assert!(!runtime_report_is_live(&old_report));

        let live_report = serde_json::json!({
            "container": "Running",
            "reconciliation": "current",
            "supervisor": { "status": "available" },
            "browser": { "status": "available" },
            "volume_exists": true,
            "volume_mounted": true,
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
}
