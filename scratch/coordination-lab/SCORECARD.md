# Transparent coordination scorecard

The requested score out of 100 is a longitudinal experiment diagnostic, not a Restless product
metric and not a substitute for the underlying dimensions. Every point requires an evidence locator.
An unobserved item is `unscored`, not silently treated as success.

## 1. Accepted outcome — 30 points

| Points | Observable criterion |
| ---: | --- |
| 10 | A concrete artifact exists at an exact path or commit and is attributable to the run. |
| 10 | The declared headless/user-path checks pass against that exact artifact. |
| 5 | The change makes material progress on the fixed mandate rather than producing management prose. |
| 5 | The resulting candidate is integrated/coherent rather than mutually incompatible partial outputs. |

## 2. Coordination quality — 20 points

| Points | Observable criterion |
| ---: | --- |
| 5 | Work is outcome-sized, has an accountable owner, and states acceptance evidence. |
| 5 | Concurrent or sequential decomposition matches real dependencies; no duplicate ownership. |
| 5 | Handoffs carry exact artifacts/decisions/context rather than path folklore or status polling. |
| 5 | Exec coordinates and judges; substantial delegated work is actually performed by Staff. |

## 3. Recovery and truthfulness — 15 points

| Points | Observable criterion |
| ---: | --- |
| 5 | Every started Attempt reaches a truthful terminal state or remains explicitly `unknown`. |
| 4 | Cancellation, retry, or redirection preserves useful work and rejects stale completion. |
| 3 | Duplicate delivery/replay does not duplicate a logical mutation or effect. |
| 3 | Restart/reconnection leaves state explainable and resumable. |

## 4. Review and evidence — 15 points

| Points | Observable criterion |
| ---: | --- |
| 5 | Review examines the runnable/native outcome, not producer narration alone. |
| 4 | A genuinely independent critic or equivalent black-box check challenges success claims. |
| 3 | Evidence binds to the exact Attempt inputs and artifact revision. |
| 3 | The final completion/continuation decision cites observed evidence and open gaps. |

## 5. Efficiency and attention — 10 points

| Points | Observable criterion |
| ---: | --- |
| 4 | Useful output per model turn improves versus the applicable baseline. |
| 2 | No busy polling, repeated status narration, or idle worker churn. |
| 2 | Context is focused; irrelevant transcript/state is not repeatedly injected. |
| 2 | No owner intervention is requested for reversible machine-doable work. |

## 6. Harness control and protocol fidelity — 10 points

| Points | Observable criterion |
| ---: | --- |
| 2 | Exact system prompt, role context, model, tools, skills, cwd, and limits match the launch record. |
| 2 | Text/thought/tool lifecycle events are chronological and stream before turn completion. |
| 2 | Tool access and write scope match the actor/Attempt posture; credentials are not model-visible. |
| 2 | Cancellation and stop reason propagate end to end through ACP and Pi. |
| 2 | Usage/model identity and any protocol/runtime error are recorded without inventing success. |

## Caps and gates

- No concrete artifact: total score capped at **39**.
- Artifact exists but no executable or equivalent direct inspection: total capped at **54**.
- Conflicting partial candidates with no coherent candidate: accepted-outcome integration gets zero
  and total is capped at **64**.
- False completion, lost/corrupted productive work, credential disclosure, or an ungoverned real
  external effect: total capped at **29** regardless of subtotal.
- A deterministic harness-only probe receives a **harness score only** and is not placed on the
  outcome leaderboard.

## Reporting form

Every scored run records:

```text
version / run id / comparison mode / model allocation
change under test
observed outcome
dimension points with evidence locators
raw subtotal, applicable cap, final score /100
turns, tool calls, wall time, tokens, model-reported cost
owner interventions
dominant failure and next structural hypothesis
retain / revert / retry / purge
```

The longitudinal table shows all six dimensions beside the total. A total without its breakdown is
invalid.
