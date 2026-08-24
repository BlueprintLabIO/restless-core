# EXP-03 four-primitives live result

**Run:** `exp03-t2-s2-primitives-glm53-r1`
**Comparison:** `exp03-t2-s2-glm53-r1`
**Model:** `zai/glm-5.3` for every cognitive actor
**Frozen task/evaluator:** byte-identical T2 inputs
**Result:** live acceptance passed

## Outcome first

The patched supervised team reached a truthful terminal result in the intended five-turn sequence:

1. supervisor commissions strategist;
2. strategist produces exact strategy commit `2469b6e6794f7cb82dbd0f571dce89211ba03618`;
3. supervisor accepts it and commissions the dependency-linked producer;
4. producer returns exact final commit `a17f320e7073278037a73bfb06063af370db5676`;
5. supervisor promotes the commit unaltered, verifies an archive-native export, and calls
   `complete_run`.

Both Work responsibilities completed in their first Attempt. The Work dependency closure covers both
workers, the supervisor authored no production content, all four repository-native checks pass, and
the runner records `decision_complete=true`.

## Before / after

| Measure | Pre-patch | Four primitives | Change |
|---|---:|---:|---:|
| elapsed | 4,650.48s | 2,321.74s | -50.1% |
| recorded cost | US$4.3736 | US$2.2993 | -47.4% |
| model turns | 14 | 5 | -64.3% |
| supervisor turns | 9 | 3 | -66.7% |
| Attempts | 5 | 2 | -60.0% |
| used tokens | 682,015 | 280,932 | -58.8% |
| output tokens | 287,229 | 148,456 | -48.3% |
| frozen evaluator checks | 14/19 | 15/19 | +1 |
| native candidate checks | 4/4 | 4/4 | equal |
| harness completion | false | true | repaired |

The final producer turn was slower and more expensive than the pre-patch producer's first attempt
(1,121.52s / US$1.3239 versus 863.0s / US$1.0091). The overall gain therefore did not come from an
easier model sample. It came from deleting recovery turns caused by the harness. A single replication
does not establish the exact percentage as a stable effect size.

## Primitive evidence

- **Attempt identity — live pass:** both worker commits imported under unique Attempt refs; no mutable
  Work ref or manual ref deletion was used.
- **Explicit completion — live pass:** event `run_completed` names exact candidate `a17f320e`; terminal
  state records `decision_complete=true`.
- **Archive-native review — live pass:** the lead reviewed the exact no-`.git` export; the final worker
  verifier reports 106 explicit checks passed.
- **Judgement resumes Work — deterministic pass, not invoked live:** the successful sample needed no
  judgement. `exp03-four-primitives-r1` proves the exact old failure path: worker request blocks the
  Attempt, lead resolution reactivates the same Work at revision +1, one wake is queued, and a fresh
  Attempt receives the preserved workspace.

The pre-patch run supplies complementary real-model evidence for the judgement seam: its worker and lead
used judgement correctly, and only the absent resume transition caused the dead end.

## Evaluation caveat

The frozen evaluator remains deliberately unchanged. Its four failures do not describe four observed
outcome defects:

- check 9 requires the literal stem `sequence`, missing ordinary `sequencing` language;
- check 12 cannot parse timecodes inside the artifact's Markdown table despite the worker verifier
  proving eight beats sum to exactly 45 seconds;
- checks 15–16 require five event names invented by the evaluator and absent from the frozen owner
  brief, rejecting the coherent eleven-event contract chosen by the team.

This confirms the scorer should be split later: deterministic checks for specified mechanical facts,
and blinded semantic judgement for open-ended quality. It is not a reason to add production guards.

## Retention decision

Keep all four primitives in the experiment harness. They reuse existing Work, Attempt, Git, decision,
transaction and outbox concepts and remove observed failure without adding a scheduler, ledger, custody
protocol or workflow interpreter. Do not yet port them wholesale into production OrgIntel; first express
the same four semantics in the implementation sprint's existing ownership boundaries.
