# EXP-16 evidence, metrics and review specification

**Authority:** subordinate to [`EXP16_PROTOCOL.md`](EXP16_PROTOCOL.md) and the
[experiment sprint](../../../exp-sprints/exp-sprint-16-embodied-npc-playtesting.md)

This specification makes a counted run reconstructable without retaining frame dumps, process state or
model homes. Unknown values stay `null`; they are never converted to zero or inferred from silence.

## 1. Frozen activation manifest

`FROZEN_MANIFEST.json` records:

- protocol and specification digests;
- exact EXP-15 commit and tree;
- runtime image, Godot/toolchain and gate-set fingerprints;
- baseline, visible and held-out scenario commitments;
- policy/profile versions and seed commitments;
- producing and reviewing model routes, versions, efforts and modalities;
- aggregate and stage budgets;
- capture retention rule;
- evaluator rubric digest; and
- activation time and authority.

Any material change creates an append-only amendment, identifies affected runs and invalidates them when
comparability changed. The old manifest is not overwritten.

## 2. Run ledger

`RUNS.jsonl` contains one terminal row per run. Required fields are:

```text
run_id, stage, scenario_id, scenario_class, attempt_id
candidate_commit, candidate_tree, build_fingerprint, gate_set_digest
scenario_seed, world_seed, policy_seed, profile_id, policy_version
controlling_peer, authoritative_peer, started_at, ended_at
sim_ticks, elapsed_ms, active_control_decisions
model_calls, vision_frames, input_tokens, output_tokens, spend_usd
goal_transitions, action_attempts, action_failures, stalls, oscillations, recoveries
objective_result, authority_violations, manual_interventions
failure_class, validity, disposition, receipt_digest, evidence_refs
processes_reaped, leases_released, raw_artifacts_retained
```

`validity` is `valid`, `infrastructure-invalid`, `protocol-invalid` or `evaluation-invalid`.
`disposition` is `passed`, `product-failed`, `npc-failed`, `stopped`, `invalid` or `unknown`.

No timeout is represented as a pass. An absent provider response, missing callback or unknown usage is
recorded explicitly.

## 3. Action and failure evidence

`ACTIONS.jsonl` may be retained for decisive runs only and contains the bounded receipt defined in
[`NPC_ARCHITECTURE_SPEC.md`](NPC_ARCHITECTURE_SPEC.md). Long successful action sequences may be
compacted into per-goal summaries after their digest and count are preserved.

`FAILURES.jsonl` records:

- exact run/candidate/seed coordinates;
- primary class: `game`, `npc-policy`, `substrate`, `provider`, `evaluation` or `authority`;
- last progress and stall/oscillation signature;
- attempted recoveries and why they terminated;
- compact visible evidence references;
- whether a human/player would plausibly encounter the same state; and
- repair Work/Attempt and regression result when accepted.

One failure receives one primary class before the producer sees evaluator commentary.

## 4. Baseline comparison

`BASELINE.jsonl` and `METRICS.json` report baseline and evaluator separately for canonical and recovery
strata. For every metric, publish count, median, range and missing count. Do not pool delivery, recovery,
robber and vampire into one success rate.

The low-level control ratio is:

```text
baseline model motor decisions / evaluator model motor decisions
```

An evaluator that uses zero model motor decisions reports the baseline count and `>=baseline_count×`
rather than infinity. Sparse semantic review calls are reported separately. The “10x” statement is
permitted only if the same scenario set shows at least a 90% decision reduction and no acceptance gate
regresses because equivalent work was hidden in a different uncounted model call.

Also report:

- elapsed-time ratio;
- model-spend ratio;
- intervention-rate difference;
- objective-completion difference;
- bounded-failure-packet usefulness; and
- bot-versus-blind-review agreement.

## 5. Blind review contract

The source-blind reviewer receives:

- the player-facing scenario objective;
- exact content-addressed rendered evidence or prepared native target;
- controls/feedback a player would know;
- the locked role-specific rubric; and
- known platform limitations that both baseline and candidate share.

It does not receive source, transcripts, producer notes, expected result, arm labels, prior scores or
hidden seed intent. Scores freeze before process or repair history is revealed.

### Shared 100-point rubric

| Criterion | Weight | Question |
| --- | ---: | --- |
| visible coherence | 25 | Can a player understand what is happening and why? |
| mechanical credibility | 20 | Does motion, collision, interaction and authority look physically consistent? |
| objective legibility | 15 | Are goals, state changes and results perceptible without source knowledge? |
| recovery quality | 15 | Does the actor respond coherently to trouble rather than oscillate or cheat? |
| role/fairness | 15 | Does the driver/robber/vampire create its intended pressure with a fair response path? |
| polish and distraction | 10 | Are severe visual, timing, camera, UI or feedback defects absent? |

Delivery-only reviews mark role/fairness as journey behaviour and redistribute no weight. A score is not
a pass if a non-negotiable mechanics, authority or completion gate fails.

Reviewers additionally answer:

1. Did the journey visibly complete?
2. Did any behaviour look like teleporting, hidden knowledge or direct state mutation?
3. What single defect most changes the player outcome?
4. Would this evidence justify a human playtest, a repair, or a stop?

## 6. Product finding and repair ledger

`REPAIRS.jsonl` records one row per candidate finding:

```text
finding_id, discovery_run_id, primary_class, player_consequence
lead_decision, work_id, attempt_id, source_before, source_after
discovery_replay, regression_run_ids, held_out_results
review_score_before, review_score_after, spend_usd, disposition
```

Accepted repair requires discovery replay plus held-out regression. “Could not reproduce” remains a
result; it is not silently deleted. Infrastructure repairs are recorded separately and do not consume
the five product loops unless they change the NPC or game artifact.

## 7. Required terminal artifacts

The compact terminal set is:

- `FROZEN_MANIFEST.json` and amendments;
- `BASELINE.jsonl`;
- `RUNS.jsonl` and optional compact `ACTIONS.jsonl`;
- `NPC_CONTRACT.md` describing implemented deviations from the architecture spec;
- `ANTI_CHEAT_RESULTS.md`;
- `FAILURES.jsonl`, `REPAIRS.jsonl` and `METRICS.json`;
- `BLIND_REVIEWS.jsonl` plus frozen rubric and identity-seal digest;
- `FINDINGS.md`; and
- `RESULTS.md` with G1–G5 verdicts and Dogfood 4 keep/revise/purge decisions.

`RESULTS.md` must state:

- what exact candidate was tested;
- what completed and what did not;
- which metric, if any, crossed 10x;
- which findings transferred to real product repairs;
- where bot success disagreed with blind or human judgement;
- model spend and invalid-run count;
- retained residual risks; and
- proof that transient artifacts and processes were cleaned.

## 8. Interpretation rules

- NPC completion proves mechanics under tested conditions, not fun.
- A blind-review score proves one review contract, not broad player preference.
- Fixed-seed repeatability does not prove held-out robustness.
- Product-role reuse requires shared code/contracts and scenario evidence, not similar naming.
- A cheaper loop that finds no real defects may still be useful for regression, but cannot claim better
  product judgement.
- Negative and invalid results remain in denominators appropriate to their class; no selective rerun is
  substituted for them.
