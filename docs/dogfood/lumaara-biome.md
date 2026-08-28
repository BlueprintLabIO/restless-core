# Scenario: Lumaara — second biome and the trainer beat

**Version:** 1
**Company shape:** building (Cosmon)
**Modes:** `single_agent` · `minimal_team` · `orgintel`

A versioned scenario package (`docs/specs/evaluation-dogfood.md` §21.1). The same
mission, budget, model and runtime are given to three companies that differ in
exactly one respect: how they are allowed to organise.

---

## Success contract

> **Add a second biome (Prism Caverns, with traversal into it) and a trainer beat
> (at least one trainer battle plus a mini-boss that gates the caverns) to the
> existing Lumaara slice, leaving the game playable end to end.**

Success is judged on the artifact, not on activity (§2.1). A run that files
commitments, writes plans and dispatches workers but leaves the game no better
than it found it has failed the contract.

### Acceptance — manual, by the owner (§21.2)

Load the built game and check:

1. The caverns are reachable from the basin, and entry is gated on the mini-boss.
2. At least one trainer battle can be started and completed.
3. The pre-existing loop (explore → encounter → bond → battle) still works.
4. Nothing in the build is broken by the additions.

Do not substitute a self-reported score for this (§25 rule 2).

## Starting state

Every mode starts from the same commit of `cosmon-game` — the roster/evolution
build (`a01bd22`) plus the loop-4 groundwork commit (`514b7b3`), which already
contains the optional-chained hook call-sites and the shared-state contract.
That groundwork is deliberately included in all three: it is prior work, and
withholding it from the baselines would rig the comparison (§25 rule 3).

## Held identical across modes

| | |
|---|---|
| Model | `zai/glm-5.2` |
| Budget ceiling | $15 |
| Runtime image | `restless-company-image:latest` |
| Starting repo | same commit |
| Wake reason | identical text |
| Effect capabilities | same set |

## The single difference

| Mode | Delegation | What a worker is told |
|---|---|---|
| `single_agent` | refused | — |
| `minimal_team` | allowed | its task only |
| `orgintel` | allowed | task + mission, open commitments, how to reach the Exec |

## Recorded per run

Cost (USD), elapsed wall-clock, turns, tool calls, staff dispatched, owner
interventions, commits produced, and the owner's acceptance verdict with a
one-line rationale.

## Predictions, recorded before the run (§2.4)

Written down so being wrong is informative rather than reinterpretable:

- `single_agent` completes the contract, because it already has — every sprint-01
  artifact came from this configuration.
- `minimal_team` is *worse* than `single_agent`: workers with no company context
  duplicate or collide, and integration costs the Exec more than it saves.
- `orgintel` beats both on elapsed time but not on cost, because parallel workers
  buy wall-clock and spend tokens.

If `orgintel` does not beat `single_agent` on any dimension, the honest reading
is that organisational intelligence does not earn its overhead at this size of
work — and that is the finding, not a failure of the run (§25 rule 10).
