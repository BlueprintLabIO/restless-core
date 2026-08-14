# Restless — working agreement

Restless is an autonomous-company control plane for ordinary owner/operators. The product runs the
business for you: the human provides judgement, taste, sign-off, and the prepared last mile; the
singleton Exec and Staff do the work. This repo is a **clean-slate rebuild** whose target architecture
is `ARCHITECTURE.md` (the v0.9 source of truth). Read `ARCHITECTURE.md` before designing or coding
anything — it is the authority, this file is the working agreement for operating inside it.

A prior implementation exists elsewhere (the "legacy" control plane: a single 94-variant universal
`Command` enum as the only write path, an append-only ledger for every mutation, a content-addressed
asset-custody machine, a per-turn disposable sandbox). It is a **salvage source and reference**, not a
baseline to extend. `docs/SALVAGE.md` records which of its components are proven and may be lifted.
Do not re-import its patterns wholesale — see "What we deliberately leave behind" below.

Complexity is weight. Keep the product as lightweight as possible while still accomplishing its outcomes.

Most of the ways this repo goes wrong reduce to three failures: hedging against risks nobody named,
reaching for a tool that does not match the problem, and committing to the first approach while
keeping every approach. The counter, in one line:

> **Name the risk and accept most of them. Classify the problem before choosing the tool. Branch, run,
> then purge to one canon.**

This is the working stance. Read [`LLM_CURE.md`](./LLM_CURE.md) before designing or coding — it is the
canonical home for these frames, why each failure mode happens, and what has already been tried.

---

## What carries over from the legacy agreement (still true)

These are product soul, not legacy mechanics. They survive the rebuild.

- **The value proposition is that the product runs the business for you** — not "just a record" or an
  audit trail with a chat box. Exec and Staff perform every machine-doable step. Do not dilute this
  into a risk-reduction story.
- **Let intelligence do the work.** Where judgement, language, or understanding is involved, route it
  through the model (the singleton Exec and Staff runtimes over ACP). Do not replace intelligence with
  static flows, canned scripts, wizards, regexes, or hard-coded branching UI.
- **Safety and authority are the substrate, not the pitch.** Commands, policy, approvals, and the
  audit record exist so a self-running business stays accountable. Present them as what makes the power
  safe, never as the main event.
- **Probe, never guess.** Any claimed capability — "this runtime is connectable", "this tool works",
  "this integration is live" — must come from a live check against the real thing, never a hard-coded
  assumption that can silently go stale. If a live check is impossible, say so honestly in the UI.
- **Bring the prepared last mile to the CEO.** The user is the human CEO. Exec and Staff do every
  machine-doable step. When personal identity, sign-in, CAPTCHA, 2FA, legal attestation, payment
  confirmation, or irreducible human judgement is required, preserve the prepared state and bring the
  exact browser session, link, or bounded confirmation to the CEO — never hand the surrounding workflow
  back as instructions, and never ask the CEO to report completion when the system can observe and
  resume it. Human participation and approval are separate authority boundaries.
- **Primary experience:** a calm main work surface plus a right-hand executive chat that can focus,
  explain, and act on the main surface. Never a sidebar-heavy agent/task administration dashboard.
  Show outcomes, decisions, risk, and next actions first; reveal roles, prompts, skills, permissions,
  spend detail, workflow IDs, and logs only on request.
- **Prefer mature open-source components** (Linux, OCI, Git, Postgres, established process
  supervision, browser infrastructure). Do not build a custom database, container runtime,
  content-addressed custody protocol, or durable workflow engine unless a demonstrated workload requires it.
- Brand-neutral code and protocols; display names come only from a brand config applied in one place.

## What we deliberately leave behind

The legacy control plane made these architectural choices. `ARCHITECTURE.md` §3.4 and §12 name them as
the core anti-patterns. **Do not recreate them here.** This list exists because muscle memory and the
legacy `CLAUDE.md` will otherwise pull agents back toward them.

- **No universal command type for all mutations.** The kernel does NOT own a single `Command` enum that
  every mutation flows through. Internal coordination (tasks, teams, work, messages, schedules) is
  ordinary recoverable state in OrgIntel, not a kernel command. Only the kernel's own concerns
  (identity, authority, secrets, external effects, budgets, lifecycle, recovery) are kernel-governed,
  and even those need not share one universal algebra. See ARCHITECTURE.md §3.4, §4.7, §12.
- **No append-everything immutable ledger.** The kernel records only governance-relevant truth
  (authority changes, approvals, grants/revocations, material effects, receipts, budgets, lifecycle).
  Internal messages, thoughts, file edits, task transitions, and process events are NOT permanently
  governed. The OrgIntel operational event stream may be compacted, repaired, or regenerated. §3.2, §4.4, §12.
- **No governed asset lifecycle / content-addressed custody.** Work is ordinary files. OrgIntel refers
  to outputs by path, repo+commit, worktree+branch, or URL — it does not export/import/materialise/
  reattach artifacts through a custody state machine. Git records meaningful checkpoints; it is not a
  real-time transaction system. §2.4, §5.3, §5.4, §6.3, §9.7, §12.
- **No bespoke durable workflow engine.** Use mature process supervision. Model Work/Attempt/retry
  orchestration only after a real workload proves the need, and keep it in OrgIntel as recoverable,
  overridable coordination — not deterministic kernel policy. §2.6, §4.5, §5.6, §12.
- **Not a per-turn disposable sandbox.** The Company Linux Runtime is a persistent company computer
  (real home, Git repos/worktrees, browser, tools, project services) that survives across turns and
  restarts. Agents are ordinary processes, not cold-started containers that die on release. §5, §17 step 2.
- **No speculative generality.** Grow entities, state machines, services, and protocols only after
  repeated real scenarios reveal the same need. Do not pursue feature parity with the legacy system.
  Pursue one successful real-company outcome. §16.1, §16.3, §16.6, §17.

## The document set — read the one your work touches

`ARCHITECTURE.md` is the cross-plane view and stays the authority on how the whole system fits
together. Six specs decompose it into planes and fill in the detail; each declares it as their parent.
They are **detail, not competition** — where a spec and `ARCHITECTURE.md` disagree, one of them is
wrong, and §16.10 applies: a real run beats both documents.

**Do not read them all.** They total ~240KB. Read the one your work touches.

| Working on | Read |
|---|---|
| Actors, goals, commitments, messages, wakes, context assembly, staff, org health | `docs/specs/orgintel.md` |
| Effects, receipts, budgets, credentials, approvals, runtime lifecycle authority | `docs/specs/authority-plane.md` |
| The company computer, container, image, Runtime Bridge, process lifecycle | `docs/specs/company-runtime.md` |
| Anything the owner sees: attention, approvals, work and people surfaces | `docs/specs/owner-cockpit.md` |
| Anything crossing two planes: identifiers, statuses, who owns which concept | `docs/specs/cross-layer-contract.md` |
| Proving it works: baselines, success contracts, dogfood scenarios, evidence | `docs/specs/evaluation-dogfood.md` |

**One concept, one authoritative owner.** When two specs seem to cover the same thing, the
cross-layer contract's §3.1 table is the tiebreaker. A layer may hold identifiers, summaries and
projections of another layer's concept; it may never become a second writer of it.

**Respect the labels.** Every spec marks its content: *Core contract* (implementation must preserve
this), *Product hypothesis* (dogfood must test this; it may be wrong), *Default pattern*
(recommended, overridable), *Example* (illustrative, not scope). Building a **Product hypothesis** as
though it were a **Core contract** is how a spec turns into speculative machinery — it is the same
failure as building before evidence, wearing a citation.

Two known defects in the set, pending a pass: the specs still call the parent *"Helm Architecture
Source of Truth v0.9"* (its former name — the file is `ARCHITECTURE.md`, titled *Restless*), and they
carry versions in their filenames, which guarantees that a v0.4 either breaks every link or
accumulates silently beside a stale v0.3. That already cost us once: OrgIntel v0.2 and v0.3 disagree
on where OrgIntel lives, and nothing in the repo said which was live.

## How we decide

Failure modes and their cures live in **[`LLM_CURE.md`](./LLM_CURE.md)**, which is canonical for this
and should be read before designing or coding. It carries three frames, the diagnosis behind each, what
we already know does not work, and the levers that are not written rules. In brief:

1. **Name risks and give each a disposition** — accepted, pending fix, guarded, or invariant. Default
   to accepted; invariant is reserved for irreversible harm. Paranoia is what unnamed risk turns into.
2. **Classify the problem before choosing the tool** — deterministic or judgement, enumerable or
   open-ended. Regex where judgement belongs and a state machine where one does not fit are the same
   error: misclassifying the problem.
3. **Branch, gather evidence, then purge to one canon** — tunnelling skips the branch, accumulation
   skips the purge. They are one loop, and the half-executed version is worse than neither.

## How we work — sprint-driven, two founders

This repo is built in sprints by two founders collaborating on the `dev` branch. The cadence is:

> `ARCHITECTURE.md` (target) → **sprint spec** (founders align) → coding agents break it into tickets
> → founders align on tickets → implement as a goal-mode sprint on `dev`.

- **`ARCHITECTURE.md`** is the high-level architecture and source of truth. It is short enough to
  reread and challenge. Do not turn it into an exhaustive executable specification; record unresolved
  design as open questions (its §14) or ADRs, not premature code.
- **Sprint specs** live in `docs/sprints/sprint-NN.md`. A spec states the outcome, acceptance criteria,
  and the slice of each layer (Kernel / OrgIntel / Runtime) it touches. A slice is complete only when
  it produces a useful artifact, decision, or external outcome — a schema, API, or invariant suite alone
  is not a successful slice (ARCHITECTURE.md §16.2).
- **Tickets are files in the repo**, one per ticket under `docs/sprints/sprint-NN/`, indexed by a status
  checklist in the sprint spec. Agents are the primary readers of tickets, and a file costs no tool
  call, no auth, and no network; it is also versioned alongside the code that implements it and
  reviewable in the same PR the founders align on. Ticket status lives **only** in the spec checklist.
  When breaking a sprint into tickets, coding agents must cite the observed outcome or friction each
  ticket serves (§16.7), name which layer it belongs to and why, and state what prior machinery — if
  any — it makes deletable.
- **Branches.** `main` is the default branch and the foundation; `dev` is the sprint integration line.
  Founders integrate on `dev`. Short-lived `feat/*` / `fix/*` branches PR into `dev`. Do not long-run
  feature branches. `main` is reserved for what is releaseable.
- **Slices before layers.** During the walking skeleton, one small cross-functional effort owns all
  three layers end to end (§16.9). Avoid separate kernel/runtime ownership until contracts are proven.
- **Observe before modelling.** Let behaviour occur through files, messages, and processes first.
  Introduce a first-class entity, lifecycle, or durable protocol only after repeated real scenarios
  reveal the same need (§16.1, §16.6).
- **Deletion is product progress.** After each sprint, identify abstractions, adapters, tables,
  protocols, and tests that no longer improve a live outcome. Removing them is part of completing the
  work, not optional cleanup (§16.5, §16.6).
- **Stop tunnelling.** If internal types/states/invariants dominate while the target company behaviour
  stays vague, or test-count is being treated as progress, return to a real company run (§16.11).

## Repo conventions

- **Rust workspace.** For now, only two empty binary crates exist: `restlessd` (the daemon, the
  "stable coordination core" of §4.4) and `restless` (the CLI). Layer crates (`kernel`, `orgintel`,
  `runtime`) are NOT pre-scaffolded — they are grown from the first slice that needs them, per §16.1.
- **The operator SPA is in `web/`** — lifted from the prior control plane (`cf8a028`): design layer,
  primitives, composed surfaces and every page, rendering from `$lib/fixtures`. It carries no truth;
  wiring it means swapping the fixture for a read client and passing real write callbacks. Its read
  model, `web/src/lib/model/view.ts` (`DeskView`), is the de facto owner-surface contract — read it
  before designing anything owner-facing. **Brand config is not in this repo yet** (cofounder ports
  branding manually). Keep code and protocols brand-neutral so a configured name is applied in one place.
- **Pushing** is owner-only; never `git push` without being asked.
- **Testing style** (carried forward): add automated tests only for key product invariants and
  security/data-integrity boundaries. Do not add tests for implementation details, trivial wiring,
  styling, snapshots, or every branch merely to increase coverage. Layer-appropriate testing (§16.8):
  Kernel gets focused invariants + adversarial tests; OrgIntel gets behavioural/recovery scenarios;
  Runtime gets real-tool integration and dogfood producing concrete artifacts. Test cases cost compute!
- **Verifying.** Prefer headless verification (CLI, API, service-level probe, headless run) with stated
  inputs and expected outcome. Manual visual inspection supplements, it does not replace, a feasible
  headless check.
- **Never report green without running it.** No component is described as working — in a commit
  message, spec, or summary — unless it has been executed with stated inputs and observed output.
  "Compiles" is not "works"; "tests pass" is not "the company produced the artifact".
