# EXP-03 four-primitives conformance

**Date:** 24 August 2026
**Primary run:** `exp03-final-primitives-20260824`
**Result:** 16/16 focused checks passed

The minimal implementation frozen in `t09-four-primitives-hypothesis.md` passes the exact failure shape
observed in T2:

1. two non-descendant repair commits for one Work remain independently resolvable under unique Attempt
   refs;
2. exact completion rejects a mismatched commit and a non-coordinator, then closes without parsing a
   free-form subject;
3. a worker judgement request blocks its exact Attempt and Work;
4. resolution reactivates the same Work at revision +1 and queues exactly one worker wake;
5. the next Attempt reuses the preserved workspace;
6. archive-native review exposes a verifier that secretly requires `.git`, accepts its portable repair,
   and leaves the canonical checkout untouched;
7. repository custody proof is explicitly separated from artifact-domain proof;
8. explicit completion rejects a dirty canonical checkout, including untracked transport scratch; and
9. SQLite reports healthy state.

Regression evidence:

- `exp03-supervisor-architecture-r5`: 16/16;
- `exp03-supervisor-multi-architecture-r2`: 9/9;
- `exp03-final-fault-20260824`: 41/41, including 256 ordered reply-free trace notifications over one
  hot connection and coordinator restart recovery.

The conformance rerun followed three observed harness repairs: event telemetry now reuses a persistent
connection instead of opening one socket per token; exact completion requires the candidate commit to
be the clean canonical checkout; and a postflight command can recertify preserved truth without
rewriting an immutable terminal summary. The proof split also prevents a repository `.git` dependency
from leaking into an archive-native artifact verifier.

No scheduler, artifact lifecycle, append-only ledger or additional workflow state was introduced. The
existing transaction, outbox, Work revision and process-supervision paths were reused.

Deterministic conformance proves only that the harness permits recovery. The fresh live GLM-5.3 run
`exp03-t2-s2-primitives-glm53-r1` decides whether the primitives improve real coordination behaviour.
