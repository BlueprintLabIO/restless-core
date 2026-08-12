# Salvage map — proven components from the prior implementation

The prior control plane (held in a separate repo) is a **salvage source and reference**, not a baseline.
This map records which of its components are proven and may be lifted into restless, versus what the
target architecture (`ARCHITECTURE.md`) requires that has **no seed code today**.

Each "lift" is an **extraction task with a re-validation step**, not a verbatim copy. Several proven
components are entangled with machinery the rebuild discards (universal command dispatch, append-only
ledger, asset custody, per-turn sandbox fencing). Treat every entry below as a hypothesis to confirm
during extraction, not a guaranteed lift.

The audit that produced this map: five dimension auditors + synthesis against `ARCHITECTURE.md`,
run 2026-08-12.

---

## Proven — candidate lifts (verify on extraction)

### Kernel layer

| Component | What it does | Evidence it works | Extraction note |
|---|---|---|---|
| **External-effect broker** (`effect.rs`, ~1547 LOC) | `ExternalEffect` lifecycle: propose → approve/standing-grant → connector-bound execute → receipt → outcome-unknown reconciliation. Self-hashing receipt digests. | Adversarial domain-invariant tests (unknown-outcome never blind-retried; canonical amount; standing-grant rejects unprovable geography/channel); integration tests (redelivery fail-closed, connector-bound effect); black-box receipt-backed effect case passed 2026-08-11. | Lifts near-verbatim. Strip the universal-`Command` envelope and references to the internal org ontology (keep capability + amount + counterparty + channel). **The single most proven, most directly-mappable component.** ARCHITECTURE.md §3.2, §9.2. |
| **`HttpConnector` + fixture simulator** (`company-connectors`, `company-testkit/fixture_connector`) | Live capability probe; idempotent effect POST with `Idempotency-Key`; transport-after-POST classified as `OutcomeUnknown` (never retried); read-only GET reconciliation that can never replay the POST. The fixture is a deterministic simulator. | `connector_fixture` test exercises probe → execute → receipt end-to-end. | Ready-made real-vs-simulated provider seam (ARCHITECTURE.md §10.8). Depends only on effect types + reqwest. Re-validate the fixture against the restless effect-broker cut. |
| **Authority grants + delegation** (`authority.rs`) | Scoped / attributable / revocable / expiring grants; delegation-depth containment; monotonic constraint narrowing (spend, data-class, geography, channel). | Capability tests; standing-grant constraint enforcement against external effects. | Reuse the grant/constraint math. **Must trim** the capability implication table of ~25 internal-coordination capabilities (`attempt.queue`, `workspace.*`, `conversation.*`…) so it governs only real authority (`effect.spend/communicate/deploy`, `model.access`, `secret.use`). §3.2, §3.3. |
| **Company identity + approvals** (`organisation.rs`, `effect.rs`) | Company lifecycle; typed approval subjects; multi-attestation resolution; mission/charter digest binding. | Activation/approval unit tests. | Drop the over-binding activation snapshot (bound exec runtime + probe into the digest); keep core approval/identity types. §3.2. |
| **Runtime binding + live probe** (`runtime.rs`) | Installed-runtime identity with live task-capable probe + freshness window + model execution profile. | Runtime tests; used by ACP admission. | Kernel model/compute access control and "probe, never guess". Shed coupling to universal command and internal attempt state. §3.2. |
| **Evidence provenance typing** (`evidence.rs`, ~184 LOC) | `ProviderReceipt` requires control-plane recording (no self-assigned labels); read-receipt binds an exact version with a re-derivable digest. | Evidence tests; integration test. | Small, pure, no command/ledger coupling. §3.5, §9.5. |
| **Dispatch spine** (`command_store.rs`, ~613 LOC — transaction machinery only) | Idempotent insert (`ON CONFLICT`), request-digest conflict detection, SERIALIZABLE whole-txn retry with bounded backoff, per-company write lane. | Retry-classification + bounded-backoff tests. | Reuse for a **kernel-local, effect-only** apply path. **Must not** carry the universal `Command` enum or the append-everything ledger. §3.2. |

### Runtime layer

| Component | What it does | Evidence it works | Extraction note |
|---|---|---|---|
| **Pure ACP session lifecycle** (~400–500 LOC inside `contained.rs`) | initialise → authenticate (model gateway) → session → select + live-verify model → prompt → capture output/tools/permissions → cancel. | Driven end-to-end by the supervisor integration test and the black-box harness against a real codex-acp container; CI runs it. | **High extraction friction.** The ACP orchestration is interleaved with sandbox transport bridging and custody `await_result`. Extract by cutting away the envelope/fence/tunnel scaffolding. §4.3, §5.2. |
| **Model gateway** (`company-model-gateway`, ~1700 LOC) | HMAC-signed short-lived purpose tokens; server-side provider-key injection (caller never sees the key); crash-durable file usage/audit spool; fail-closed request/byte limits. | Auth/proxy/usage tests; live Codex turn 2026-08-09. | Standalone crate already. Most ready-made §3.2 model-credential-isolation component. **Add the missing company-level spend dimension** (today only request-count bounded). §3.2, §6.1, §9.6. |
| **Sandbox boundary** (`company-sandboxd`) | Owner-private Docker + Chromium supervisor: digest-pinned adapters, non-root read-only-rootfs, no host ports, live inspection, generation-fenced browser transfer. | 38 lib + 2 broker + 4 browser-provider + 58 supervisor tests passed 2026-08-11. | Keep the **process-isolated boundary and provider-neutral protocol**. **Invert the lifecycle**: today one disposable tmpfs-home container per turn, deleted on release — the §5 opposite. Restless wants a persistent company container with a real `/company` home. Much of the custody/journal/lock machinery is dropped. §5, §16.3. |
| **Browser session + human attach** (`handoff.rs`, `docker_browser.rs`) | Run Chromium; hand a live generation to a human (offer/accept/return/revoke) with a one-time bearer for 2FA/CAPTCHA/sign-in. | Browser-broker + provider tests; full-system gate. | Concept maps cleanly (§5.6, §3.5 prepared last mile). The 4-state + fence + bearer lifecycle is more state machine than needed and shrinks without custody scaffolding. |
| **Adapter image package list** (`infra/sandbox-agent/Dockerfile`) | node24, chromium, git, python3, ripgrep, socat, playwright-mcp, codex-acp on a Debian base. | Built + digest-pinned + live-probed by sandboxd. | A credible starting point for a §5.2 "standard company image". Remove the single-codex-entrypoint and tmpfs-home assumptions; it becomes a persistent, multi-process company computer. |

### OrgIntel / UI layer

| Component | What it does | Evidence it works | Extraction note |
|---|---|---|---|
| **Context assembly** (`context.rs`, ~358 LOC) | Deterministic turn-context compilation into a versioned envelope with digest + source pinning. | Exercised on every worker turn; digest is load-bearing and tested. | Already a pure compile step over a read-only snapshot, decoupled from the write path. Lift against the new non-kernel OrgIntel read model; drop kernel aggregate-version pinning, keep the deterministic-snapshot + digest idea. §4.4, §9.5. |
| **Outbox / LISTEN-NOTIFY transport** (`worker/delivery.rs`) | Reactive dispatch with lease/recovery semantics. | Powers all execution today; recovery tested. | Reusable as the §4.4 event-driven-wakeup **transport**. The Work/Attempt state machine wrapping it is NOT reused. The scheduler/periodic wakeups must be built on top — this is only the seed. |
| **Directed messaging + follow-up** (`communication.rs`) | Directed messages with recipients/kinds; review-decision follow-up that routes back to an accountable actor. | Used by the SPA communications surface; tested. | Concept maps to §4.4 messaging/inboxes once stripped of the universal-command envelope. The follow-up-stage escalation is a good seed for §4.4 escalation timing. |
| **SPA posture** (`src/routes/+page.svelte`) | Calm main surface + right-hand exec chat + drawer drill-downs; outcomes/decisions/effects first. | Live product; matches the target posture. | Thin SSE/API client carrying no truth; lifts cleanly once it reads from a new OrgIntel API. The strongest non-Rust salvage. Not in this repo yet. |

### Cross-cutting / validation

| Component | What it does | Evidence it works | Extraction note |
|---|---|---|---|
| **Live-probing discipline** | Before claiming any capability, live-probe the real runtime/connector and record result/time/version; installed/authenticated/operational/task-capable are distinct. | Black-box live probes; real Codex cold probe 2026-08-09. | A behaviour/practice, not just code. Ports directly; §10.8 real-vs-simulated correctness depends on it. |
| **Black-box golden scenario shape** | Compiled-CLI full-company run: formation → probes → staff work → approval → receipt-backed effect → stop/resume → restart recovery → escalation → handoff → verify; exact terminal Docker-cleanup proof; disposable-DB supervision. | 6/6 supervised cases passed 2026-08-11. | The §15 / §17-step-2 acceptance harness. The supervision + cleanup-proof wrapper is the most portable asset. **Rewrite the scenario driver** against restless's small kernel effect API; keep the scenario shape verbatim. |

---

## Greenfield — required by ARCHITECTURE.md, no seed code today

These are the hardest net-new work and the actual product differentiators. They are **not** extraction
tasks.

1. **Persistent Company Linux Runtime.** Invert the per-turn disposable sandbox into a persistent
   company computer with a real `/company` home, Git repos/worktrees, project services, surviving
   across turns and restarts. §5, §17 step 2.
2. **Proactive OrgIntel.** A time- **and** event-driven scheduler (deadlines, periodic Exec planning,
   event-driven wakeups on dependent results); a minimal Exec planner that converts a vague directive
   into a bounded milestone; health/heartbeat + reassignment for agent-crash recovery; stagnation /
   duplicate-work detection. Today the prior Exec only acts when the owner types. §4.2, §4.4, §4.5.
3. **Files-as-primary-primitive + artifact references.** Agents edit ordinary files and commit at
   meaningful milestones; OrgIntel refers to outputs by path / repo+commit / worktree+branch / URL —
   not via custody. §2.4, §5.3, §5.4, §6.3.
4. **Infisical adapter behind a kernel-owned `SecretBroker` trait** (file-loader as a second impl).
   Zero references today. §3.2.
5. **Company-level dollar/token cost accounting** against a budget, with a pre-flight check. The model
   gateway bounds only request counts today. §9.6.
6. **Snapshot/restore that separates host truth from company state** — kernel receipts survive every
   restore while internal state can roll back. Not expressible in the prior single-schema design. §9.2, §9.4.
7. **Persistent Exec process** with durable inbox/memory continuity across model-session restart (the
   prior system cold-starts every turn). §4.3, §7.1.

---

## First outcome target

The first sprint should target one real outcome to validate the slice, not breadth. Per ARCHITECTURE.md
§10, the reference company is **Cosmon** (a small game studio). The smallest useful first outcome is a
**browser-deployable build of a minimal exploration–encounter–capture loop** (one zone, one creature) —
a deliberate shrink of §10.2's first milestone. This exercises ambiguous creative decomposition,
multi-agent code production, Git branches/worktrees, cross-disciplinary integration, and recovery —
without premature MMO infrastructure, and without Aris/Thymelake/multiplayer/hosting.
