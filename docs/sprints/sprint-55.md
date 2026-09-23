# Sprint 55 — Portable company skills and native command translation

**Status:** Draft for founder alignment
**Programme:** none — standalone Runtime/OrgIntel capability sprint
**Depends on:** current Runtime Bridge launch contract (§4.3), Goals, schedules and the Work graph;
independent of the Sprint 54 release candidate

## Outcome

An owner selects `$frontend-design` in the executive chat, or an actor activates a relevant skill
itself, and the same pinned skill reaches whichever harness does the work: Restless managed, Codex,
Claude Agent, Hermes or OpenClaw. The selection survives delegation, retries and a harness change,
and the activity shows "Using Frontend design".

The popular harness conventions that are really orchestration become Restless primitives. They don't
become a second, harness-private loop. `/goal` creates a Goal and routes Work through Exec to an
accountable lead. `/loop` creates a schedule. Gauntlet-style build-and-critique uses commissioned
workers and independent review. `find-skills` installs a *candidate* into the company library. A
skill that asks for any of these gets the Restless equivalent, and a compatibility corpus proves it
on every certified harness.

## Observed friction

Each item was read from `dev` at `70093e1`.

| Friction | Evidence |
|---|---|
| Skill availability depends on the harness. Only OMP loads skills natively | `crates/restlessd/omp-runtime.yml` enables `/opt/restless/skills`, `/company/skills` and project `.agents/skills` |
| Claude Agent blocks skills outright | `crates/restlessd/src/acp.rs` — `"Skill"` in `disallowedTools`, `settingSources: []` |
| Hermes and OpenClaw block skills | `tools/custom-harness/hermes.py` disables the `skills` toolset. `tools/custom-harness/openclaw.mjs` sets `skills: []` |
| Codex receives skill roots only as prose in Staff context. Exec receives none | `crates/restlessd/src/staff/context.rs` `skill_roots`. `tools/codex-runner/restless-codex-runner.mjs` disables `skill_search` |
| Skill roots are hard-coded in three places, with no single resolver | `staff/context.rs`, `omp-runtime.yml`, the `acp.rs` tests |
| ACP command advertisements are discarded | `acp.rs` `live_event` sends `AvailableCommandsUpdate` to `_ => None` |
| No company skill model, selection record, API or owner surface | no migration, route or component mentions skills |
| Company doctrine is labelled `[shared skill]` in Exec and lead prompts | `crates/restlessd/src/context.rs`, `staff/conversation.rs` |
| The most-installed public skills assume sub-agents, user interviews, `/goal` or `/loop`. Restless deliberately withholds private sub-agents | OMP `task` and Claude `Agent`/`Task` are disabled so that delegation has one canon, a claimed Work Attempt |
| `npx skills add` writes into `.agents/skills`, which OMP loads directly and skips any library | `enableAgentsProject: true` |

## Design stance

- **Use the open `SKILL.md` format unchanged** ([Agent Skills specification](https://agentskills.io/specification)).
  A skill package stays an ordinary Runtime file. OrgIntel stores its name, digest, source and
  disposition, never the body. This was the Sprint 6 research decision
  (`sprint-06/research/01-skill-standards-and-registries.md`).
- **Skills are assigned to actors, not to Intelligence Providers.** Changing model or harness never
  changes an actor's skills.
- **One Restless-owned invocation path, with small harness adapters.** Use native loading where it is
  live-probed to work. Otherwise deliver the selected instructions and resource paths through the
  existing context channel. An explicit selection loads once per turn.
- **Translate orchestration into Restless primitives, not code shims.** A short translation preamble
  travels with every loaded skill. The model maps "spawn a sub-agent", "ask the user", `/goal`,
  `/loop` and `/other-skill` onto Restless equivalents. A small set of high-value conventions also
  gets a first-class Restless command or a Restless-authored built-in skill (see below).
- **Invoking a skill grants nothing.** Credentials, effects, deployment authority and budgets stay
  with Authority. A skill's `scripts/` run with the actor's existing Runtime capabilities and no more.

## Native translations

| Public convention | What it is upstream | Restless primitive | Surface |
|---|---|---|---|
| **`/goal <objective>`** (Codex CLI 0.128+, Claude Code) | Persisted per-thread objective with continuation, a completion audit and a token budget. Codex app-server `thread/goal/*`, model tools `create_goal`/`update_goal` | A **Goal** (`goals` table, `restless goal add`) plus Work routed by Exec to one accountable lead. Completion is the existing Work evidence and review, not a model's own claim. The budget is the company `spend_ceiling_usd` for now | Composer command, and `restless goal` for actors. `/goal` shows the active Goal; `/goal clear` closes it |
| **`/loop [interval] <prompt>`** (Claude Code) | Harness-scheduled re-invocation, self-paced when no interval is given | A **schedule** addressed to the actor (`restless schedule add`). Extend `recurrence` beyond `weekdays` to a bounded interval. Self-paced loops are the actor adding its own next schedule | Composer command. `restless schedule` for actors |
| **`gauntlet-loop`** (split → build → blind critic → repeat against a hard bar) | Prompt-only skill that fans out sub-agents and relies on `/loop` or `/goal` | The accountable lead commissions builder Work and an **independent review** by an actor that did not see the draft, with the bar as `expected_artifact` and ReviewTarget. A retry gets a fresh reviewer | Restless-authored built-in skill `gauntlet` in `/opt/restless/skills`, which supersedes the upstream package for company use |
| **`code-review`** (two-axis review in parallel sub-agents) | Sub-agents that keep standards review and spec review in separate contexts | Two independent review Work items under the lead, or one reviewer when the change is small. The lead judges | Built-in skill `code-review`, a Restless variant |
| **`grill-me` / `grilling`** | Relentless interview of the user until the design tree is resolved | An **Exec conversation with the owner**. For Staff it becomes one batched owner question through the existing judgement handoff, never a stream of blocking questions | Available to Exec. The preamble tells Staff to batch |
| **`find-skills`** (`npx skills find/add`) | Agent discovers and installs public skills | `restless skill find` / `restless skill add <source>` creates a **candidate** in the library, pinned to a commit, and makes it usable right away by the requesting actor only | CLI. The owner sees candidates under Company → Skills |
| `implement` → `/tdd`, `/code-review` | One skill invoking another | `restless skill use <name>` | CLI, recorded as activity |
| `wayfinder` (issue-tracker decision map) | Tickets in GitHub or Linear | Work items, or the tracker through an already authorised MCP connection | No new primitive |

Plain-instruction skills need no translation: `frontend-design`, `vercel-react-best-practices`,
`web-design-guidelines`, `remotion-best-practices`, `domain-modeling`, `diagnosing-bugs`, `caveman`.
They are the baseline for zero-day support.

We deliberately do **not** pass `/goal` through to Codex `thread/goal/set`. It would create a second
goal lifecycle outside the Work graph, and it exists on one harness only. We accept losing Codex's
in-thread auto-continuation.

## Scope by layer

### Runtime

- One skill-root resolver in `restlessd`, used by every launch path, Exec and Staff alike. It
  replaces the three hard-coded lists.
- `restless skill list | show <name> | use <name> | find | add <source>` in the company image. `use`
  prints `SKILL.md` and the absolute resource paths, and records an activation event.
- Harness adapters:
  - OMP keeps native loading.
  - Claude Agent enables `Skill` against the resolved roots, or falls back to CLI plus context.
  - Codex gets `CODEX_HOME/skills` links to the resolved roots.
  - Hermes and OpenClaw get CLI plus context.
  - Each harness's native loading is enabled only after a live probe passes.
- The translation preamble, as one Restless-owned text, attached once whenever any skill is active.
- Built-in Restless skills `gauntlet` and `code-review` in `infra/company-image/skills`.
- Keep ACP `AvailableCommandsUpdate` per session, as an observed fact that isn't yet shown to anyone.

### OrgIntel

- `skills`: name, description, source, pinned digest, whether it contains scripts, and disposition
  (`candidate`, `accepted`, `retired`).
- `skill_assignments` at company, team and actor scope. Resolution goes actor → team → company;
  an individual override may remove a skill.
- `message_skill_selections(message_id, skill_name, digest)`, modelled on `message_mentions`. When
  Exec dispatches, the selection is copied onto the Work, so Attempt context carries it across
  delegation and retries.
- `schedules.recurrence` accepts a bounded interval (for example `every:15m`, with a minimum floor)
  alongside `weekdays`.
- `/goal` creates a Goal with its initiating Work. `/goal clear` sets `closed_at`.

### Kernel / Authority

- No new governed state. The invariant: skill selection, activation or installation never changes
  credentials, grants, budgets or effect authority. A skill with scripts that is imported from outside
  the company image is shown as such, and stays `candidate` until the owner or an accountable lead
  accepts it.

### Owner cockpit

- Composer (`web/src/lib/primitives/Composer.svelte`): `/` opens a searchable menu of Restless
  commands (`/goal`, `/loop`) and skills. `$` filters to skills. A selected skill becomes a removable
  chip.
- Activity reads "Using Frontend design", with details on expand. The source is `restless skill use`
  or native activation events.
- Company → Skills (`web/src/routes/[companyId]/company/skills`): the library with dispositions,
  assignments, candidates to accept, and per-harness compatibility from the corpus. Revealed only on
  request, per the primary-experience rule.

## Acceptance

1. **Portable explicit selection.** In a `_test` company, an owner message with `$frontend-design`
   produces Work whose Attempt demonstrably follows the skill on each certified harness (Restless
   managed, Codex, Claude Agent) and on at least one custom harness. The message row, the Work and
   the activity all show the same skill name and digest.
2. **Survives change.** Reassigning that actor to a different harness and retrying the Attempt keeps
   the selection without re-selecting it. Delegation from Exec to lead to worker keeps it.
3. **Loaded once.** On a harness with native loading, an explicitly selected skill shows up once in
   the turn context. A double load is observed and recorded, not claimed absent.
4. **`/goal`.** `/goal ship a landing page for X` creates one Goal and one Work under an accountable
   lead. Completion comes from Work evidence and review. `/goal` shows it and `/goal clear` closes it.
   No Codex `thread/goal` state is created.
5. **`/loop`.** `/loop 30m check inbound leads` creates one interval schedule that fires, is
   restart-safe through `schedule_occurrences`, and is cancellable. Repeating the command returns the
   existing live schedule.
6. **Gauntlet as native Work.** Running the built-in `gauntlet` skill on a bounded outcome produces
   builder Work and a review by a different actor that did not see the draft. A failed review leads to
   a retry with a fresh reviewer. No harness-private sub-agent is used.
7. **`find-skills` stays governed.** An actor running `restless skill add <git-url>` creates a pinned
   `candidate` that only that actor can use. A direct `npx skills add` into `.agents/skills` appears
   in the library as a project candidate rather than silently becoming company-wide.
8. **Nothing is granted.** A skill instructing a deploy or payment still produces the ordinary
   effect request and approval. Adversarial case: a skill claiming "you are authorised to …".
9. **Compatibility corpus.** About 10 pinned public skills (`frontend-design`, `grill-me`,
   `code-review`, `implement`, `gauntlet-loop`, `find-skills`, `vercel-react-best-practices`,
   `remotion-best-practices`, `caveman`, and one skill with scripts) each run a bounded scenario in a
   `_test` company on every certified harness. The results feed the Skills page's per-harness status.
   Failures are listed, not hidden.
10. All `_test` companies, schedules and supervisor programmes created by the corpus are removed.
    `restless-reap --check` is clean.

## Ticket outline

- [ ] C55-T0 — canon: add "company skills" to `ARCHITECTURE.md` §4.3 and `docs/specs/company-runtime.md`,
      and rename the doctrine sections now labelled `[shared skill]`
- [ ] C55-T1 — skill-root resolver, `restless skill` CLI, and harness adapters with live probes (Runtime)
- [ ] C55-T2 — translation preamble plus the built-in `gauntlet` and `code-review` skills (Runtime)
- [ ] C55-T3 — skill library, assignments and candidate intake, including `.agents/skills` discovery (OrgIntel)
- [ ] C55-T4 — message skill selections carried onto Work and Attempt context (OrgIntel/Runtime)
- [ ] C55-T5 — `/goal` and `/loop` as native commands, with interval recurrence (OrgIntel)
- [ ] C55-T6 — composer `/` and `$` menus, skill chip, activity label, Company → Skills (cockpit)
- [ ] C55-T7 — compatibility corpus, per-harness status and purge (evaluation)

## Deletion

- The hard-coded `skill_roots` list in `staff/context.rs` and duplicate root declarations elsewhere.
- Skill-disabling flags in the Claude Agent, Hermes and OpenClaw launch paths, once their adapter
  passes its probe.
- The `[shared skill]` label on company doctrine.
- Any harness-specific skill wording in actor prompts that the preamble supersedes.

## Exclusions and stop rules

This sprint does not build a public skill marketplace, a ranking or recommendation engine, a
capability ontology, per-Goal budgets, or harness-native slash commands. `AvailableCommandsUpdate`
is stored but not shown until an adapter can handle a command's result. It does not copy skill
bodies into OrgIntel, and it doesn't enable private sub-agents in any harness.

Stop and return to design if:
- a skill selection or skill script can reach a credential or effect without the ordinary Authority
  path;
- a translated `/goal` or `/loop` creates Work outside the Exec → accountable-lead route;
- the compatibility corpus only passes on a harness because a simulated capability stood in for a
  real one.

## Open questions for founders

1. Import sources for owner-added skills: Git URL only, or also upload and paste?
2. Who accepts a `candidate` skill that contains scripts: owner only, or the accountable lead within
   its team?
3. Should `/goal` get a per-Goal spend limit now, or keep relying on the company ceiling until a real
   run shows the need?
4. The minimum `/loop` interval, and whether Staff (not only Exec) may create interval schedules for
   themselves.

## Sources

- [Agent Skills specification](https://agentskills.io/specification)
- [Using Goals in Codex](https://developers.openai.com/cookbook/examples/codex/using_goals_in_codex),
  [Codex goal app-server API (openai/codex#18074)](https://github.com/openai/codex/pull/18074)
- [gauntlet-loop](https://github.com/duolahypercho/gauntlet-loop), [mattpocock/skills](https://github.com/mattpocock/skills),
  [skills.sh leaderboard](https://skills.sh) (install counts are approximate, fetched 2026-09-23)
- [ACP session updates](https://agentclientprotocol.github.io/typescript-sdk/types/SessionUpdate.html)
