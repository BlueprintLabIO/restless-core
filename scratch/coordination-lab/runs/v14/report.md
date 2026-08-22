# v14 — explicit provider repair reaches a positive terminal callback

## Failure from v13

The first Attempt on the focused Work ended unknown after Laguna XS encountered a shared-pool 429.

## Change under test

Repair the same durable Work, increment its revision, and run a new Attempt with a different live-free
model. Do not create replacement Work and do not hide model substitution inside the harness.

## Evidence

- Work: same `work-7c2b86b87b`
- Revision 1 / Attempt `attempt-f53cfe7821`: remains `unknown` with exact provider-error reason
- Explicit Exec `redirect(action=repair)` created revision 2
- Revision 2 model: `cohere/north-mini-code:free`; live prompt/completion prices `0`
- Usage: 25,286 input/cache-read / 871 output; 10 tool calls; $0
- New commit: `6fbc7a21167d0d303750baa3182048fd17eb9db1`
- Parent is exact Work input: `514b7b3d0a65e093af608b08ca142344412181f4`
- Exact artifact: `docs/v13-positive-callback.md`
- Contents: `RESTLESS_POSITIVE_CALLBACK_V13`
- SHA-256: `6a4b0c4bdecd14b6da491df261fe4da2a31ec012e822850b1c2f3ba909312e2c`
- Declared gate ran as exact argv `["test","-s","docs/v13-positive-callback.md"]`: exit 0
- The actor called `report(outcome_met)` in the original revision-2 process
- OrgIntel state: Attempt produced, Work completed, file and commit artifact references recorded
- Workspace clean; SQLite quick check OK
- No controller inferred completion and no finalisation process was launched

## Score

Focused positive-callback score: **95/100**.

All correctness properties in the probe passed. Five efficiency points are withheld because a
one-line artifact required ten tool calls; this is model/tool ergonomics, not callback correctness.
This score is not outcome-leaderboard comparable.

## Decision

Retain the semantics:

- model/provider selection is part of an Attempt launch;
- a provider failure ends that Attempt as unknown;
- an explicit organisational repair creates a new Attempt/revision on the same Work and workspace;
- only the actor's successful callback plus substrate checks completes Work.

Next fix the remaining telemetry amplification for tool-call deltas before another full outcome run.
