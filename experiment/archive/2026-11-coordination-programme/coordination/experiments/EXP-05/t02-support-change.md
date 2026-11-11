# EXP-05 customer-operations material-change result

**Completed:** 26 August 2026
**Disposition:** causal supervision prevents stale completion; terminal-only supervision produced a
counted outcome failure

## Main answer

A non-producing lead must be addressable while work is live when policy or harm changes materially.
Waiting until terminal review was not merely slower: it left 16 of 96 cases on stale policy and ended
in a stable semantic deadlock. Causal delivery reached the exact Work boundary, superseded the two
stale Attempts and closed all 96 cases under policy v2.

The causal lead was not infallible. Its final package still received `repair` because one ordinary
case inherited duplicate-case language and several claims overstated internal evidence. Availability
and correct event routing preserve the opportunity to supervise; they do not guarantee good
supervisory judgement.

## Matched result

| Arm | Terminal disposition | Accepted | Stale v1 accepted | Redirect / repair | Charged spend | Blind result |
| --- | --- | ---: | ---: | --- | ---: | --- |
| S-T terminal delivery | `counted_outcome_failure` | 80/96 | 16 | lead repaired batches 03–12 but incorrectly grandfathered batches 01–02 | $1.7859 | not run; exact outcome failed |
| S-E causal delivery | `counted` | 96/96 | 0 | two running Attempts superseded; two replacements; first replacement 33.99s after effective change | $1.6832 | repair; mean 8.42, worst 5, no high-consequence breach |

The causal arm ran at 558.6 accepted cases/request-hour, p90 446.4s, peak Staff model concurrency two,
lead active fraction 38.2%, 15 lead wakes and 14 Attempts. The first stale Attempt terminated 31.87s
after delivery without being accepted.

Evidence: [`causal run`](results/support-causal-r1-20260826-glm53-r5/run-result.json) and
[`terminal outcome failure`](results/support-terminal-r1-20260826-glm53-r1/run-failure.json).

## Why the terminal arm counts as an outcome failure

All 12 original Work batches completed and every actor session was terminal. The lead explicitly
waited for validator completion that could never arrive because C001–C016 remained policy v1. Work,
actor-session, artifact and exact-gate evidence jointly showed a stable failure; the operator then
stopped the outer process. No timeout decided success, no post-hoc repair hint was injected and no
production was replayed.

## Product consequence

Keep the existing primitives. A material event should causally reach the accountable lead and exact
affected Work; ordinary success remains coalesced. Existing exact outcome gates must expose terminal
failure evidence as well as success so a lead cannot wait forever on a success-only callback. This is
narrow event/gate wiring, not a universal workflow engine or deterministic supervisor policy.
