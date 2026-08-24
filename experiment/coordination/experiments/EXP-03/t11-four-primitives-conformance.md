# EXP-03 four-primitives conformance

**Date:** 24 August 2026
**Primary run:** `exp03-four-primitives-r1`
**Result:** 13/13 focused checks passed

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
   and leaves the canonical checkout untouched; and
7. SQLite reports healthy state.

Regression evidence:

- `exp03-supervisor-architecture-r5`: 16/16;
- `exp03-supervisor-multi-architecture-r2`: 9/9;
- `exp03-four-primitives-fault-r1`: 39/39.

No scheduler, artifact lifecycle, append-only ledger or additional workflow state was introduced. The
existing transaction, outbox, Work revision and process-supervision paths were reused.

Deterministic conformance proves only that the harness permits recovery. The fresh live GLM-5.3 run
`exp03-t2-s2-primitives-glm53-r1` decides whether the primitives improve real coordination behaviour.
