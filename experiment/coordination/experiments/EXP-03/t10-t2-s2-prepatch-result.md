# EXP-03 T2 S2 pre-patch result

**Run:** `exp03-t2-s2-glm53-r1`
**Model:** `zai/glm-5.3` for every cognitive actor
**Elapsed:** 4,650.48 seconds (77.5 minutes)
**Recorded cost:** US$4.37356316
**Status:** excellent final artifact; orchestration completion invalid

## Outcome

The strategist and producer created a coherent evidence-bound marketing campaign. The non-producing
supervisor found a genuine defect that the producer's in-repository check missed: the campaign verifier
failed when the candidate was exported without `.git`. After repair, the exact final worker commit
`279b06581e8021033af93a31836c876c09837aa5` passed all four repository-native checks and satisfied the
multi-worker dependency-closure protocol.

The run nevertheless recorded `decision_complete=false`. The lead used a run-scoped free-form decision
subject while the runner expected the literal subject `run`.

## Churn trace

- 14 model turns: 9 supervisor, 4 producer, 1 strategist;
- 5 Attempts across 2 durable Work responsibilities;
- the producer's correct revision-2 sibling commit could not replace the revision-1 artifact because
  `refs/heads/artifacts/<work-id>` required a fast-forward update;
- the worker requested judgement, ended without a terminal report, and became `unknown`;
- resolving the judgement did not resume the worker;
- the supervisor manually deleted the stale ref and commissioned two further revisions before the
  final commit could be reported.

This is productive evidence about the supervisor's review value but invalid evidence about normal team
latency or coordination cost: most late turns repaired the harness rather than the campaign.

## Frozen evaluator

The declared evaluator reported 14/19 checks. Its five failures are retained unchanged for experimental
integrity, but inspection found brittle keyword, timecode, negation and invented-event-name requirements.
They must not be silently reclassified as artifact defects. The exact declared output is preserved at
`workdir/exp03-t2-s2-glm53-r1/declared-evaluator-evidence.json`.
