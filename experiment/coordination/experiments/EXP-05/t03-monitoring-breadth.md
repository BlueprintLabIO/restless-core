# EXP-05 monitoring breadth result

**Completed:** 26 August 2026
**Disposition:** scoped Q2 parallel-breadth win; no cognitive fan-in

## Main answer

Two workers materially improved a 40-entity, 280-document monitoring queue when each worker owned
disjoint entities and returned complete alerts. No model rewrote the alert set into a memo. The result
is the cleanest EXP-05 evidence that independent search breadth can scale horizontally.

| Arm | Exact coverage | Accepted throughput | p90 latency | Charged cost/alert | Lead active fraction | Blind result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Q1 | 40/40 | 602.9/h | 218.7s | $0.012948 | 29.8% | accept; mean 8.58, worst 8.5 |
| Q2 | 40/40 | 1,155.6/h | 121.1s | $0.011048 | 46.6% | accept; mean 8.58, worst 7 |

Q2 delivered 1.917× accepted throughput, 0.554× p90 latency and 0.853× charged cost per accepted
alert. Both arms had full exact coverage and no high-consequence breach. Q2's average blind score held,
but its worst entity fell by 1.5 points because uncertainty phrasing and feed ordering were less
consistent. This is a real tail warning, not enough to overturn the accepted result under this
fixture's frozen monitoring contract.

Evidence: [`Q1`](results/monitoring-q1-r1-20260826-glm53-r1/run-result.json),
[`Q2`](results/monitoring-q2-r1-20260826-glm53-r1/run-result.json), and
[`comparison`](results/program-status.json).

## Fan-in disposition

The useful owner surface was the deterministic alert feed/index. There was no named decision,
contradiction or integrated artifact requiring cognitive synthesis, so no fan-in ran. A later memo is
justified only when several accepted alerts must jointly change one explicit allocation, response or
go/no-go decision.
