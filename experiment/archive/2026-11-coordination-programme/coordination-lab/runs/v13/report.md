# v13 — positive callback probe interrupted by free-pool rate limit

## Change under test

Run one focused Staff Work through the real coordinator and first-party ACP/Pi harness:

1. create one exact marker file;
2. advance the Git commit;
3. pass `test -s` expressed as structured argv;
4. call `report(outcome_met)` in the original actor process.

## Evidence

- Model: `poolside/laguna-xs-2.1:free`; live prompt/completion prices `0`
- Turn limit: 10
- The actor made three tool calls, then OpenRouter returned an upstream shared-pool 429
- Runtime result: `error` / ACP-compatible `refusal`, with the exact provider error retained
- Usage before failure: 5,869 input + 3,040 cache-read / 301 output; $0
- OrgIntel result:
  - Attempt `attempt-f53cfe7821`: `unknown`
  - Work `work-7c2b86b87b`: `blocked`
  - reason: `Actor process ended as error without a terminal report`
  - artifacts: 0; decisions: 0
- Workspace remained clean at exact input `514b7b3d0a65e093af608b08ca142344412181f4`
- No automatic finalisation, inferred retry, or false completion occurred
- SQLite quick check: OK

## Score

Focused recovery score: **90/100**.

The provider-error path is truthful and preserves state, but the intended positive callback path
remains unproven, so this is not 100 and is not an outcome leaderboard score.

## Decision

Retain the unknown state and repair the same Work with a different live-free model. Do not prepare a
fresh Work: provider substitution should resume the durable organisational responsibility and its
workspace rather than discard provenance.
