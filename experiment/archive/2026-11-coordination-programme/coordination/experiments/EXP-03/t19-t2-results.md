# EXP-03 T2 result — complementary marketing specialists

**Recorded:** 24 August 2026
**Status:** matched set closed; scoped S2 quality win
**Models:** every cognitive actor `zai/glm-5.3`, high reasoning
**External effects:** none; all campaign assets remained fictional and unpublished

## Decision

Two complementary specialists earned a narrow advance on this work shape. The strategist-to-producer
chain cost more and was not faster, but the fresh topology-blind reviewer preferred its complete pack
because it enforced the frozen truth boundary materially better. This is evidence for a differentiated
sequential specialist seam, not for larger teams generally and not for parallel coauthoring.

## Matched results

| Measure | S1 — one end-to-end worker | S2 — strategist then producer |
| --- | ---: | ---: |
| Exact candidate | `79510f511af2a75a23bf96b7594e31e17b32fdb7` | `a17f320e7073278037a73bfb06063af370db5676` |
| Protocol / clean exact artifact | pass | pass |
| Native worker verifier | 123 checks pass | 106 checks pass |
| Frozen evaluator | 14/19 | 15/19 |
| Blind mean score | 8.2/10, revise | **8.8/10, accept** |
| Cognitive-span latency | 2,117.72 s (35.30 min) | 2,280.40 s (38.01 min) |
| Reported cost | $1.64513844 | $2.29927348 |
| Worker cost | $1.08914980 | $1.66178624 |
| Supervisor cost | $0.55598864 | $0.63748724 |
| Used tokens / tools | 232,209 / 121 | 280,932 / 119 |
| Turns / Works / Attempts | 5 / 1 / 2 | 5 / 2 / 2 |

S2 cost 39.8% more and its cognitive span was 7.7% longer. Its advantage was whole-outcome quality:
strict atomised claim evidence, explicit anti-embellishment decisions, an operational measurement
contract and a real strategist artifact that the downstream producer used. S1 was more vivid, but its
public copy and claims register introduced product specifics outside the frozen truth set. The
reviewer's medium-confidence preference was S2.

The frozen evaluator remains a mechanical diagnostic, not the quality verdict. Several failures were
literal-string or invented-event checks not required by the task. It was kept unchanged; the blind
review used exact candidate artifacts, no topology labels and the frozen scenario/rubric. Exact review
output is [`t15-t2-blind-review.json`](t15-t2-blind-review.json); mapping and prompt hashes remain in
`v2/workdir/exp03-blind-t2/mapping.json`.

## What generalises

Use a second specialist when its upstream artifact changes the downstream producer's causal choices
and the final outcome needs differentiated expertise. Do not call this parallelism: the S2 path was a
dependency-linked sequence. The useful unit was not “another person” but a reusable strategy boundary
that improved evidence discipline. Stop at two for this evidence; no larger marketing cell is
justified.
