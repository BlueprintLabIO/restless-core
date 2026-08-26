# EXP-08 — entry preflight

**Status:** blocked; no experiment arm, candidate playbook, frozen contract, or workload has started.

**Checked:** 26 August 2026

**Experiment contract:** [`exp-sprint-08-evidence-backed-capability-playbooks.md`](../../../exp-sprints/exp-sprint-08-evidence-backed-capability-playbooks.md)

## Observed entry state

| Required gate | Required evidence | Observed repository state | Result |
| --- | --- | --- | --- |
| EXP-07 successful Restless greenfield web outcome | Owner blind judgement followed by unsealed arm evidence identifying the Restless result | [`EXP-07 blind review checkpoint`](../EXP-07/results/README.md) says `Awaiting owner judgement`; its arm identity and process evidence remain sealed | Blocked |
| Sprint 20 terminal research-publication evidence | A terminal Sprint 20 report and its evidence package | [`Sprint 20`](../../../../docs/sprints/sprint-20.md) remains a draft whose ticket checklist is entirely unchecked; `docs/sprints/sprint-20/` does not exist | Blocked — hard entry condition |
| Evidence-backed frozen treatment | A candidate that cites only completed, scoped evidence | Sprint 20 has not produced the required evidence, so no candidate playbook or evidence certificate has been authored | Not attempted |
| EXP-07 callback, ReviewTarget and verification-output preflight | A passing product-path probe or an explicit `product-invalid` classification | No EXP-08 arm is eligible to run; this gate remains pending and cannot be attributed to the treatment | Pending |

## Decision

EXP-08 does not start yet. In particular, this preflight deliberately creates no web-production
playbook, arm contract, randomisation, workload repository, registry, database, team router, or
production-site change. Writing a treatment before its required Sprint 20 evidence exists would turn
an intended test of evidence-backed reuse into a speculative prior.

The risk of losing momentum while waiting is **accepted**: the missing inputs are explicit and the
experiment can restart from this record. The risk of smuggling an unearned playbook into a later arm
is **guarded** by leaving no candidate artifact to retrieve.

## Earliest valid restart

1. Record the owner’s EXP-07 blind decision and unseal the relevant Restless outcome evidence.
2. Complete Sprint 20 to a truthful terminal classification with its required evidence package.
3. Recheck the two source locators above, then freeze the smallest candidate playbook and its dated,
   scoped evidence certificate before either arm sees it.
4. Freeze the matched workload-1 source pack, objective checks, blinded native-review packet, model
   envelope and arm order; then run only workload 1 first.
5. Use its result to decide whether workloads 2 and 3 are worth running. Do not infer the transfer
   result from a successful first site.

This is a preflight record, not an EXP-08 result or a claim that a web-production playbook helps.
