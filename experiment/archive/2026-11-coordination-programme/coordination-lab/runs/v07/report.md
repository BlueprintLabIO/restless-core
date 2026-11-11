# v07 — loose Exec-to-worker prose handoff

## Change under test

Add a read-only Exec that frames one bounded worker brief, then pass its final prose to one writable
worker. There is no Work/Attempt record, dependency edge, callback, or artifact-bound review.

## Evidence

- Exec model: `nvidia/nemotron-3-super-120b-a12b:free`
- Worker model: `poolside/laguna-s-2.1:free`
- Both launches independently passed the live zero-price gate
- Exec used two reads (`.` failed as non-file, then `README.md`) and produced a formally specific brief
- The brief commissioned Sunleaf exploration, six creatures, temperament behavior, bonding, and team
  switching—features the README explicitly says are already implemented and verified
- The brief's non-goals excluded combat and evolution, also already implemented and verified
- Worker spent 14/14 turns and 26 tool calls re-inspecting the existing implementation
- Worker usage: 177,584 input / 11,535 output / 41,248 cache-read tokens; $0
- Worker terminal: truthful `max_turn_requests` / `max_turns`
- Final seed and status: unchanged `514b7b3...`, clean; no artifact or commit
- Raw trace growth: 4 MB Exec + 112 MB worker

## Score

Outcome score: **15/100** (no-artifact cap 39).

| Dimension | Points | Evidence |
| --- | ---: | --- |
| Accepted outcome /30 | 0 | no artifact or change |
| Coordination /20 | 2 | accountable prose handoff existed, but commissioned already-complete work |
| Recovery/truth /15 | 5 | max-turn state truthful and files preserved |
| Review/evidence /15 | 0 | no outcome to review |
| Efficiency/attention /10 | 0 | 14-turn repeated orientation, no output |
| Harness/control /10 | 8 | exact launches/events/scope/usage; no cancellation needed |

## Dominant failure

Adding an Exec is not enough. The current capability state was buried beneath a large mandate, and the
Exec did not reconcile its own brief with the README it had read. The result was a precise but stale
assignment. The worker then lacked a generic way to reject/satisfy/redirect that brief and kept
orienting until its ceiling.

The next coordination context must lead with observed current state, unknowns, and gaps. `commission`
must create inspectable Work, while `report(blocked|outcome_met)` gives Staff a terminal response to a
bad assignment. This is not a larger prompt; it is a better organisational handoff.

## Decision

Preserve as mode-B baseline. Do not add more prose or worker turns. First remove telemetry
amplification, then test explicit Work/callback semantics.
