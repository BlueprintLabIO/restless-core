# EXP-05 sales demand and capacity result

**Completed:** 26 August 2026
**Model envelope:** `zai/glm-5.3`, medium for Exec, lead and Staff; high for blind review
**Disposition:** D0 Q2 crossover replicated; D1/D2 did not cross; Q4 stopped on tail quality

## Main answer

Parallel account ownership became valuable when all 240 independently closing accounts were waiting
at once. It did not produce the same response-time gain when the same six batches arrived every 45
seconds. The discriminating variable was demand shape and useful overlap, not the label “sales” or the
nominal size of 240 accounts.

Every counted arm completed exactly 240/240 units with valid attribution and no cognitive assembler.
Charged provider cost is authoritative; token count is not used as a money proxy.

## Counted curve

| Demand / arm | Value-adjusted units/h | p90 unit latency | Charged cost/unit | Lead active fraction | Blind decision / worst unit |
| --- | ---: | ---: | ---: | ---: | --- |
| D0 Q1 anchor | 2,098.5 | 384.1s | $0.002484 | 29.7% | repair / 4 |
| D0 Q2 anchor | 4,164.5 | 203.3s | $0.001988 | 35.6% | accept / 6 |
| D0 Q1 reversed-order replicate | 2,048.1 | 392.1s | $0.002353 | 18.4% | accept / 6 |
| D0 Q2 reversed-order replicate | 5,805.1 | 146.7s | $0.002012 | 40.2% | accept / 8 |
| D1 Q1 | 2,166.9 | 203.5s | $0.002864 | 37.1% | accept / 7.5 |
| D1 Q2 | 2,686.8 | 203.2s | $0.003190 | 55.6% | accept / 6 |
| D1 elastic | 2,098.7 | 206.3s | $0.002644 | 33.5% | repair / 5 |
| D2 Q1 | 2,287.8 | 203.0s | $0.002667 | 41.1% | accept / 5 |
| D2 Q2 | 3,028.3 | 203.2s | $0.002476 | 35.3% | accept / 8 |
| D2 elastic | 2,367.4 | 202.8s | $0.002345 | 35.6% | accept / 7 |
| D0 Q4 conditional | 10,243.4 | 81.4s | $0.001953 | 49.9% | accept / 7 |

Evidence: [`program-status.json`](results/program-status.json) and the linked `run-result.json` in
each named result directory.

## Frozen gates

- **D0 anchor:** Q2 delivered 1.985× throughput, 0.529× p90 and 0.800× charged cost/unit. All six
  crossover gates passed.
- **D0 replicate:** Q2 delivered 2.834× throughput, 0.374× p90 and 0.855× charged cost/unit. All six
  gates passed in reversed arm order.
- **D1:** Q2 delivered 1.240× throughput, 0.998× p90 and 1.114× cost/unit. It missed the +25%
  throughput gate, the 20% p90 gate and the frozen tail tolerance.
- **D2:** Q2 delivered 1.324× throughput and 0.929× cost/unit, but p90 was 1.001× Q1. It therefore
  did not cross even though throughput and tail-quality gates passed.
- **Elastic D1/D2:** the lead retained one worker in both arms; observed peak model concurrency was
  one. These are valid natural decisions, not proof that the same choice is optimal for every paced
  queue.

## Conditional Q4 stop

Q4 was authorised only after the D0 Q2 crossover replicated. Against replicated Q2 it produced
1.765× throughput, 0.555× p90 and 0.971× charged cost/unit, with exact 240/240 closure and four-way
model concurrency. The blind worst-unit score fell from 8 to 7, exceeding the frozen 0.5 tolerance.
Q4 therefore fails the marginal gate despite its raw speed. Q2 is the accepted ceiling for this
specific all-at-once fixture; no Q8 arm is justified.

## Scope

This is one fictional account policy, one model family and one provider/runtime envelope. It supports
a demand-sensitive staffing judgement, not a threshold router. A real queue still needs observed
backlog age, arrival shape, response value, exception rate, provider headroom and marginal quality.
