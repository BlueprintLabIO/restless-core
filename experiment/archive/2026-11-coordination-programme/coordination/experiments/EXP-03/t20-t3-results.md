# EXP-03 T3 result — replicated sales batch

**Recorded:** 24 August 2026
**Status:** Q1/Q2 matched set closed; Q4/Q8 stopped by marginal gate
**Models:** every cognitive actor `zai/glm-5.3`, high reasoning
**External effects:** none; eight fictional prospect units remained unsent

## Decision

Q2 did not add throughput or a robust quality gain. The two operators worked concurrently, but a
third synthesis worker and extra supervisor turns converted their locally useful units back into one
serial batch artifact. Q4 and Q8 are therefore not activated. This rejects the tested
produce-then-assemble shape; it does not reject a real queue whose units close independently without a
model rewrite.

## Scoped span curve

| Measure | Q1 — one operator | Q2 — two operators + assembler |
| --- | ---: | ---: |
| Exact candidate | `6d2f2284d368b19ddd805f3362ae4a8c29fe2412` | `54411aea5e1efe546a5bbca5978a4a96e1eb846d` |
| Protocol / clean exact artifact | pass | pass |
| Units materially completed | 8/8 | 8/8 |
| Frozen evaluator | 17/18 | 17/18 |
| Cognitive-span latency | 1,159.78 s (19.33 min) | 1,943.35 s (32.39 min) |
| Accepted-unit throughput | **24.83 units/hour** | 14.82 units/hour |
| Reported cost | **$0.87529324** | $1.54977144 |
| Cost per completed unit | **$0.1094** | $0.1937 |
| Supervisor time / cost | 300.00 s / $0.29754460 | 755.23 s / $0.66930888 |
| Used tokens / tools | 134,560 / 48 | 247,243 / 146 |
| Turns / Works / Attempts | 3 / 1 / 1 | 7 / 3 / 3 |

Q2 created 405.89 seconds of actor overlap, yet latency rose 67.6%, cost 77.1%, supervisor time 151.7%
and tools 204.2%. Its final assembler alone consumed 841.79 seconds and $0.54086524. The bottleneck was
not the two independent unit producers; it was reconstructing one final owner artifact after them.

## Quality and evaluator interpretation

Both exact artifacts are substantive, evidence-bounded and explicitly unsent. Their frozen 17/18
scores have different meanings:

- Q1 has a real but small schema defect: `nothing_sent` is nested under `batch` instead of the frozen
  top-level location.
- Q2 has the exact top-level flag. Its only failure is an evaluator false positive that reads “No
  message has been sent” as an affirmative sent outcome.

Two fresh, topology-blind GLM-5.3 reviews disagree on the threshold winner. The original prefers Q1
with medium confidence because it is self-contained and does not leak assembly IDs, commit provenance
or duplicate unit representations. The confirmation prefers Q2 with high confidence because it treats
Q1's exact schema miss as a completion-gate failure. Interestingly, Q1 receives the higher linear mean
score in both reviews; the preference flips on how the schema threshold is weighted. Exact outputs are
[`t18`](t18-t3-blind-review-original-recovered.json) and
[`t17`](t17-t3-blind-review-confirmation.json). There is no robust Q2 quality premium.

## Boundary learned

Replicated work scales only when units **close locally**. A sales account, support case or recruiting
candidate should return an attributable unit result that the lead can accept, sample or escalate. A
deterministic projection may aggregate counts and exceptions; another model should not have to reread
and rewrite every unit to make the batch real. T3 therefore sets current span to one for a shared
batch-artifact architecture and leaves the true local-closure queue as a distinct future test.
