# EXP-03 four-primitives recovery hypothesis

**Date:** 24 August 2026
**Status:** frozen before implementation

## Observation

The first live T2 S2 run produced a strong campaign through a real GLM-5.3 strategist, producer and
non-producing supervisor. The supervisor then found a real packaged-artifact defect. Repair did not
fail because the organisation lacked judgement: it failed because the harness stored every revision
of one Work behind one fast-forward-only Git ref, interpreted run completion through a magic decision
subject, and did not resume the requesting worker when its judgement was resolved.

The harness already has mutable SQLite operational state, transactions, an outbox and persistent
workspaces. Rebuilding a ledger, asset-custody system or workflow engine would duplicate machinery and
repeat the legacy failure.

## Hypothesis

Four local primitives are sufficient:

1. **Attempt identity:** import each produced commit under one unique
   `refs/heads/attempts/<attempt-id>` anchor. The artifact remains an ordinary Git commit recorded on
   its Attempt. Repair never overwrites an earlier attempt or needs a fast-forward relationship.
2. **Explicit completion:** the accountable lead calls `complete_run(candidate_commit, rationale,
   evidence)`. The operation records the canonical `run=complete` decision itself; free-form decision
   subjects are no longer transport.
3. **Judgement resumes Work:** a worker judgement request terminally blocks that Attempt. Resolving the
   request reactivates the same Work as its next revision, preserves its workspace and messages, and
   queues one worker wake.
4. **Archive-native review:** the supplied review command executes the exact candidate commit from a
   Git archive without `.git`. It exposes packaging defects before `complete_run` while leaving the
   acceptance judgement with the lead.

## Non-goals and deletion boundary

- no universal command type or cross-product command algebra;
- no immutable mutation ledger;
- no content-addressed asset custody or artifact lifecycle;
- no new general workflow states or workflow interpreter;
- no deterministic quality gate beyond observable Git identity and tool execution;
- no production OrgIntel change until a real model run proves the experiment seam.

If these primitives require a second scheduler, general recovery graph, artifact promotion protocol,
or more than the existing Work/Attempt/decision/outbox concepts, reject the design and keep only the
smaller proved subset.

## Frozen acceptance tests

1. Two sibling repair commits for the same Work import successfully, retain distinct Attempt refs and
   remain independently resolvable.
2. `complete_run` closes the run with the exact canonical candidate without relying on a subject string;
   a non-coordinator or mismatched commit is rejected.
3. A worker judgement request ends its current Attempt as blocked. Resolution by the assigned lead
   reactivates the same Work at revision +1 and leaves exactly one pending worker wake.
4. Archive-native review fails a verifier that depends on `.git`, passes its portable repair, and does
   not dirty or move the canonical checkout.
5. Existing supervisor conformance remains green.
6. One fresh supervised GLM-5.3 run reaches a truthful terminal result without manual Git-ref repair,
   magic-subject recovery or idle-wake rescue.

## Interpretation

Passing deterministic tests proves only that the substrate no longer obstructs the intended behaviour.
The real model run decides whether the four primitives are sufficient in practice. Additional structure
is admitted only for a new observed failure that cannot be expressed by these primitives.
