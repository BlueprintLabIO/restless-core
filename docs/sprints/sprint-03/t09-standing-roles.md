# S03-T9 · Standing roles — the OrgIntel demonstration

**Layer:** OrgIntel (roles, patterns, evolution records) + Runtime (the processes)
**Depends on:** nothing — but it is what makes T5's approval worth having
**Blocks:** nothing; it is the sprint's proof that OrgIntel is a product, not bookkeeping

---

## Why this ticket exists

Sprint 02 spent four attempts comparing `single_agent`, `minimal_team` and
`orgintel` and could never have learned anything, for a reason the harness could
not show: **there was no team to compare.**

Three facts, all checkable:

- `staff.rs:182` derives staff auth from `exec::agent_auth(config)` — every staff
  member runs **the same model** as the Exec.
- `staff.rs:166` writes `add_actor(&actor, "staff", …)` — every worker's role is
  the literal string `"staff"`.
- `context.rs` contains **zero** occurrences of `org_mode` — the Exec's briefing
  is byte-identical in all three modes.

So delegation means handing work to a copy of yourself with less context. The Exec
declined, correctly: that buys parallelism and nothing else. `minimal_team` and
`orgintel` are the same configuration under two names.

Meanwhile [orgintel §6.3](../../specs/orgintel.md#L647) is labelled **Core
contract** and §10.1's V0 acceptance list, item 12, requires *"initial teamwork
patterns: single owner, parallel exploration, producer–critic, specialist
pipeline, and recovery huddle."* We have implemented none of the five. This ticket
implements one, properly, on the sprint's own critical path.

## Independent convergence, which is why this is worth doing now

Anthropic runs 20–30 standing routines daily across its own codebases — a crash
fuzzer that opens the app in a simulator and taps around, a dup unifier, a
dead-code remover, an abstraction police. Boris Cherny reports 388 PRs opened over
a few weeks, **180 merged after review**.

Four things in that setup are the same design as `orgintel.md`, arrived at
independently:

| Their pattern | Our spec |
|---|---|
| Each routine is one narrow job, not a generalist | §6.3 specialist roles |
| *"writes a state file at the end… next execution reads that state file at the start"* | §2.1 durable actors, replaceable sessions — our `current-plan.md` + `journal/NNNN.md` |
| Scheduled and event-triggered | §10.1 item 10, durable scheduled and event-driven wakeups |
| *"we ask Claude to tune its routines so it's better the next day"* | §3.4 self-evolution |
| Human review gates the merge | Owner cockpit attention |

**We already have three of those five** — file-based continuity, the scheduler,
and effect receipts. What we lack is exactly the thing this ticket adds: a role
that persists, is narrow on purpose, and gets tuned from its own results.

The lesson worth taking is the one the crash fuzzer embodies: it is good *because*
it does not know about the network code. Narrow context is the feature.

## Scope — one pattern, on the critical path

**Producer–critic on Aris's outreach copy.** §6.3 names its best fit as *"hidden
errors, subjective quality, external-facing output"*, which is cold outreach copy
exactly. It is chosen over the more glamorous options because it is load-bearing
for this sprint rather than a demo beside it: T5 asks the owner to approve a real
send to a real person, and a critic pass before that approval is what makes the
owner's yes cheap.

1. **Roles are files.** `/company/org/roles/<role>.md`, following the template
   below. The Exec writes them; they are readable, revisable, and enter context.
2. **A role can name its own model.** `SpawnRequest` grows an optional `model`;
   absent, it inherits the company's. `agent_auth` already maps
   provider-qualified strings to credentials, so this is small. The *policy* — who
   may pick an expensive model — is the owner's ceiling, unchanged.
3. **A staff actor carries its role, not `"staff"`.** `add_actor(&actor, &role, …)`.
4. **One pattern, executed:** the drafter produces outreach copy; the critic — a
   different role, narrow brief, no access to the drafting context — returns
   specific objections; the drafter revises; only then does T5 raise the approval.
5. **The tuning loop is a record, not a vibe.** After each cycle, an improvement
   record per §3.4. This is what makes the role better next week rather than the
   same forever.

**Not in scope:** the other four patterns, persistent long-lived processes (roles
persist as *files and actors*; sessions stay disposable per §2.1), and any
scheduler change. Standing scheduled roles are the obvious next step and are
deliberately left until one pattern has earned it.

## Templates

Three files, all readable markdown, all entering context. The first is new; the
other two are the spec's own, copied verbatim so we are testing the spec rather
than our paraphrase of it.

**`/company/org/roles/<role>.md`** — new, modelled on §6.3's list of what a
pattern must state:

```markdown
# Role: <name>

## What this role is for
## What it must not do
## Model and why
## Context it gets (and what it is deliberately denied)
## Outputs and where they go
## How we know it is working
## Exit conditions
```

**`/company/org/hypotheses/<slug>.md`** — orgintel Appendix A, verbatim:

```markdown
# Hypothesis: <title>

## Question
## Observations
## Hypothesis and prediction
## Important assumptions and unknowns
## Cheapest informative test
## Owner and budget/time box
## Stop and expansion criteria
## Evidence and artifacts
## Result and decision
## What changed in our beliefs?
```

**`/company/org/improvements/<slug>.md`** — §3.4's evolution record, verbatim:

```markdown
Observed problem:
Proposed change:
Why it may help:
Predicted observable effect:
Scope and budget:
Baseline or comparison:
Result:
Adopt, revise, or revert:
```

T0 already establishes `hypotheses/` and `improvements/`; this ticket adds
`roles/` and makes all three load-bearing rather than optional.

## Acceptance

1. Two roles exist as files with different models named, and `restless people` (or
   the actor rows) shows two actors whose roles are **not** `"staff"`.
2. A critic returns at least one specific objection that changes the copy, and the
   diff is observable. A critic that only says "looks good" is a failed
   demonstration, and that is recorded as the finding.
3. The approved copy reaching T5 is the revised version, verified by comparing the
   draft and the approval payload.
4. An improvement record exists naming what the cycle got wrong and what changed,
   with a prediction concrete enough to be wrong next time.
5. Cost is reported per role, so "was the critic worth it" is answerable in dollars
   rather than opinion.

## What this makes deletable

`OrgMode::MinimalTeam` and `OrgMode::OrgIntel` as separate variants — they are the
same configuration and always were. If a real pattern is the unit of comparison,
the mode enum collapses to "does this company use patterns or not", or disappears
entirely. `infra/compare-modes.sh` goes with it: comparing two identical arms and
a third that differs by a refusal is not an experiment, and keeping the harness
invites re-running it.
