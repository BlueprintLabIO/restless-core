# T04 — T1 coupled-product result

**Disposition:** counted; both organisations missed the company completion gate
**Date:** 24 August 2026
**Primary failure class:** model/tool-loop throughput
**Secondary cost:** sequential supervisor handoff and review boundary

## Result in one sentence

With GLM-5.3 at `high`, both arms produced implementations that passed the frozen 11/11 native outcome,
but neither completed the company outcome inside the 30-minute envelope: the supervised arm returned
a stronger clean worker artifact too late for supervisor review/promotion, while the direct actor was
interrupted before commit or terminal report.

## Matched results

| Measure | S1-T — supervisor + worker | C0 — direct actor |
| --- | ---: | ---: |
| Frozen input/evaluator | exact hash match | exact hash match |
| Company outcome | **fail** | **fail** |
| Elapsed at terminal | 1,850.65 s | 1,894.64 s |
| Cognitive turns | 2 | 1 interrupted |
| Work | exactly 1 | exactly 0 |
| Candidate at terminal | clean seed; worker commit unpromoted | dirty seed checkout; no commit |
| Frozen evaluator on produced tree | **11/11** | **11/11** |
| Own new harness | self-hosting, **23/23** | **13/13** only with a manually supplied server; otherwise connection refused |
| Existing browser suites | 12/12 + 7/7 + 29/29 | 12/12 + 7/7 + 28/29 in terminal check; immediate rerun 29/29 |
| Provider-reported spend | US$1.51666348 | unknown after forced process end |
| Recorded tokens | 150,584 used; 60,027 output | unknown after forced process end |
| Recorded tool calls | 80 | unknown after forced process end |

The C0 roster/evolution failure did not reproduce and is classified as a harness flake. It had no
console, page or request errors. C0's frozen evaluator pass proves the dirty implementation's target
behaviour, but does not cure its missing commit, missing callback or non-self-contained proof harness.

## S1-T trace

- The supervisor spent 434.95 seconds, 46,869 tokens, 15,584 output tokens, 15 tool calls and
  US$0.16748272 building a causal model and commissioning exactly one end-to-end Work.
- Its brief correctly anticipated fresh-key semantics, browser auto-repeat, the zero-damage clamp,
  delayed projectile leakage, burn isolation, energy regeneration, discoverability and native proof.
- The worker spent 1,384.99 seconds, 103,715 tokens, 44,443 output tokens, 65 tool calls and
  US$1.34918076. It returned clean commit
  `62920588d8baf30b36e05f0c9e34860507b28fa6` plus observed gates.
- The worker callback arrived during drain at about 29 minutes 53 seconds from the supervisor's first
  turn start. The run terminal followed before the queued supervisor wake could inspect and promote
  the commit.
- Protocol therefore correctly reported `valid: false`: the candidate remained the seed rather than
  the exact worker artifact. The lead authored no production.

## C0 trace

- The direct actor spent roughly 13.5 minutes before its first visible edit and entered browser proof
  around 28 minutes 49 seconds.
- It implemented the same observable mechanic and passed the frozen evaluator 11/11.
- The 120-second drain ended while its ACP turn was still open. The tree remained dirty with five
  changed/new files and no completion decision.
- The interrupted ACP process returned no final usage record, so the apparent US$0 cost and zero tool
  calls are missing telemetry, not free cognition.
- Its promised harness depended on a server at port 8124 and immediately failed with
  `ERR_CONNECTION_REFUSED` when executed as the required standalone command. With a manually supplied
  server its 13 checks passed.

## What this cell establishes

1. **No coordination churn occurred.** S1-T used one correct Work, one worker, one attempt, four lease
   renewals and no polling, retries, reassignment or duplicate production.
2. **Actor cadence dominated latency.** Both producers spent long periods between visible tool calls;
   both deferred browser proof until the end of the envelope.
3. **Supervision improved closure quality in this sample.** The worker returned a clean attributable
   commit, stronger standalone proof and fully green regressions. The direct artifact achieved the
   target behaviour but did not close its own operational contract.
4. **Supervision also added an unaffordable sequential premium for this small coupled task.** The
   supervisor's seven-minute causal audit delayed production, and its mandatory final judgement could
   not occur after the late worker callback.
5. **Callback completion is correct but insufficiently observable.** Lease renewals proved liveness;
   phase/tool telemetry would have distinguished analyse, edit, verify and handoff without waking the
   supervisor or inferring failure from elapsed time.
6. **Forced process end loses accounting.** Usage must be streamed or checkpointed during a turn, not
   recovered only from its final ACP response.

## Design consequence, not an architecture reversal

The owner decision that leads remain non-producing supervisors stands. T1 instead narrows the
optimisation target:

- commission stable, well-specified work quickly; do not require an exhaustive repository audit before
  a worker can start;
- allow supervisory risk analysis to proceed without blocking production, and deliver only material
  deltas through explicit events;
- checkpoint provider usage and expose low-cost phase/tool liveness from the runtime;
- make proof and handoff an early worker obligation rather than end-of-envelope cleanup;
- preserve enough post-artifact budget for mandatory supervisor judgement, or treat a worker callback
  near the budget edge as an incomplete company outcome;
- test whether shared/forked working history can retain the supervisor's useful causal insight without
  forcing the worker to re-orient from a long prose brief.

Do not infer a universal supervisor premium from one coupled coding cell. T2–T5 must determine whether
the value/cost relation changes for mixed creative work, replicated units, volatile customer work and
independent evidence breadth.

## Evidence locations

- S1-T raw summary: `v2/workdir/exp03-t1-s1t-glm53-r3/summary.json`
- S1-T worker tree: `v2/workdir/exp03-t1-s1t-glm53-r3/workspaces/work-d339b004ea`
- C0 raw summary and dirty tree: `v2/workdir/exp03-t1-c0-glm53-r1/`
- Excluded setup preflights: [`t03-t1-preflight-invalid.md`](t03-t1-preflight-invalid.md)
