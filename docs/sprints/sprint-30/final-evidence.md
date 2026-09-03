# Sprint 30 final evidence

**Decision:** Pass

**Source state:** local `dev` worktree on 31 August 2026. This receipt records verification; it does not
claim a commit or deployment that was not requested.

## Contract results

| Slice | Result | Strongest evidence |
|---|---:|---|
| T1 nominal route | Pass | One Exec-commissioned coherent Work, exact worker Attempt, passing gate, zero lead messages/turns. |
| T2 material wakes | Pass | 100 progress facts and clean completion stayed silent; related failures coalesced; ambiguity, effect authority, conflict and owner correction each created one durable lead obligation. |
| T3 request accounting | Pass | Four same-actor requests settled under distinct IDs; three exact charges survived one request-local unknown; replay did not append or double charge; late exact settlement reconciled after reopen. |
| T4 review custody | Pass | Root-owned detached Git tree, content digest, stable alias and reviewer-identity read/non-write probe. Mutation and unreadable-file injections were refused. |
| T5 telemetry | Pass | Read-only `restless telemetry` projection reconciled model, Work, Attempt, gate, wake, intervention, duplicate and replacement facts. Unsupported measurements serialized as unknown. |
| T6 residue | Pass | Attempt/gate/session cleanup verified absence; live leases were empty; owned and orphaned process sessions were killed and re-observed absent. |
| T7 integrated fixture | Pass | One live Docker/Postgres fixture combined the nominal route, parallel gates and every authority/runtime adversary, then returned zero scoped residue. |

## Verification run

- `cargo check --workspace` passed.
- Focused model relay suite: 26 passed.
- Request-local accounting test passed, including reopen and idempotency.
- Exact-execution Postgres suite: 4 passed.
- Telemetry collector Postgres scenario passed.
- Local immutable review scenario passed.
- Live integrated Docker/Postgres fixture passed in 14.81 seconds after the final process-session case
  was added.
- Full workspace passed with no failures. The `restlessd` target reported 235 passed and eight
  intentionally ignored live/external cases; generated TypeScript bindings and all other workspace
  targets passed in the same run.
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` and
  `git diff --check` passed.
- The closing sweep found no scoped review, gate, Attempt, agent-session or repository residue. One
  disposable schema from an earlier interrupted exact-execution test was identified by its unique
  test prefix, removed explicitly and re-observed absent.

## Adversarial injections retained in source

- related gate failures and contract ambiguity;
- effect-authority request, owner correction and cross-worker conflict;
- one corrupt/missing provider terminal among concurrent requests;
- exact accounting replay;
- reviewer mutation and unreadable declared file;
- engine error despite exit zero, leaked child, timeout and orphaned process marker;
- concurrent resource allocation and cache coalescing;
- daemon/OrgIntel reopen with unread material judgement; and
- exact Attempt, gate, alias, review and agent-session residue probes.

## Learnings

1. Accountability and model activation are separate. A lead can remain responsible and interruptible
   while exact policy handles the routine path with no paid narration.
2. Failure domains should shrink to the smallest durable identity. Request IDs prevent one ambiguous
   provider terminal from rewriting the truth of its siblings.
3. “Immutable” is not an ownership claim. Custody becomes useful only when the exact reviewer proves
   it can traverse and read the declared content, cannot write it and has the narrowly scoped Git trust
   needed to inspect it.
4. Telemetry is a projection, not another database. Unknown active time or tool outcomes must stay
   unknown until Runtime emits canonical facts.
5. Cleanup is part of terminal correctness. Issuing `rm` or `kill` is not evidence; re-observing absence
   is the receipt.
6. Integrated fixtures find cross-layer defects that unit tests miss. The live run exposed Git's
   ownership safety check only after review custody became genuinely root-owned; scoped per-process
   trust fixed the access problem without weakening global repository safety.

## Next empirical work

Run EXP-18's frozen single-worker versus four-worker independent-unit comparison, then start Dogfood 4 v0.6
from its sealed playable baseline. Neither outcome is pre-claimed by this implementation pass.
