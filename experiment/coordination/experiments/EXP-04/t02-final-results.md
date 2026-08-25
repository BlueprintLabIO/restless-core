# EXP-04 final results — local closure is necessary, not sufficient

**Completed:** 25 August 2026
**Disposition:** accepted sparse evidence; no parallel crossover established at the tested loads
**Primary model:** live first-party `zai/glm-5.3`
**External effects:** none; all sales, support and monitoring inputs were fictional `_test` fixtures

## Main answer

Removing the model assembler works mechanically, but it does not by itself make more workers faster.
At 48 accounts and 12 monitored entities, one strong worker was already fast enough that another
actor's fixed orientation plus the required lead review erased the available overlap.

Parallel queue workers therefore need **both**:

1. locally closing, disjoint units with no cognitive batch rewrite; and
2. enough valuable backlog, service urgency or search breadth to amortise actor startup and lead review.

Local closure is a prerequisite for scaling, not evidence that the crossover has been reached.

## Counted and diagnostic results

| Cell | Exact/native quality | Worker-window throughput | Request / lead active | Cost per accepted unit | Decision |
| --- | --- | ---: | ---: | ---: | --- |
| Sales Q1 | 48/48; blind 9/10 usefulness, 10/10 grounding/tail | 1,356.0 accounts/h | 257.0s / 155.2s | $0.005085 | baseline |
| Sales Q2 pre-coalescing diagnostic | 48/48; same exact tail | 1,109.1 accounts/h (-18.2%) | 509.3s / 403.5s | $0.014255 | loses; also exposed an unnecessary intermediate lead wake |
| Sales Q2 corrected consequence-partition diagnostic (E1 uniform) | 48/48; exact tail 10/10 | 1,631.2 accounts/h (+20.3% vs Q1) | 266.8s / 184.4s | $0.005967 | misses +25% gate; end-to-end slower and 17.3% dearer |
| Monitoring Q1 | 12/12 exact; blind 9/10 usefulness, 10/10 grounding/tail | 418.6 alerts/h | 365.1s / 276.4s | $0.025942 | baseline |
| Monitoring Q2 diagnostic | 12/12 exact | 419.7 alerts/h (+0.3%) | 414.5s / 347.4s | $0.038133 | no throughput gain; stops Q4 |

The corrected Q2 sales diagnostic used a different, predeclared consequence partition for E1, so it
does not replace the frozen interleaved Q2 arm. It does show that removing the redundant wake is not
enough to cross the topology threshold.

## Effort and closure

E1 held the lead and exception worker at `high` and changed only the routine worker:

| Effort policy | Exact tail | Worker window | Request | Cost/unit | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| uniform high | 10/10 | 105.9s | 266.8s | $0.005967 | baseline |
| routine low / exception high | 10/10 | 146.7s | 336.4s | $0.008989 | loss: 38.4% slower worker window, 26.1% slower request, 50.6% dearer |

Configured `low` was not cheaper behaviour: the routine worker took longer and used more tool/model
cadence. Effort is a capability/cadence control to validate per model and workload, not a reliable cost knob.

C1 was non-discriminating and therefore not forced into two redundant calls. The frozen Work set left
no optional production admission for an open envelope to overspend. Real final verification/handoff
already consumed 29.9% of Q1 sales cost and 35.2% of E1 uniform cost. Preserve closure capacity
dynamically; do not encode 25% as a universal bucket.

## Sparse stops

- **Q4/Q8 sales and Q4 monitoring:** not counted. Q4 sustained provider/tool admission was
  infrastructure-invalid: four exact files, only three terminal reports.
- **Q-SUPPORT:** not activated because H7's frozen in-flight policy-change gate was not run.
- **R-DECIDE R4-S:** not activated because Q2 discovery did not win and Q4 capacity failed. This is
  not evidence that broad research can never parallelise.
- **M1 cheaper workers:** not activated because no same-model topology won.
- **P1 concurrent departments:** not activated because the participating parallel department shapes
  had not crossed their isolated baselines.
- **reserve workloads:** not activated; no remaining ambiguity justified a department tour.

These are final sparse dispositions, not missing retries.

## Structural fixes that survived

The retained implementation is small: resumable actor sessions, delta-correct usage, exact live
cancellation, null-preserving telemetry, phase observations, arbitrary non-code fixtures, deterministic
local composition, and coalesced ordinary batch completion. The full conformance record is
[`t01-wave0-conformance.md`](t01-wave0-conformance.md).

## Enduring versus contingent

Enduring:

- local outcomes should remain local; synthesis needs one named cognitive reason;
- local closure is necessary but not sufficient for parallel capacity;
- ordinary success events may be durable without demanding immediate supervisor attention;
- a lead's sample/exception judgement is real work and can dominate queue economics;
- exact deterministic evidence and blind semantic judgement answer different questions;
- actor continuity belongs to the model session and durable facts, not necessarily one immortal process;
- missing provider telemetry stays unknown.

Contingent:

- Q1 as the current sales/monitoring span;
- Q2 as the proved provider envelope and Q4 as invalid today;
- GLM-5.3 `low` losing to `high` on this routine queue;
- the 48-account and 12-entity crossover results;
- ACP/OMP cache and in-flight usage visibility.

## Next experiment

Do not search more team topologies yet. Use a demand-backed queue with arrivals or a much larger frozen
backlog, preserve local closure and coalesced review, then increase workers only while response-time or
accepted-throughput value exceeds fixed actor and lead cost. Separately, test a real external/material
policy change before enabling volatile support queues. A cognitive research fan-in should reopen only
after parallel discovery first demonstrates a coverage or latency win.
