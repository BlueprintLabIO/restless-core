---
title: "Capacity has an arrival shape"
deck: "The same sales population rewarded two workers when it arrived at once and did not when it arrived gradually."
thesis: "Parallel capacity becomes useful when independently closing work is waiting at the same time and earlier completion changes its value."
publishedAt: 2026-08-26
order: 2
readTime: "6 min"
status: "Provisional evidence"
experiments: ["EXP-04", "EXP-05"]
evidence:
  - label: "Local-closure result"
    locator: "experiment/coordination/experiments/EXP-04/t02-final-results.md"
    scope: "Sales and monitoring queues at smaller tested backlogs."
  - label: "Demand-capacity result"
    locator: "experiment/coordination/experiments/EXP-05/t01-sales-demand-capacity.md"
    scope: "Matched 240-account sales backlogs with tail-quality gates."
  - label: "Monitoring result"
    locator: "experiment/coordination/experiments/EXP-05/t03-monitoring-breadth.md"
    scope: "Forty locally closing monitoring entities under the same model family."
  - label: "Programme conclusion"
    locator: "experiment/coordination/experiments/EXP-05/t05-final-results.md"
    scope: "Controlled test companies only. No demand or revenue claim."
---

"How much work is there?" is usually the wrong question. A large total can arrive slowly enough
that a second worker spends its useful window getting ready. The same total can arrive all at once,
where waiting is expensive and independently closing work can overlap.

## What we expected

We expected a queue to make parallelism obviously worthwhile. EXP-04 made the first correction.
Local closure removed the need for a model assembler, but at 48 sales accounts and 12 monitoring
entities, one worker was already fast enough that another worker's setup and lead review erased the
overlap.

Local closure was necessary. It was not the crossover.

## What happened

EXP-05 used the same 240-account sales population in different arrival shapes. With the whole backlog
ready at once, two workers crossed the frozen throughput threshold twice. With the same accounts
arriving gradually, two workers improved throughput but did not improve the p90 response time that
would justify their fixed cost. Four workers were faster in one arm but weakened the frozen tail
quality comparison, so the accepted ceiling remained two.

An independent monitoring workload reached a similar but separate result at 40 entities. Two workers
delivered roughly 1.9 times throughput, reduced p90 latency and lowered charged cost per accepted
alert. The aggregation was deterministic. No model had to reauthor the individual unit results.

## What changed

The organisational question is now narrower: are useful, independent units waiting while the current
worker is occupied, and does earlier completion change the value of the work? If both are true, a
lead can add capacity and inspect the marginal quality. If either is false, a larger team can be
performative overhead.

This is why a department name is weak evidence. Sales and monitoring happened to provide locally
closing units. Another sales queue with paced arrivals did not justify the same span.

## Where the conclusion stops

The accounts and monitored entities were controlled `_test` inputs. The result says nothing about
real customer demand, universal service levels or the right worker count under another provider. It
does establish a practical test: measure the ready backlog, response value, quality floor and lead
cost before calling extra capacity an improvement.
