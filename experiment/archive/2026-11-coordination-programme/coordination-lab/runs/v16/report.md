# v16 — Runtime-neutral Staff context repairs preserved partial Work

## Failure from v15

Staff's prose said “current working directory,” but observed-state JSON leaked the Docker adapter's
`/workspace` locator into a Pi process running in a host worktree. North spent most of its turn probing
contradictory paths, wrote the marker last, and ended unknown before commit/callback.

## Change under test

Project organisational state now projects only `git_status` and `head`. The Runtime Bridge supplies
cwd through the launch contract; OrgIntel no longer repeats an environment-specific workspace/cell
locator. Repair the same Work so its preserved uncommitted file is reused.

## Evidence

- Same Work: `work-1ab1984f93`; revision 1 remains unknown
- Preserved revision-1 state entering repair: `?? docs/` with exact marker file present
- Revision 2 model: `cohere/north-mini-code:free`; live prompt/completion prices `0`
- No tool call referenced `/workspace`; no path error occurred
- New commit: `915c63a86572b1f5bafa83cac7aa93a43888ee10`
- Exact marker file reused and committed; workspace clean
- Structured `test -s` gate: exit 0
- Original revision-2 actor called `report(outcome_met)`
- Attempt produced; Work completed; file and commit artifacts recorded; SQLite quick check OK
- Usage: 22,326 input/cache-read / 1,030 output; nine tool calls; $0
- Durable trace: 54,364 bytes
- Comparison: revision 1 used 12 calls and ended unknown; revision 2 used 9 and completed

## Score

Focused context-and-recovery score: **96/100**.

Correctness and reuse passed. Four efficiency points remain open because the actor still listed Git
internals and redundantly re-wrote already-correct content before reporting.

## Decision

Retain. Organisational context carries stable identity and evidence, not Runtime-local paths. Hide
`.git` from ordinary list output next, then return to the full game mandate.
