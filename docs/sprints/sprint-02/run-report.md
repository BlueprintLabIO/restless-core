# Sprint 02 run report

**Status:** in progress — T4's comparison is executing.
**Question the sprint exists to answer:** does an Exec-led organisation beat one
strong agent on the same mission, given the same model, tools, budget and time?

---

## What landed

| Ticket | Commit | How it was verified |
|---|---|---|
| T2 · `restless spawn` as a tool | `a66f689` | T9's staff machinery ran for the first time: refusals returned synchronously, worktree created, staff commit `2e42a90` on `staff/probe-staff` |
| T7 · Party-level repeat detection | `3343b18` | Greg's real scenario — flagged across two honest keys, **not** flagged on replay |
| T8 · Organisational health signals | `206c0b2` | caught its own false positive against real receipt data before shipping |
| T3 · Three org modes + harness | `ddfb545`, `2b77eda` | mode gating verified in both directions |
| Purge · model gateway | `4f20054` | **2,999 deletions**; 102 spend records migrated and verified |

## Findings so far

### Sprint 01 was running `minimal_team`, unlabelled

The three-mode design did not have to invent its independent variable. Staff
have always been handed a task string and nothing else — no mission, no idea
what else is in flight, no way to raise a blocker. That is `minimal_team` by
the evaluation spec's own definition. So sprint 01 was not running a degraded
OrgIntel; it was running the middle baseline without knowing it, and reporting
the result as the product.

`orgintel` mode is the first time a worker is told what company it works for.

### Event-driven wakeups fire — proven by accident

A throwaway one-line-comment probe completed, its commitment completed, and the
scheduler woke the Exec:

```
scheduler wake company="cosmon"
  reason="event: commitment completed by staff-probe-staff: ..."
```

Sprint 01 listed this as an open question ("do event-driven wakeups actually
fire, or does the Exec go silent after a dependent result lands?"). They fire.
The staff→Exec trigger chain works end to end.

### Unknown is not failure — the same bug, twice, in one day

`classify_turn` fell through from its error path to its consumption check,
reporting a 20-minute work-turn boundary as "the model never ran, check
provider credit". Fixed in the morning.

By the afternoon the organisational health signals had grown a second, subtly
different definition of "failed" — an allowlist of success words that
classified `deployed` and `refunded` as failures. It would have accused a
healthy company of repeating a failed approach every time it deployed.

Both are the same error: **treating unknown as negative.** The fix is now
structural — one three-state `Outcome`, shared by reconciliation and health,
pinned by a test over every status word both companies have actually emitted.
A second definition of a shared predicate is the duplicate-ownership problem
the cross-layer contract forbids, appearing inside a single service.

### Unreachable concepts are not dead code

The purge scan found `add_goal`, `add_decision` and `add_artifact_ref`
uncalled. They are storage for three concepts §3.1 assigns to OrgIntel as
authoritative, with **no write path for any actor**. Recorded as a gap rather
than deleted — purging them would silently narrow the ontology.

---

## T4 — the comparison

**Scenario:** [`lumaara-biome`](../../scenarios/lumaara-biome.md) v1 — add Prism
Caverns plus a trainer beat to the Lumaara slice, leaving it playable.

Held identical: model `zai/glm-5.2`, $15 ceiling, image, starting commit
(`514b7b3` groundwork atop `a01bd22`), wake reason, capabilities.

### Predictions, recorded before the run

- `single_agent` completes the contract — every sprint-01 artifact came from it.
- `minimal_team` is **worse** than `single_agent`: context-free workers collide
  and integration costs more than it saves.
- `orgintel` beats both on elapsed time, not on cost.

### Results

| Mode | Termination | Tools | Staff | Cost | Elapsed | Owner acceptance |
|---|---|---|---|---|---|---|
| `single_agent` | PENDING | | | | | |
| `minimal_team` | PENDING | | | | | |
| `orgintel` | PENDING | | | | | |

Acceptance is manual, against the scenario's success contract. Numbers do not
substitute for loading the build (`evaluation-dogfood` §21.2, §25 rule 2).

### Reading

PENDING. If `orgintel` does not beat `single_agent` on any dimension, the honest
reading is that organisational intelligence does not earn its overhead at this
size of work — recorded as the finding, not reinterpreted as success
(§25 rule 10).
