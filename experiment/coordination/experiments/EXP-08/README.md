# EXP-08 — entry preflight

**Status:** awaiting Sprint 20 terminal evidence and usable EXP-07 Restless-source evidence; the
isolated verification-output preflight has passed. No experiment arm, candidate playbook, frozen
contract, or workload has started.

**Checked:** 26 August 2026

**Experiment contract:** [`exp-sprint-08-evidence-backed-capability-playbooks.md`](../../../exp-sprints/exp-sprint-08-evidence-backed-capability-playbooks.md)

## Observed entry state

| Required gate | Required evidence | Observed repository state | Result |
| --- | --- | --- | --- |
| EXP-07 successful Restless greenfield web outcome | A successful greenfield outcome plus usable Restless-specific source evidence | The [experiment index](../../../exp-sprints/README.md) records both candidates as objective-matrix passes and founder-judged excellent, while the [blind checkpoint](../EXP-07/results/README.md) still keeps arm identity and process evidence sealed | Partially evidenced; do not attribute a prior to the Restless arm until its source evidence is usable |
| Sprint 20 terminal research-publication evidence | A terminal Sprint 20 report and its evidence package | [`Sprint 20`](../../../../docs/sprints/sprint-20.md) is in progress. Its [staging report](../../../../docs/sprints/sprint-20/staging-report.md) records `staged_candidate`, explicitly not an accepted publication, with no model-led run, peer review, browser review, owner judgement, or terminal classification | Blocked — hard entry condition |
| Evidence-backed frozen treatment | A candidate that cites only completed, scoped evidence | Sprint 20 has not produced the required evidence, so no candidate playbook or evidence certificate has been authored | Not attempted |
| EXP-07 callback, ReviewTarget and verification-output preflight | A passing product-path probe or an explicit `product-invalid` classification | [P1](review-path-preflight.md) remains `product-invalid`: its silent `grep -q` gate retained no stdout. The separately frozen [P2](verification-output-preflight.md) completed the ordinary path, caught and repaired a false green, then retained the marker in the owner-visible gate output and completed explicit deterministic owner acceptance | Passed for validity gate 6 only; neither preflight is treatment evidence |

## Decision

EXP-08 does not start yet. In particular, this preflight deliberately creates no web-production
playbook, arm contract, randomisation, workload repository, registry, database, team router, or
production-site change. Writing a treatment before its required Sprint 20 evidence exists would turn
an intended test of evidence-backed reuse into a speculative prior.

The risk of losing momentum while waiting is **accepted**: the missing inputs are explicit and the
experiment can restart from this record. The risk of smuggling an unearned playbook into a later arm
is **guarded** by leaving no candidate artifact to retrieve.

## Earliest valid restart

1. Make the exact EXP-07 Restless source evidence usable without misrepresenting the sealed blind
   decision.
2. Complete Sprint 20 to a truthful terminal classification with its required evidence package.
3. Preserve P1's failure and P2's passing fixed-output callback/ReviewTarget preflight rather than
   replacing either run.
4. Recheck the two source locators above, then freeze the smallest candidate playbook and its dated,
   scoped evidence certificate before either arm sees it.
5. Freeze the matched workload-1 source pack, objective checks, blinded native-review packet, model
   envelope and arm order; then run only workload 1 first.
6. Use its result to decide whether workloads 2 and 3 are worth running. Do not infer the transfer
   result from a successful first site.

This is a preflight record, not an EXP-08 result or a claim that a web-production playbook helps.
