# EXP-17 evaluation and decision specification

**Authority:** subordinate to [`EXP17_PROTOCOL.md`](EXP17_PROTOCOL.md) and the
[experiment sprint](../../../exp-sprints/exp-sprint-17-worker-architecture-benchmark.md)

This document separates outcome judgement from process economics. Reviewers score the locked artifact
before they learn whether it came from solo Codex or Restless supervision.

## 1. Evidence custody and blinding

For each run, the controller creates:

- an opaque artifact label unrelated to arm or run order;
- exact task/start/model/tool/budget lineage;
- a content-addressed terminal artifact or prepared native review target;
- native gate receipts;
- a sealed mapping from artifact label to arm; and
- a process ledger hidden from outcome reviewers.

Two independent reviewers receive only the owner brief, allowed references, artifact, native outcome
evidence and locked rubric. They do not receive transcripts, cost, model turns, Staff/lead messages,
prior scores, sibling artifacts or arm identity.

Reviewers freeze signed/hashed scores before the seal opens. If criterion scores differ by more than 15
points or reviewers disagree on a serious blocker, one named adjudicator sees only that criterion and
the same blind evidence. There is no generic committee summary.

## 2. Common outcome rubric

Every task-specific rubric totals 100 and preserves these common dimensions:

| Dimension | Default weight | Meaning |
| --- | ---: | --- |
| outcome correctness | 30 | The requested result is factually/behaviourally correct |
| completeness and consumer fitness | 25 | The named consumer can use it without material missing work |
| robustness and serious-defect avoidance | 20 | Hidden/adverse cases do not reveal false completion |
| evidence and provenance | 15 | Claims/results are traceable and native gates are honest |
| clarity and maintainability | 10 | The artifact is understandable and proportionate |

Task freeze may move at most 10 points between dimensions and must explain why before arms run. Native
non-negotiable gates and serious blockers override the numeric average.

### Coding additions

Reviewers consider regression scope, architecture proportionality, operational behaviour and the
prepared native outcome. Code volume, framework novelty and number of tests are not quality proxies.

### Non-coding additions

Unit-level source support, policy compliance, actionability, isolation and tail quality are scored.
Word count, prose polish and portfolio-level synthesis do not compensate for rejected units.

### Longitudinal additions

The final score explicitly checks that the material change displaced stale work, the duplicate caused no
semantic duplication, useful work survived process replacement, and the scheduled follow-up was handled
without owner re-decomposition.

## 3. Native gates and hidden cases

Native checks run against the immutable candidate before blind review. Results are attached without arm
identity. A hidden test is used only when it represents a frozen owner requirement, not an arbitrary
implementation preference.

A serious blocker includes:

- the core requested outcome does not run or cannot be consumed;
- a security, authority, policy or data-isolation violation;
- false completion despite a failed non-negotiable gate;
- fabricated source/evidence;
- stale requirements survive the material change;
- important work is lost after the process event; or
- independent-unit quality breaches the frozen tail floor.

One blocker prevents a quality win even when the average score is high.

## 4. Process ledger

`RUN_LEDGER.jsonl` stores one row per run:

```text
run_id, pair_id, cell, task_id, opaque_artifact_label
arm, arm_order, validity, failure_class, replay_of
starting_capsule_digest, candidate_digest, parity_manifest_digest
model_requested, model_observed, reasoning_effort, tool_manifest_digest
started_at, terminal_at, elapsed_ms, active_ms
input_tokens, cached_input_tokens, output_tokens, reasoning_tokens, spend_usd
model_turns, tool_calls, failed_tool_calls, repeated_actions
exec_turns, lead_turns, supervisor_wakes, supervisor_interventions
worker_sessions, process_replacements, orientation_reads
feedback_events, duplicate_deliveries, explicit_interrupts
gate_executions, gate_cache_hits, duplicated_verification
manual_rescues, owner_attention_requests, artifact_lineage_errors
terminal_status, artifact_digest, native_gate_status
raw_review_score, serious_blockers, transfer_score
```

Unavailable provider telemetry is `null`. Supervisor activity is zero only when the ledger positively
observed none. A safety termination records the responsible condition and cannot imply task failure
without attribution.

## 5. Pair validity

`PAIR_VALIDITY.jsonl` is frozen before scores are compared. A pair is valid only when:

- task/start bytes and hidden fixtures match;
- the same Codex runner, exact producing model, effort and task tools were observed;
- aggregate budget and safety envelope match;
- arm order followed the frozen schedule;
- event delivery occurred at the frozen equivalent milestone;
- artifacts and evaluator evidence did not leak across arms; and
- neither arm received controller/human semantic rescue.

A substrate/provider/evaluation failure invalidates both members and permits one symmetric replay after
repair. A product, model or coordination failure is counted and cannot be replayed merely because it
hurts an arm.

## 6. Primary and secondary analysis

For every matched task report:

1. each blind score and serious blocker;
2. paired score difference and practical-parity result;
3. rescue-free completion;
4. total and producer-only spend;
5. elapsed and active time;
6. recovery time and useful work retained when applicable;
7. supervisor cost and which decisions materially changed;
8. duplicated work/gates and artifact-lineage errors; and
9. transfer result when applicable.

Use medians/ranges only within a cell with compatible tasks. Do not pool coding and non-coding scores,
small and longitudinal work, or locally closing units and coupled artifacts.

The experiment is a sparse product decision, not a powered statistical study. Report raw paired data,
order effects, task-author uncertainty and confounds. Do not use significance language.

## 7. Crossover decisions

Apply the main sprint's predeclared thresholds:

- quality differences under five points are practical parity unless a serious blocker exists;
- `C` remains default when `R1` is at parity but adds at least 20% latency or spend without material
  continuity, recovery, quality or transfer benefit;
- `R1` earns the cell when it is within parity, adds no blocker and materially improves rescue-free
  completion, recovery/retention, regression or transfer;
- `RP` earns a separable cell only within quality parity and with at least 30% elapsed-time or accepted-
  throughput improvement including lead/review/fan-in work; and
- otherwise the cell is `unknown` or preserves the safer incumbent routing.

Supervisor governance may remain a product invariant for authority-sensitive live work even where it
loses this efficiency benchmark. The result must distinguish governance necessity from measured
performance.

## 8. Failure attribution

Assign exactly one primary class before arm reveal:

- `product/outcome-failure`;
- `model/actor-failure`;
- `coordination-failure`;
- `runtime/harness-failure`;
- `provider/capacity-failure`;
- `evaluation-infrastructure-invalid`; or
- `authority/safety-stop`.

Secondary contributing facts may be recorded, but cannot be used to reclassify a counted loss after
identities are known.

## 9. Required terminal artifacts

- `PARITY_MANIFEST.json` and preflight report;
- frozen task/fixture/rubric/order/budget manifests and hashes;
- `RUN_LEDGER.jsonl` and `PAIR_VALIDITY.jsonl`;
- blind scores, adjudications and sealed arm mapping;
- per-cell paired result tables;
- recovery/event and transfer reports;
- `CROSSOVER_GUIDE.md` mapping observed work shapes to `C`, `R1`, `RP` or `unknown`;
- `KEEP_CHANGE_PURGE.md` for Restless defaults and harness paths; and
- `RESULTS.md`, a short founder-facing narrative with limits and next decision.

Raw model homes, provider secrets, hidden reasoning, temporary worktrees, native captures and evaluator
scratch state are deleted after compact evidence is synthesised.
