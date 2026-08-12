# Sprint 01 — Walking skeleton, three companies

**Status:** Draft for founder alignment
**Date:** 12 August 2026
**Architecture refs:** §15 (immediate validation slice), §16.2, §16.9 (slices before layers),
§17 step 2 (walking skeleton), §17 step 5 (second materially different outcome), §10.7 (dogfood
portfolio), §10.8 (simulated external world)

---

## Outcome

> **A thin end-to-end skeleton of all three layers, running three materially different reference
> companies from one owner directive each, producing a concrete artifact per company — plus a friction
> report that tells us which abstractions are real and which are overfit.**

This is `ARCHITECTURE.md` §17 step 2 (walking skeleton) with §17 step 5 (a second materially different
outcome) pulled forward into the same sprint. The reason for pulling it forward: an ontology shaped by
one company for three sprints is an ontology we cannot evaluate. Running Cosmon, Aris and Thymelake
together is the cheapest available test of whether the small §4.4 vocabulary survives building,
selling, and operating.

### The primary measurement

**Build the skeleton against Cosmon. Then add Aris and Thymelake as configuration, prompts and
simulated connectors. The cost of adding companies 2 and 3 is itself the sprint's main result.**

If they are cheap, the abstractions are real. If either needs bespoke engineering, that finding is
worth more than any artifact this sprint ships, and it directly answers §14 open question 12 (which
company should provide the ongoing walking-skeleton dogfood).

---

## Deliberate posture: no governance this sprint

Grants, delegation, capability checks, approvals, receipt reconciliation, Infisical, and
snapshot/restore are **all out**. They are ceremony around a company that is not yet touching anything
real, and building them now would consume the sprint that is supposed to teach us what the company
actually does.

Two things are kept, and neither is governance:

- **A model spend ceiling.** A fuse, not a safety feature. Three companies of autonomous Execs looping
  unattended is how an unbounded bill happens. It is a counter and a comparison.
- **The provider key living outside the company container.** Near-zero cost now (calls route through a
  local gateway); miserable to retrofit once three company images assume ambient environment variables.

**Why deferring the rest is safe here specifically:** blast radius this sprint is bounded
*environmentally*, not architecturally — every external provider is simulated and nothing real is
connected. This posture expires the moment the first live provider is wired up, which is the trigger
condition for the governance sprint.

---

## Acceptance criteria

Headless where practical, with stated inputs and expected results (CLAUDE.md → "Verifying").
Manual play supplements but does not replace a feasible headless check.

### Per company

| Company | Pass bar | Headless check |
|---|---|---|
| **Cosmon** | A playable local browser build of a minimal exploration → encounter → capture loop, committed to Git under `/company/repos` | A Playwright script drives the build through the loop's key transitions (move → encounter fires → capture resolves) against the served build |
| **Aris** | A simulated prospect carried from segment choice to a simulated purchase, with the funnel legible | Scripted run asserts: offer artifact exists, outreach effect requested and receipted, simulated reply handled, purchase effect receipted |
| **Thymelake** | A simulated restaurant carried from prospect to a processed test order | Scripted run asserts: menu artifact configured from simulated input, QR deploy effect receipted, simulated order processed and acknowledged |

Cosmon needs **no art pipeline** — three.js primitives or 2D, procedural only. Original assets are a
later milestone; the slowest, weakest agent loop is not what this sprint is measuring.

### Cross-cutting

1. **Persistence.** `restless down && restless up` preserves `/company` — files, Git history, browser
   profile — for each company.
2. **Continuous autonomous work.** One directive drives multiple turns with no owner input, until the
   milestone completes or the company is genuinely blocked.
3. **Crash recovery.** Kill the Exec mid-turn and kill a staff process mid-turn. Both restart, rehydrate,
   and *continue the milestone* rather than restarting it. Committed work is intact in every case.
4. **Restart recovery.** Restart `restlessd`. Running companies resume against recoverable coordination
   state; already-produced files and commits remain valid (§4.8).
5. **Spend fuse.** With a deliberately tiny ceiling, the gateway fails closed and the company pauses
   inspectably rather than crashing.
6. **Key isolation.** Grep container environment and filesystem: no provider API key present. Model
   traffic is observable at the gateway.
7. **Human judgement point.** Each company surfaces at least one genuine decision to the owner and
   blocks until answered via the CLI (§10.5 step 9, §15 item 8).
8. **Recorded per company:** elapsed time, dollar cost, owner-intervention count, and the friction list.

---

## Layer slices

### Kernel — thin, but the seams exist

**In:**
- Company identity and configuration as files, not tables. One config per company.
- Model access through a gateway: provider key held host-side, injected server-side; dollar/token
  accounting against a per-company ceiling; fail closed at the ceiling.
- **Effect surface, not effect gate.** `request_effect(capability, args, idempotency_key) -> receipt`,
  routing to a simulated provider. The interface the governed version will eventually have; the
  approval and grant checks are simply absent.
- Company environment lifecycle: up, down, status.

**Out:** grants, delegation, capability checks, approvals, standing grants, Infisical, outcome-unknown
reconciliation, snapshots, kernel Postgres.

**Note on why the effect surface is not optional:** Aris and Thymelake *are* external effects — send
email, take payment, deploy a menu, process an order. Without an effect surface those two companies can
only write documents to each other, and the sprint learns nothing about them.

### Simulated external world (§10.8)

Only the **behavioural** simulators are in scope. The deterministic simulators exist for kernel
correctness, and this sprint has no kernel correctness to test.

Behavioural simulators are **model-driven** — a prospect who replies and objects, a restaurant that
sends a messy menu and complains about a wrong order. This is a prompt behind a fake connector, not a
second product. The company must not need different logic for a simulated provider.

### OrgIntel — the layer we are actually here to learn about

**In:**
- Actors and sessions (Exec + bounded staff).
- The small §4.4 ontology: goal, commitment, actor, artifact reference, decision. Commitments in five
  states: proposed, active, blocked, completed, abandoned.
- Messages and inboxes: assignments, updates, questions, review requests, decisions.
- Scheduler: periodic Exec planning ticks **and** event-driven wakeups on dependent results.
- Context assembly on wake.
- Operational event stream, for the CLI to watch.
- Storage: **Postgres, one schema per company.** Ordinary recoverable company state, explicitly outside
  the constitutional trust boundary (§4.9). This reverses an earlier no-database position: with three
  companies × multiple actors × messages × schedules, files stop being the cheaper option.

**Out:** resource allocator, team designer, stagnation detector, organisational historian, playbook
versioning, competency estimates, WIP limits, review routing. All §4.5 intelligence modules beyond a
minimal Exec planner wait for observed friction.

### Runtime

**In:**
- A standard company image (Debian-derived), one persistent container per company.
- Persistent `/company` volume surviving container and daemon restarts.
- ACP agent processes for Exec and staff, as ordinary supervised processes.
- Git repositories and worktrees; separate worktrees for code-producing staff (§9.7).
- Browser and shell.
- Filesystem conventions per §5.3, kept deliberately evolvable.

**Out:** per-turn disposable containers, custody, leases, project-level container isolation, desktop
session polish, snapshot/restore.

### Owner surface

CLI only — no SPA in this repo yet. `up`, `down`, `status`, `tell "<directive>"`, `watch`, `attach`,
and inspection of goals, commitments and inboxes.

---

## Judgement vs determinism register

Recording this explicitly because it is the most product-defining set of choices in the sprint, and the
easiest to get wrong by reflex.

**Model call — these must never become regex, keyword lists, or threshold heuristics:**

- Directive → bounded milestone decomposition.
- Staffing: how many staff, what shape, when to spawn.
- Whether a commitment is genuinely blocked, or merely quiet.
- Whether two pieces of work are duplicates.
- Context selection on wake.
- All simulated customer, prospect and restaurant behaviour.
- Continue / pivot / stop recommendation to the owner.
- Run report synthesis.

**Deterministic — no model in the path:**

- Spend counter and ceiling comparison.
- Idempotency key generation and effect receipt recording.
- Process supervision, restart, and health signalling.
- Git operations and file IO.
- Scheduler firing and event stream append.
- Container lifecycle.

---

## Salvage

Per `docs/SALVAGE.md`. Every lift is an extraction task with a re-validation step, not a copy.

| Lift | Used for | Re-validation |
|---|---|---|
| **Model gateway** (`company-model-gateway`) | Kernel model-credential isolation | Standalone crate already; re-validate after adding the missing company-level dollar dimension. Confirm fail-closed at ceiling with a live tiny-ceiling run. |
| **Adapter image package list** (`infra/sandbox-agent/Dockerfile`) | Starting point for the company image | Strip single-entrypoint and tmpfs-home assumptions. Confirm the image runs as a persistent multi-process company computer with a real `/company`. |
| **Pure ACP session lifecycle** (inside `contained.rs`) | Exec and staff processes | **High extraction friction** — see open decisions. Whichever path wins, validate by live-probing a real agent binary end to end (initialise → session → prompt → output). |
| **Context assembly** (`context.rs`) | Context package on wake | Lift the deterministic-snapshot + digest idea against the new OrgIntel read model. Drop kernel aggregate-version pinning. |
| **Outbox / LISTEN-NOTIFY transport** (`worker/delivery.rs`) | Event-driven wakeup transport only | The Work/Attempt state machine wrapping it is **not** reused. Validate that a dependent result wakes the right actor. |
| **Directed messaging** (`communication.rs`) | Messages and inboxes | Strip the universal-command envelope. Validate against real Exec↔staff traffic. |
| **Black-box golden scenario shape** | The acceptance harness | Keep the scenario shape and the cleanup-proof wrapper; rewrite the driver against this sprint's CLI. |

**Deliberately not lifted:** the external-effect broker in full. It is the most proven component
available, but its value is its governance, and governance is out this sprint. We take the interface
shape only.

---

## Proposed tickets

Tickets are files in [`sprint-01/`](./sprint-01/). Each names its layer, the outcome or friction it
serves, and what it makes deletable (§16.7). **This checklist is the only place ticket status lives** —
tick the box and note the commit.

Dependency order, which is how they should be worked:

```text
T3 ACP spike ──┬── T1 container ──┬── T4 Exec ── T7 context ── T6 scheduler ── T9 staff ──┐
               │                  │                                                        ├── T11/12/13 ── T14 ── T15
               ├── T2 gateway ────┘                                                        │
               └── T5 OrgIntel ──── T8 effects ── T10 CLI ─────────────────────────────────┘
```

| ✓ | Ticket | Layer | Depends on | Commit |
|---|---|---|---|---|
| [ ] | [T1 · Company image + container lifecycle](./sprint-01/t01-company-image.md) | Runtime | — | |
| [ ] | [T2 · Model gateway + spend fuse](./sprint-01/t02-model-gateway-spend-fuse.md) | Kernel | 1 | |
| [ ] | [T3 · ACP session client](./sprint-01/t03-acp-session-client.md) | Runtime | — | |
| [ ] | [T4 · Persistent Exec + file continuity](./sprint-01/t04-persistent-exec.md) | Runtime / OrgIntel | 1, 3, 5 | |
| [ ] | [T5 · OrgIntel core](./sprint-01/t05-orgintel-core.md) | OrgIntel | — | |
| [ ] | [T6 · Scheduler](./sprint-01/t06-scheduler.md) | OrgIntel | 5 | |
| [ ] | [T7 · Context assembly on wake](./sprint-01/t07-context-assembly.md) | OrgIntel | 5 | |
| [ ] | [T8 · Effect surface + simulated providers](./sprint-01/t08-effect-surface.md) | Kernel | 1 | |
| [ ] | [T9 · Staff spawn and supervision](./sprint-01/t09-staff-supervision.md) | OrgIntel / Runtime | 1, 3, 5 | |
| [ ] | [T10 · CLI owner surface](./sprint-01/t10-cli-owner-surface.md) | Owner surface | 1, 5 | |
| [ ] | [**T11 · Cosmon — the skeleton is built here**](./sprint-01/t11-cosmon.md) | All | 1, 3–7, 9, 10 | |
| [ ] | [T12 · Aris](./sprint-01/t12-aris.md) | All | 8, 11 | |
| [ ] | [T13 · Thymelake](./sprint-01/t13-thymelake.md) | All | 8, 11 | |
| [ ] | [T14 · Crash and restart harness](./sprint-01/t14-crash-restart-harness.md) | Cross-cutting | 11 | |
| [ ] | [T15 · Run report, deletion pass, friction backlog](./sprint-01/t15-run-report.md) | Cross-cutting | 11–14 | |

**Start with T3.** It is the sprint's main technical unknown and it blocks T4, T9 and every company.

Since this is the first sprint, no ticket makes prior machinery deletable. The §16.7 "what this makes
deletable" slot is answered honestly as *nothing yet* — except T3, which deletes its own losing branch,
and T15, which owns the deletion pass over everything the runs did not exercise.

---

## What we are trying to learn

The friction report is a first-class deliverable, not a postscript. It is what makes sprint 02
evidence-driven rather than imagined (§16.4 friction backlog).

- Does the small §4.4 ontology survive three company shapes, or does each want its own vocabulary?
- Do event-driven wakeups actually fire, or does the Exec go silent after a dependent result lands?
- Where does owner attention get pulled in — and is it genuine judgement, or missing machinery?
- Does file + Git work survive an agent crash intact, without custody machinery?
- What did companies 2 and 3 cost to add, relative to company 1?
- Cost and elapsed time per useful outcome, per company.
- Which company is the strongest ongoing dogfood? (§14 open question 12)
- Which parts of the skeleton were never exercised and can be deleted immediately?

---

## Open decisions before ticketing

1. **ACP: extract or rewrite.** `SALVAGE.md` flags the extraction as high-friction — tangled with
   sandbox transport bridging and custody `await_result`. Working hypothesis: a fresh thin client
   against the ACP spec is *smaller* than the extraction, because a persistent container deletes the
   fence and tunnel machinery that made the original complex. **Resolve by spike, not by argument** —
   build both minimally and let the run decide.
2. **Which ACP agent binary.** The legacy image used `codex-acp`. Whichever we choose, live-probe it;
   do not assume connectability.
3. **Staff cap.** Proposed: two per company, spawned on demand. Enough to generate handoff and crash
   friction without tripling token burn across three companies.
4. **Done criteria.** Proposed: **Cosmon green plus honest findings from Aris and Thymelake is a pass.**
   Requiring all three green risks converting a learning sprint into a grinding sprint, and the finding
   is the point.
5. **Spend ceiling value**, per company and in total, and who sets it.

*Resolved: naming.* `ARCHITECTURE.md` now says "Restless" throughout, and §14 open question 1 records
`restlessd` as the settled name for the stable coordination service. Brand-neutral code remains the
standing rule — display names come from a brand config applied in one place.

---

## Risk register

Per `LLM_CURE.md` frame 1: every risk named, each given exactly one disposition. Default is accepted;
escalation takes an argument.

| Risk | Disposition | Why |
|---|---|---|
| No authority, grants, approvals, or capability checks on effects | **Accepted** | Every provider is simulated. Blast radius this sprint is bounded environmentally, not architecturally. |
| Prompt injection or a compromised worker | **Accepted** | Nothing real is connected; the worst outcome is wasted tokens and bad files, both recoverable. |
| The agents produce poor or wrong work | **Accepted** | That is the thing this sprint measures, not a failure to engineer against. |
| Loss of the OrgIntel Postgres state | **Accepted** | Recoverable coordination state by design (§4.8). Files and Git hold the actual work. |
| Provider API key leaking into a company container | **Guarded** | Key held host-side, injected at the gateway; verified by grep of container env and filesystem. |
| Unbounded model spend across three autonomous companies | **Guarded** | Per-company dollar ceiling, fail closed; staff capped at two per company. |
| ACP is the main technical unknown | **Guarded** | Spike both paths (ticket 3) before the rest of the sprint depends on the answer. |
| Three companies is real surface area | **Guarded** | Build-one-then-configure-two sequencing, plus the done criterion in open decision 4. |
| The skeleton grows past "thin" | **Guarded** | The out-of-scope lists above, plus a deletion pass before sprint 02 opens: any path no run exercised comes out. |
| Ambiguous external-effect outcomes; receipts surviving restore | **Pending fix** | Real, and the whole point of the effect broker — but it needs a live provider to be meaningful. Governance sprint. |

**When the accepted risks expire:** the first four are accepted *because nothing real is connected*.
That disposition expires the moment a live provider is wired up — which is the trigger condition for
the governance sprint, not a project phase we choose.

---

## Housekeeping

`.env` at the repo root is a leftover from the prior implementation (Helm-prefixed variables, a
`DATABASE_URL`, a duplicated `OPENROUTER_API_KEY`). It should be cleared before it becomes an
accidental dependency of the new path (§16.3).
