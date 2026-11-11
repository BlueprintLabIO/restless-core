# v06 — truthful turn-ceiling terminal state

## Failure from v05

The harness imposed a turn ceiling through Pi's graceful `shouldStopAfterTurn`, but Pi retained the
model's last `toolUse` stop reason. The adapter mapped that to `end_turn`/`completed`, concealing that
the model still required a continuation.

## Fix and evidence

The harness now distinguishes a naturally finished response from a tool-bearing turn stopped by the
configured ceiling. Live probe:

- Model: `poolside/laguna-xs-2.1:free`; live prompt/completion prices `0`
- Limit: one turn
- Model called `read`; tool completed
- A second model turn was required to answer but was not permitted
- ACP stop: `max_turn_requests`
- Restless outcome: `max_turns`
- Usage: 220 input / 41 output; $0
- No success was inferred from the successful intermediate tool call

## Score

Harness terminal-truth score: **100/100**.

The probe directly distinguishes completed, cancelled, provider/model error, max tokens, and max
turns. This is a harness-only score, not an outcome score.

## Decision

Retain. A max-turn Attempt remains resumable/unknown from the organisational perspective; OrgIntel
may continue, redirect, or reassign it. The harness does not decide Work status.
