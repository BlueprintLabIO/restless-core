# EXP-03 T5 result — independent-evidence research

**Recorded:** 24 August 2026
**Status:** matched set closed; S3 clear loss
**Models:** every cognitive actor `zai/glm-5.3`, high reasoning
**External effects:** none; the corpus and decision were frozen and fictional

## Decision

One end-to-end analyst beat two parallel evidence workers plus a synthesis worker. The S3 evidence
split was real, concurrent and lossless, so this result is not explained by the source-transfer defect
that invalidated earlier research runs. At this corpus size the extra evidence boundaries did not add
enough independent insight to repay synthesis, supervisory review and verifier repair.

## Matched results

| Measure | S1 — one analyst | S3 — two evidence workers + synthesist |
| --- | ---: | ---: |
| Exact candidate | `0a5c55e6c88f899a000d2772687ea7e73fa96b19` | `dd74ae6de684d758da769aae568c92509888cedd` |
| Exact protocol | pass at terminal | **pass on explicit postflight recertification** |
| Native artifact verifier | pass | pass; checkout and no-`.git` archive independently pass |
| Frozen evaluator | **18/18** | 17/18; one diagnosed regex false negative |
| Blind mean / acceptance | **8.6, accept** | 8.0, accept |
| Blind preference | **S1, medium confidence** | — |
| Cognitive-span latency | 3,138.13 s (52.30 min) | about 3,611.05 s active (60.18 min) |
| Total elapsed including founder pause | 52.30 min | 75.55 min |
| Reported cost | **$1.84067112** | $3.33292420 |
| Worker cost | $1.07369548 | $1.69581340 |
| Supervisor cost | $0.76697564 | $1.63711080 |
| Used tokens / tools | 260,867 / 106 | 497,310 / 202 |
| Turns / Works / Attempts | 5 / 1 / 2 | 11 / 3 / 5 |

S3 cost 81.1% more, used about 90.6% more tokens and tools, and took about 15.1% longer on the active
execution clock. Its two evidence workers did run concurrently and their four evidence files arrived
in the final commit byte-identically. The remaining cost came from a synthesis boundary, six
supervisor turns and repair of the final verifier.

The first terminal summary truthfully failed protocol because the canonical checkout contained one
untracked harness transport file, `xd__decide.json`. The new completion guard now rejects that state.
After removing that exact scratch file, the explicit postflight command recertified the preserved
candidate as clean, exact, protocol-valid and archive-native without changing the immutable terminal
record. This is a harness repair, not a favourable reinterpretation of S3.

The frozen evaluator's sole S3 failure requires literal `/next action/i`; the 13,637-character memo has
the explicit heading “Next executable action” and a structured `decision.next_action`. The frozen
17/18 result remains recorded, while the defect is classified as evaluator/contract. The blind review
still preferred S1: S1 supplied a more complete week-by-week operating sequence; S3 was stronger on
denominator and failure awareness but contained a load-bearing arithmetic overclaim and left capacity
underallocated. Exact review output is [`t16-t5-blind-review.json`](t16-t5-blind-review.json).

## Boundary learned

Parallel research is not valuable merely because sources divide cleanly. Activate it when the corpus
or search space exceeds one analyst's effective context, regions require genuinely different
expertise or access, evidence outputs are independently useful, or latency matters enough to repay a
separate synthesis pass. For a source-complete corpus that one strong worker can hold, current span is
one.
