# Coordination learning system

This folder is the durable, systematic knowledge base for Restless's agent-team research. It answers
one question:

> Which organisational shape produces the best accepted outcome for this kind of work, and why?

It is not a second OrgIntel database, an academic benchmark platform or a place to preserve every
idea forever. Raw traces, artifacts and immutable run reports remain in the evaluation harness or
Company Runtime. This folder retains only the experiment contract, evidence index and current
synthesis needed to make the next organisational decision.

## Canonical files

| File | Purpose |
|---|---|
| [`CANON.md`](CANON.md) | The current compact set of supported beliefs, counterevidence and expiry conditions |
| [`PROGRAM.md`](PROGRAM.md) | Full staged plan for testing the sixteen communication wildcards |
| [`BASELINES.md`](BASELINES.md) | Sparse ordinary-team crossover design that must run before wildcards |
| [`REGISTRY.md`](REGISTRY.md) | One status row per experiment; the only experiment-status source of truth |
| [`EVIDENCE.md`](EVIDENCE.md) | Index from claims to raw runs and research priors |
| [`FAILURES.md`](FAILURES.md) | Stable failure vocabulary for comparing runs without inventing a new label each time |
| [`templates/experiment.md`](templates/experiment.md) | Contract completed before a new experiment runs |
| [`templates/run-report.md`](templates/run-report.md) | Comparable evidence report completed after a run |
| [`templates/learning.md`](templates/learning.md) | Claim format used when evidence changes the canon |
| [`templates/workload.md`](templates/workload.md) | Pre-run feature card separating work type, size and parallelisability |

Do not create one directory per speculative idea. Create `experiments/<id>/` only when its first
executable probe begins. A live experiment directory may hold its frozen contract, scenario locator,
run indexes and small analysis artifacts. Large/generated traces remain under `scratch/` or the
Company Runtime and are linked rather than copied.

## Evidence flow

```text
frozen experiment contract
→ raw run + native artifact
→ immutable run report
→ finding with scope and counterevidence
→ repeated or strongly discriminating learning
→ CANON.md
→ architecture / OrgIntel default / ADR only when warranted
```

The reverse flow is equally important. Contradictory evidence weakens or removes a canonical belief.
Git preserves the old wording; the active canon does not retain two competing defaults.

## Claim states

| State | Meaning |
|---|---|
| **Hypothesis** | Plausible and untested locally |
| **Provisional** | Supported by at least one informative Restless run |
| **Accepted** | Replicated across relevant conditions or established by a clear architectural decision |
| **Rejected** | A fair test failed to support it |
| **Superseded** | Replaced by a better-scoped explanation |
| **Blocked** | The mechanism could not be exercised; this is not negative evidence about its value |

Every claim records scope, evidence, counterevidence, confidence and the observation that would change
it. “Accepted” never means universal.

## Run discipline

1. Freeze the success contract, starting artifact, model/tool envelope and acceptance target.
2. Change one important mechanism at a time.
3. Preserve the Exec → accountable lead boundary in every arm. Team structure varies below the lead.
4. Compare against both a strong lead working alone and the ordinary brief/handoff team where relevant.
5. Judge native outcomes before reading producer reasoning.
6. Separate harness failure, model/provider failure and organisational failure.
7. Record negative and unknown results without repair-by-narrative.
8. Update `REGISTRY.md` after every counted run and `CANON.md` only when the evidence changes a belief.
9. Purge mechanisms that lose their decision gate; do not retain every wildcard as a production mode.

## Relationship to existing evidence

The completed v01–v23 scratch programme remains the historical evidence base at
[`scratch/coordination-lab`](../../../scratch/coordination-lab). Its reports are not rewritten. This
folder corrects or promotes their interpretation through linked claims. The evaluation rules in
[`docs/specs/evaluation-dogfood.md`](../../specs/evaluation-dogfood.md) remain authoritative.
