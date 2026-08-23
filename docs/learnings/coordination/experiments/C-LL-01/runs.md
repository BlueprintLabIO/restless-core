# C-LL-01 run index

Status: complete; provisional ordinary-team loss

## Frozen allocation

- Order seed: `C-LL-01:v26:ordinary-crossover:2026-08-23`.
- Order SHA-256: `1358de66e98c038c13f78f57061a048daf520f8159e93e2f54bc0f1aef3fc147`.
- Even first-32-bit parity selected: **B0 → B1**.
- Lead in both arms: `gpt-5.6-sol`, medium reasoning.
- B1 producer: `gameplay-systems` on `stealth/ox-alpha`, runtime-default reasoning.
- B1 topology: one accountable lead, exactly one producer, one Staff slot, ordinary artifact handoff.
- B0 topology: the same accountable lead with zero Staff below it.
- Outer envelope: 7,200 seconds per arm; actor completion is callback/process exit, not elapsed time.
- Drain: 120 seconds. Nominal spend ceiling: USD 6 per arm.

## Preflight evidence

- Scenario SHA-256: `90457d619e0e051f8070f9f3e878bf76a5bbe26ac808f42dd36b70b63446efd2`.
- Evaluator SHA-256: `1063015de6b0f22fbf86270c0879ce6ca373e6f9dba37f64cc752061ef3085e7`.
- Seed negative control: 5/23 pass, 18 intended absent-feature failures, zero browser errors.
- Two independent judges classify the workload as large with a deep integrated critical path, high
  shared mutable state and only one credible early world-presentation seam.
- Baseline-isolation/runtime architecture: 39/39 on protocol 24, including legacy telemetry migration.
- Production Exec concurrency: one live Postgres scenario passes with two department Attempts running
  while Exec owns neither.

## Counted B0 — accountable lead alone

- Run ID: `v26-cll01-b0-sol`.
- Candidate: `de8e14d511dbcd1f8dce94e9ef4091053b604e5e` from the frozen seed.
- Valid topology: one accountable Sol lead, zero Staff and one lead turn.
- First lead start to owner decision: 471.85 seconds; actor wall time was 479.36 seconds.
- Usage: 2,265,823 input tokens, 2,131,200 cached input tokens, 19,717 output tokens,
  5,292 reasoning tokens and 50 tool calls.
- Change: five files, 433 insertions and 11 deletions.
- Frozen external evaluator: 23/23, with zero browser errors.
- Candidate-native evidence: boss 20/20, battle 12/12, combat 7/7 and roster 29/29.
- Worktree and candidate were clean at decision time.

## Counted B1 — accountable lead plus one ordinary producer

- Run ID: `v26-cll01-b1-ox`.
- Candidate: `3e545d90eb2f2845d5e6d4480655203bf834e462` from the same frozen seed.
- Valid topology: one accountable Sol lead and exactly one Ox producer. The lead commissioned one
  Work item after about 15 seconds and changed the gate/world seam while the producer owned the battle
  seam.
- First lead start to owner decision: 2,218.94 seconds.
- Lead usage across three wakes: 4,278,304 input tokens, 4,115,712 cached input tokens, 20,690
  output tokens and 51 tool calls.
- Producer usage across two Attempts: 115,195 input tokens, 40,853 output tokens and 108 tool
  calls. ACP did not expose a trustworthy cached-input count for the producer.
- Counted combined usage: 4,393,499 input tokens, 61,543 output tokens and 159 tool calls.
- Change: eight files, 698 insertions and 23 deletions.
- Frozen external evaluator: 23/23, with zero browser errors.
- Candidate-native evidence: boss 25/25, gate 12/12, battle 12/12, combat 7/7 and roster 28/28.
- Worktree and candidate were clean at decision time.

### Recovery sequence

The first producer Attempt ran for 1,698.8 seconds, used 102,395 input tokens and 98 tool calls, and
committed candidate `18a3bcff56a0104fc83b5e677f9552dfa3dcfadf`. It ended without the required terminal report. The
runtime preserved the artifact, classified the Attempt as `unknown` rather than success, and woke the
lead. The lead inspected the evidence and issued one narrower repair limited to the battle module and
native verifier. That Attempt reused the existing commit, reported normally after 122.6 seconds, and
the lead selectively integrated it. This was artifact-first recovery, not a blind retry.

## Matched interpretation

Both arms met the frozen owner contract at 23/23. B0 was independently preferred on artifact quality
and won every measured efficiency axis:

| Measure | B0 | B1 | B0 reduction |
|---|---:|---:|---:|
| Time to owner decision | 471.85s | 2,218.94s | 78.7% |
| Input tokens | 2,265,823 | 4,393,499 | 48.4% |
| Output tokens | 19,717 | 61,543 | 68.0% |
| Tool calls | 50 | 159 | 68.6% |

B1 produced about 279 seconds of genuine overlap: the lead advanced the world/gate seam while the
producer worked on battle behavior. That overlap did not repay delegation. The producer's first
artifact arrived only after B0 would already have made the owner decision, crossed the proposed seam
into shared game and verification state, and required bounded repair plus final integration.

The blind review mapping, findings and reviewer usage are recorded in [`blind-review.md`](blind-review.md).
It found no frozen-contract failure in either candidate, preferred B0, identified one contained B0
gate-routing risk and two broader B1 compatibility/coherence risks.

## Scope of the conclusion

This is a provisional loss for an ordinary Ox producer under a strong Sol lead on a broad,
high-coupling product slice. It does not prove that equal-strength teams lose or that all nominally
large work should remain solo. The pre-run judges overestimated solo duration by roughly 4–14×: the
lead completed the outcome in one effective session despite 50 tool calls and a 433-line change.
Therefore C-LL-01 is evidence below the strong lead's saturation boundary, not the final large-work
crossover point. A later calibration must deliberately exceed one lead session before closing that
boundary.
