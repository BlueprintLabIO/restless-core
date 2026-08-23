# v01 — first-party ACP/Pi stream conformance

## Change under test

Replace the opaque OMP turn wrapper with a minimal first-party ACP v1 server composed from Pi Agent
Core. The launch contract fixes actor identity, exact system prompt, OpenRouter model, tool list,
workspace, write posture, limits, and event log.

## Run

- Model: `nvidia/nemotron-3.5-lightning:free`
- Live price check: prompt `0`, completion `0`
- Input: read `README.md` and return its exact marker
- Observed result: `LUMAARA-ACP-STREAM-17`; the model correctly stated that it made no edit
- Turns: 2
- Usage: 500 input, 196 output, zero model-reported cost
- Ordered harness events: 238, contiguous sequence 1-238
- Lifecycle: thought chunks -> tool start -> tool completion -> thought chunks -> answer chunks ->
  terminal `end_turn`
- Deterministic checks: 5/5

Evidence: `pi-harness/fixtures/probe/`, the launch/system hashes in the generated event trace, and the
captured run summary in this report. Generated raw traces remain ignored operating evidence.

## Score

Harness-only score: **80/100**. This is not comparable with the outcome scorecard.

| Harness criterion | Points |
| --- | ---: |
| Exact launch controls | 20/20 |
| Chronological live thought/text/tool streaming | 20/20 |
| Tool/write posture and credential stripping | 20/20 |
| Cancellation/stop propagation | 0/20 — not exercised |
| Usage/model/error truthfulness | 20/20 |

## Decision

Retain. The dominant missing proof is ACP cancel -> Pi abort -> tool-child termination -> truthful ACP
stop. V02 tests that path; no coordination machinery should be added before it is known.
