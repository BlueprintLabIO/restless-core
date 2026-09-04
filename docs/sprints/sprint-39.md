# Sprint 39 — Work Through Attention With Verified Agent Harnesses

**Status:** Implemented release candidate; provider-backed three-harness qualification pending  
**Date:** 4 September 2026  
**Depends on:** Sprint 11 controlled ACP sessions, Sprint 17 scoped session continuity, the current Attention and work-linked conversation flow, and the current native Codex runtime

## Outcome

An owner can open an existing Attention item, choose **Work through this**, and collaborate with the accountable executive or lead in the existing Executive Rail until the issue is ready for an explicit decision.

That experience works through a small, certified set of agent harnesses:

- **Restless Managed** — the current OMP-backed ACP process and the default for both coordination and productive work;
- **Codex** — the existing first-party Codex App Server integration; and
- **Claude Agent** — the Claude Code agent loop, packaged through the official Claude Agent SDK and exposed over the maintained `claude-agent-acp` adapter.

This sprint does not create a Focus Session object, another inbox, a harness marketplace, or a harness picker on each Attention item. Harness selection is company execution policy. The owner keeps working through the responsible Restless actor and the existing Attention object.

## Existing Contracts Extended

- [`owner-cockpit.md`](../specs/owner-cockpit.md) already owns Attention, the Executive Rail, work-linked conversation, and explicit owner resolution.
- [`company-runtime.md`](../specs/company-runtime.md) already requires the Runtime Bridge to own the full ACP launch contract rather than inheriting ambient harness policy.
- [`cross-layer-contract.md`](../specs/cross-layer-contract.md) already defines session lifecycle, session input, harness selection in the launch envelope, and one cross-layer Attention contract.

This sprint adds no competing owner or runtime canon.

## Observed Starting Point

- [`runtime.rs`](../../crates/restlessd/src/runtime.rs) models the productive worker runtime as only `Omp | Codex`; coordination conversations remain on OMP.
- [`acp.rs`](../../crates/restlessd/src/acp.rs) contains reusable ACP lifecycle code, but its executable, arguments, config root, and launch assumptions are OMP-specific.
- [`staff/execution.rs`](../../crates/restlessd/src/staff/execution.rs) refuses Codex conversations and connected MCP tools rather than preserving parity.
- [`infra/company-image/Dockerfile`](../../infra/company-image/Dockerfile) pins OMP and Codex but does not contain a Claude Agent ACP implementation.

The sprint begins from those concrete seams rather than designing a universal provider framework.

## Why This Sprint Exists

Attention already identifies the moments where autonomous work is unlikely to converge safely or quickly without the owner's judgment. The missing part is the prepared continuation:

- the owner can inspect an item, but hard goals and wide search spaces often require several conversational turns;
- the responsible actor should retain the Work, evidence, decision, and conversation context while the owner sharpens the objective or trades off options;
- some companies will get materially better results by using the native behavior of Codex or Claude rather than routing every actor through one wrapper; and
- merely launching another executable is not support. A supported harness must preserve Restless identity, policy, tools, attribution, usage, cancellation, recovery, and cleanup.

The product move is therefore small and the runtime move is exact: extend the existing Attention conversation path, then admit Claude Agent at the same controlled execution boundary already used by Restless Managed and Codex.

## Frozen Product Decisions

1. **Attention remains the entry point.** “Work through this” is a contextual action on an existing Attention item. It opens the existing Executive Rail with the item and its evidence still visible.
2. **No new coordination object.** The durable objects remain Attention, Work, Message, Decision, Session, and Attempt. There is no `FocusSession`, `AlignmentSession`, or new Attention category.
3. **Conversation does not imply resolution.** Sending messages, receiving an answer, or ending a harness turn never resolves the Attention item. Only the existing typed owner action—accept, request changes, direct, approve, reject, defer, or dismiss as applicable—changes its lifecycle.
4. **The owner works through accountable leadership.** The action targets the responsible executive or lead. Staff work remains inspection-first; this sprint does not turn every worker into a direct owner chat endpoint.
5. **Harness is execution policy, not an Attention choice.** Companies receive separate coordination and worker defaults. A session records the exact resolved harness. There is no per-item picker and no hot migration of a running session between harnesses.
6. **Restless Managed remains the default.** Existing companies preserve current behavior unless their configuration is explicitly changed.
7. **The certified set is intentionally small.** This sprint supports Restless Managed, Codex, and Claude Agent. It does not promise compatibility with every ACP registry agent.
8. **Claude is presented as “Claude Agent.”** Technical diagnostics may name Claude Code, the Claude Agent SDK, and `claude-agent-acp`. Owner-facing product copy uses the partner-appropriate “Claude Agent” label.
9. **Claude uses ACP, not a second bespoke protocol.** Restless launches a pinned `claude-agent-acp` process. The interactive Claude terminal UI and arbitrary user-installed Claude binaries are not the product integration.
10. **Native transports remain allowed.** Codex continues to use its first-party App Server API where that produces a stronger and better-controlled integration. Restless normalizes the contract above the transport; it does not force all harnesses through ACP.
11. **Baseline parity is shared; enhancements are capability-gated.** Turn-by-turn conversation, events, permissions, usage, cancellation, and recovery are required. In-flight steering, plan updates, and other native affordances appear only when the observed session supports them.
12. **No ambient harness authority.** User-level configuration, plugins, hooks, skills, MCP servers, and remembered approvals cannot silently augment the Restless launch contract.
13. **Claude Agent API authentication is the supported path.** Restless does not expose a `claude.ai` login flow or reuse subscription credentials for a third-party integration without explicit commercial approval. Provider credentials remain outside the Runtime Plane.
14. **Model and harness are separate but validated together.** Restless never silently substitutes a model to make a harness launch. An incompatible saved combination is rejected with an actionable configuration error before work starts.

## User Experience

### Entry

On an eligible Attention item, the existing conversation action becomes:

> **Work through this with {responsible lead}**

Selecting it:

1. focuses the current Attention item;
2. opens the existing Executive Rail to the accountable actor;
3. attaches the current `workId` and `attentionId` to the conversation launch context;
4. keeps the item summary, recommendation, evidence, and exact decision actions visible; and
5. restores the existing scoped conversation if one is already active.

There is no modal wizard and no choice of harness in this flow.

### During the conversation

- The first frame tells the actor what triggered Attention, what evidence exists, and which owner action remains open.
- The owner can refine the goal, constrain the search space, ask for options, or request another bounded investigation.
- New evidence appears through the current message and evidence projections; it is not copied into a second transcript.
- If the harness accepts in-flight input, the UI labels the input as applied to the current turn. Otherwise it is visibly queued for the next turn; the product never implies live steering when only cancellation and restart are available.
- The actor may recommend a decision, but cannot manufacture owner approval.

### Leaving and resolving

- Closing the rail leaves the Attention item unresolved and resumable.
- Ending, failing, or cancelling a harness session does not lose the conversation or evidence.
- The owner resolves the item through its existing typed action. The resulting message or decision preserves the normal work-link and provenance.

### Settings and status

Harness configuration is an expert company setting, not a daily workflow control:

- **Coordination harness** controls Exec and producing-lead owner conversations.
- **Worker harness** controls productive Staff Attempts.
- Each option shows a compact status: installed version, ready/not ready, authentication requirement, and capability limitations that affect Restless.
- Unsupported identifiers and arbitrary executable paths are refused.
- Existing `worker_runtime = "omp" | "codex"` configuration remains readable during migration and maps to the corresponding worker harness. New writes use the canonical harness fields.

## Runtime Contract

### One normalized boundary

The bridge accepts a session launch envelope and resolves it to one certified harness implementation. The boundary must cover:

- stable harness identifier and pinned build identity;
- transport kind (`acp` or native Codex App Server);
- company, actor, responsibility, work, attempt, and conversation scope;
- exact working directory and writable roots;
- Restless-authored system instructions and actor identity;
- project instruction policy;
- selected model and effort;
- approved tools, MCP servers, and external-effect boundary;
- isolated harness configuration and session storage roots;
- initialize/readiness result and observed capability digest;
- normalized session events, permissions, usage, and terminal outcome;
- input, cancellation, replacement, recovery, and cleanup behavior; and
- the exact harness identity persisted on the Session and Attempt.

The boundary is intentionally not a user-extensible provider abstraction. The certified implementations are compiled or statically configured by the Restless release.

### Certified profile matrix

| Product label | Transport | Initial role support | Required status at sprint exit |
|---|---|---|---|
| Restless Managed | ACP via current OMP process | Coordination and workers | Regression-free default |
| Codex | Native Codex App Server | Coordination and workers | Conversation, approved MCP, usage, cancellation, and recovery parity proven |
| Claude Agent | ACP via pinned `claude-agent-acp` | Coordination and workers | Full baseline parity and live proof |

Optional protocol features may differ. Required semantic outcomes may not.

### Claude Agent admission

The counted Claude integration uses the official Claude Agent SDK's Claude Code agent loop through the maintained ACP adapter. The release must pin:

- adapter package version and integrity;
- the bundled Claude Code/Agent binary identity actually executed;
- Node/runtime requirements;
- ACP protocol compatibility;
- license and distribution disposition; and
- the external authentication route.

Claude launches with an isolated `CLAUDE_CONFIG_DIR`. The bridge must demonstrate that ambient user settings, plugins, hooks, subagents, MCP configuration, cached permissions, and session state cannot enter the company session unless Restless explicitly supplies them. Project-local instructions may be admitted only through the existing project-instruction policy and must not override Restless system authority.

The Runtime Plane must not receive an Anthropic root API key or reusable subscription credential. The preferred route is a host-owned, scoped Anthropic-compatible model relay attributed to company, session, actor, and work. If the adapter cannot use that route without leaking a provider credential, Claude Agent fails admission and the product must not claim support.

### Session and history rules

- Session locators are namespaced by company, harness, actor, responsibility, and conversation or attempt identity.
- Resume uses the harness's native session identifier only after scope and harness identity match.
- A configured harness change starts a new session and reconstructs from canonical Restless messages/evidence. It does not import another harness's private state.
- Historical updates emitted during load/resume are tagged as reconstruction and cannot become new canonical tool effects, usage, or current-turn messages.
- A harness-side transcript is runtime state, not a second source of truth.

### Permissions and external effects

- Internal read/edit/command operations run within the existing sandbox and launch allowlist.
- External side effects remain behind Restless-owned broker/MCP authority and idempotency controls.
- Harness permission requests map to normalized Restless events and may not invoke an unscoped native approval cache.
- Refused or unsupported permissions produce a structured turn failure, not a hung session or fabricated success.

### Usage and outcomes

- Provider-observed charged usage wins when available.
- Harness-reported token usage is preserved separately with source and confidence.
- Missing fields remain unknown; they are never coerced to zero.
- Every turn ends once with `completed`, `failed`, or `cancelled`, with structured reason and harness identity.
- Process exit, adapter error, protocol error, provider error, permission refusal, timeout, and cancellation remain distinguishable.

## Deterministic vs. Judgment Work

Deterministic work in this sprint includes version pinning, configuration migration, launch construction, capability capture, event normalization, session scoping, usage parsing, cancellation deadlines, and cleanup assertions.

Judgment work includes deciding when owner collaboration is useful, narrowing a search space, comparing options, and recommending a decision. Those outcomes are evaluated through realistic scenarios and fresh review rather than content-classifier heuristics.

## Failure Semantics

- **Harness unavailable:** refuse before Work starts; preserve the Attention item and show the exact readiness problem.
- **Authentication unavailable or expired:** refuse before prompt delivery; do not fall back to another credential or harness silently.
- **Capability missing:** disable only the optional UI affordance. If a baseline capability is missing, the harness is not ready.
- **Incompatible model:** reject configuration or launch before provider traffic; never substitute.
- **Process dies during a turn:** emit one failed terminal outcome, preserve partial events as non-authoritative evidence, and allow explicit retry/reconstruction.
- **Cancel acknowledgement missing:** terminate the owned process group after the bounded deadline, mark the session honestly, and clean runtime state.
- **Runtime restart:** mark the old process/session lost or reconstructable according to observed state; never replay prior tool effects as new work.
- **Owner input races turn end:** assign it deterministically to the current turn only when the transport acknowledged steering; otherwise queue it to the next turn.
- **Attention item resolved elsewhere:** conversation may continue as ordinary work-linked history, but it cannot reopen or mutate the resolved decision implicitly.
- **Harness configuration changes mid-session:** the current session retains its recorded harness; the next explicit session uses the new setting.

## Risk Disposition

| Risk | Disposition |
|---|---|
| Different harnesses produce different styles and quality | Accepted; quality difference is part of the product value |
| Provider or adapter versions drift | Guarded by release pins, integrity checks, readiness probes, and explicit upgrades |
| Arbitrary ACP agents execute under a “generic” seam | Avoided; only certified profiles are accepted |
| Ambient Claude/Codex/OMP configuration changes company policy | Invariant; isolated roots and launch inspection must prove absence |
| Provider credentials reach the Runtime Plane | Invariant |
| Third-party Claude subscription login is offered without approval | Avoided; API authentication only unless separately approved |
| Resume replays historical tool effects as current work | Invariant |
| Existing companies change behavior during migration | Invariant; Restless Managed remains the default and legacy config maps exactly |
| Optional native features create misleading UX parity | Guarded by observed capability gating and explicit queued/applied states |
| A conversation is mistaken for owner approval | Invariant; only typed owner actions resolve Attention |
| Harness abstraction grows into a marketplace prematurely | Avoided; install/discovery/custom command surfaces are out of scope |

## Non-Goals

- supporting every ACP registry agent;
- Gemini CLI, arbitrary local CLIs, or custom executable profiles;
- a harness marketplace, install flow, or dynamic package resolution;
- a per-Attention or per-message harness picker;
- moving a live conversation from one harness to another;
- importing a user's personal Claude Code, Codex, or OMP history/configuration;
- exposing Claude's interactive terminal UI inside Restless;
- multi-agent orchestration within a single owner alignment conversation;
- changing Attention ranking or inventing a new Attention category;
- replacing the existing Executive Rail or canonical message timeline;
- using chat completion as an implicit decision; or
- changing the default harness based on benchmark results inside this sprint.

## Acceptance Criteria

- [x] An eligible Attention item exposes **Work through this with {responsible lead}** and opens the existing work-linked Executive Rail with evidence and typed resolution actions still visible.
- [x] Closing, cancelling, completing, or failing the conversation leaves the Attention item unresolved until an explicit existing owner action occurs.
- [x] Existing companies retain Restless Managed behavior without configuration edits; the legacy `worker_runtime` values migrate without semantic change.
- [x] New canonical coordination and worker harness settings accept only `restless-managed`, `codex`, and `claude-agent`.
- [x] Every Session and productive Attempt records the resolved harness identifier, build identity, transport, model, capability digest, and relevant scope.
- [x] The ACP implementation no longer embeds OMP-specific executable, argument, storage-root, or capability assumptions in shared session/event logic.
- [x] Restless Managed passes its existing ACP launch, session, event, usage, permission, cancellation, and cleanup regression suite.
- [x] The company image contains a pinned, integrity-checked `claude-agent-acp` distribution and the exact Claude Code/Agent binary it executes.
- [x] A Claude Agent launch proves exact cwd, actor/system context, model, effort, approved tools/MCP, isolated config, readiness-before-prompt, and no ambient extensions or approvals.
- [x] A Claude Agent session receives provider access without a reusable Anthropic provider credential or subscription token entering the Runtime Plane.
- [x] Claude Agent supports new and resumed scoped conversations, productive attempts, normalized live events, permissions, usage attribution, cancellation, crash handling, restart reconstruction, and cleanup at the common runtime boundary.
- [x] Codex supports the same required coordination-conversation and productive-attempt outcomes, including approved connected tools, while retaining its native App Server transport.
- [x] In-flight owner input is labeled `applied` only after transport acknowledgement; otherwise it is queued for the next turn.
- [x] Unsupported identifiers, arbitrary executable paths, incompatible model/harness combinations, and unready harnesses fail before Work starts with actionable errors.
- [x] Historical events observed during resume/reconstruction cannot create new canonical messages, tool effects, or usage charges.
- [ ] A live acceptance corpus completes across all three certified harnesses, including one hard-goal Attention conversation, one productive edit attempt, one permission/external-tool path, one cancellation, and one restart/recovery case.
- [ ] Runtime inspection finds no leaked provider secrets, orphaned harness processes, cross-company session state, or second canonical transcript.
- [ ] A fresh independent reviewer verifies the live evidence and records the release disposition for each harness separately.

## Implementation Verification

The implementation is complete across product, runtime, persistence, gateway, configuration, and release-image layers. The deterministic release evidence is:

- Rust workspace compilation and all non-live `restlessd` tests pass, including the shared ACP lifecycle, Claude isolation/exact-selection, Codex MCP, Anthropic relay/metering, Attention context, and harness-readiness cases.
- The schema-35 Postgres corpus passes serially, including durable `agent_sessions` and productive Attempt harness identity. Database cases are intentionally serialized because their scratch-schema setup contends under Cargo's default parallel runner.
- Web type checking, production build, Office corpus, and reader corpus pass. The work-through URL now opens the existing rail for both prepared-computer and discussion-only Attention items.
- The release image builds from the pinned package tarball and integrity, then its live health probe reports Restless Managed `omp-18.0.10`, Codex `codex-cli-0.151.0`, Claude ACP `0.73.0`, Claude Code `2.1.257`, and Claude Agent SDK `0.3.257`. Startup fails if the Claude adapter, native agent, or SDK identity drifts.
- A credential-free ACP process probe completed `initialize` and `session/new` against the release package, advertised only the exact configured Claude model, and confirmed the configured model, effort, and ordinary permission mode before any productive prompt.
- The adapter package is Apache-2.0, requires Node 22 or newer, and runs on the image's pinned Node 24 base. Owner-facing copy remains **Claude Agent**; diagnostics retain the exact Claude Code/SDK/adapter identities.

Provider-backed T7 dogfood is not marked complete. This checkout has no Anthropic credential or dedicated `*_test` Company, so no paid Claude turn, three-harness hard-goal run, or independent sign-off was fabricated. The release dispositions remain:

- **Restless Managed:** supported default; regression corpus green.
- **Codex:** implementation-qualified; final supported disposition awaits the shared live T7 matrix.
- **Claude Agent:** package, protocol, isolation, credential-boundary, and metering implementation qualified; final supported disposition awaits an API-key-backed live T7 matrix.

## Slice By Layer

### Control Plane

- Add canonical coordination and worker harness configuration with exact legacy mapping.
- Persist the resolved harness/build/capability identity on sessions and attempts.
- Reject unsupported or incompatible configurations before queueing work.
- Preserve existing Attention lifecycle and typed owner actions.

### Organization Intelligence

- Continue targeting the accountable executive or lead for owner conversation.
- Prepare the existing Attention, Work, evidence, and open decision context for the selected actor.
- Do not invent a separate focus-session state machine.

### Runtime Bridge

- Extract OMP-specific launch details from shared ACP session/event code.
- Add the pinned Claude Agent ACP profile and readiness/admission checks.
- Bring native Codex coordination and connected-tool behavior to the shared semantic contract.
- Normalize events, usage, permissions, steering/queue state, cancellation, recovery, and cleanup.

### Authority and Model Gateway

- Provide the supported Claude Agent API authentication route without exposing reusable provider credentials to Runtime.
- Attribute provider calls and usage to company/session/actor/work.
- Preserve broker authority for external effects.

### Interface

- Extend the existing Attention action and Executive Rail only.
- Show queued versus applied owner input truthfully.
- Add compact harness readiness/configuration status in expert settings.
- Show actionable launch failures without exposing credentials or raw internal commands.

### Observability and Verification

- Record launch manifest, observed capabilities, version identity, terminal reason, usage source/confidence, and cleanup result.
- Run the same semantic corpus across all certified harnesses.
- Capture evidence for default regression, secret isolation, cancellation, restart, and cross-company boundaries.

## Implementation Tickets

- [x] [T0 — Freeze the certified harness contract and admit the Claude route](sprint-39/T0-harness-contract-claude-admission.md) — implementation admission complete; its paid live proof remains in T7.
- [x] [T1 — Extract the harness-neutral ACP session runner](sprint-39/T1-harness-neutral-acp-runner.md)
- [x] [T2 — Package and constrain Claude Agent](sprint-39/T2-package-constrain-claude-agent.md)
- [x] [T3 — Complete Claude session, usage, and recovery parity](sprint-39/T3-claude-session-runtime-parity.md) — common-boundary implementation complete; provider-backed qualification remains in T7.
- [x] [T4 — Extend harness policy to coordination sessions](sprint-39/T4-coordination-harness-policy.md)
- [x] [T5 — Add Work through this to the existing Attention flow](sprint-39/T5-attention-work-through-flow.md)
- [x] [T6 — Close Codex coordination and connected-tool parity](sprint-39/T6-codex-coordination-parity.md) — implementation complete; provider-backed qualification remains in T7.
- [ ] [T7 — Dogfood the three-harness release and purge scaffolding](sprint-39/T7-three-harness-dogfood.md) — blocked on a dedicated test Company, Anthropic API credential, and independent reviewer.

## Expected Execution Order

`T0 → T1 → { T2 → T3, T4 → T6 } → T5 → T7`

After `T1`, Claude lifecycle work and coordination/Codex work can proceed independently. The owner flow in `T5` closes only after all three harness paths are available; the release proof in `T7` then verifies the whole experience.

## Stop Conditions

Stop and return to founder alignment if:

- Claude Agent requires a reusable provider or subscription credential inside Runtime;
- the maintained adapter cannot preserve Restless system/tool/MCP policy without a bespoke fork;
- current Claude distribution or partner terms do not permit the proposed product integration or naming;
- coordination support requires a new durable focus-session object rather than the existing Attention/conversation contract;
- Codex connected-tool parity requires bypassing Restless Authority; or
- the abstraction begins accepting arbitrary executables or remote agent packages.

## Primary Upstream References

- [Anthropic Claude Agent SDK overview](https://code.claude.com/docs/en/agent-sdk/overview)
- [Anthropic Claude Agent SDK sessions](https://code.claude.com/docs/en/agent-sdk/sessions)
- [Anthropic Claude Agent SDK hosting](https://code.claude.com/docs/en/agent-sdk/hosting)
- [`claude-agent-acp` maintained adapter](https://github.com/agentclientprotocol/claude-agent-acp)
- [Agent Client Protocol v1 overview](https://agentclientprotocol.com/protocol/v1/overview)

## Makes Deletable

At sprint exit, delete or retire:

- OMP-specific executable, config-root, and launch-argument assumptions from shared ACP session code;
- the `WorkerRuntime::Omp | Codex` type as the canonical product configuration surface;
- duplicated per-harness session/event/terminal normalization where the common boundary now owns it;
- any prototype Claude launch scripts, raw-key experiments, unpinned packages, and permissive test configuration;
- UI copy that implies conversation completion resolves Attention; and
- any temporary custom-agent or per-item harness picker scaffolding used during exploration.
