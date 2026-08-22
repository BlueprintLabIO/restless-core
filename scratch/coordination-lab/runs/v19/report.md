# v19 — same owner, live provider rotation, truthful partial-work recovery

## Change under test

Project bounded Runtime recovery evidence onto an unknown Attempt and ignore deferred Staff inbox rows
when deciding whether this bounded comparison runner has runnable work. Resume the preserved v17/v18
Work and exercise actor-independent model selection live.

Preflight: Python compile passed; coordination/adversarial suite 34/34; SQLite quick check OK.

## Evidence

- Backfilled the four v17 Attempt summaries only from their preserved Runtime traces so Exec could see
  the exact Google AI Studio 429 failure domain. States, revisions, owners, files and artifacts were
  unchanged.
- Exec repaired `work-b53ec0f739` without reassigning `gameplay-systems`.
- Revision 5 deterministically selected `cohere/north-mini-code:free`; live proof at
  `2026-08-22T00:52:34.132Z` confirmed zero prompt/completion price and tool support.
- North used 18 tool calls, consumed 394,140 input and 5,631 output tokens, then reached the truthful
  `max_turns` outcome. It left a four-line partial edit in `js/game.js`: cooldown state and a `KeyX`
  call to an as-yet undefined `_tryExploration` method.
- The Attempt became `unknown` with model and `max_turns` recorded; the modified workspace survived.
- Exec repaired the same Work again. Revision 6 selected
  `nvidia/nemotron-3-nano-30b-a3b:free`; live proof at `2026-08-22T00:57:26.066Z` confirmed zero
  price and tool support. It observed North's partial edit, but spent 18 tool calls repeatedly locating
  an insertion point and also ended `max_turns` without advancing the diff.
- Exec repaired once more; revision 7 correctly cycled to Gemma 31B and reproduced the already-known
  Google shared-pool 429 before tokens. The run was stopped before another blind cycle.
- Across all three new Attempts the Work owner remained `gameplay-systems`; no hidden fallback or
  reassignment occurred. The workspace remains modified and uncommitted, artifacts remain empty, and
  SQLite quick check is `ok`.
- New durable event traces total 429,684 bytes. The run used zero dollars but more than 800k accounted
  input/cache tokens across Exec and Staff turns.

## Score

Outcome score: **33/100** (no-artifact cap 39).

| Dimension | Points | Evidence |
| --- | ---: | --- |
| Accepted outcome /30 | 0 | partial code calls an undefined method; no commit or accepted artifact |
| Coordination /20 | 9 | stable ownership and explicit repairs worked; Exec still needed excess recovery wakes |
| Recovery/truth /15 | 13 | exact failures, models and revisions persisted; uncommitted work crossed providers intact |
| Review/evidence /15 | 0 | no reviewable candidate exists |
| Efficiency/attention /10 | 1 | two 18-call turns and a known-bad provider cycle produced four incomplete lines |
| Harness/control /10 | 10 | live free proofs, deterministic selection, stops, usage and ordered traces are exact |

## Dominant failure and 10x decision

The provider/actor boundary is no longer the limiting factor. Recovery context is. The next model sees
only `M js/game.js`, the original brief, and the latest generic feedback; it does not receive the exact
partial diff or prior Attempt outcomes. It consequently repeats discovery instead of finishing the
preserved work.

For v20, assemble recovery context exactly as the OrgIntel spec requires: original brief, previous
Attempts and failure evidence, plus a bounded Git diff/stat of the persistent workspace. This is not
more turn budget or a workflow step. It lets a new model begin at the unresolved delta and leaves the
same actor, Work and seven-command contract intact.
