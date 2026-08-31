# Sprint 26 run report — exact unattended substrate

**Result:** Pass

**Closed:** 30 August 2026

**Activation authority:** This report and the adjacent `activation-receipt.json` are the Sprint 26
handoff to EXP-16. The receipt is authoritative for machine coordinates; this report explains what
was proved and what was deliberately retained.

**Post-activation amendment:** EXP-16 exposed a GNU `setsid` wait defect that the original fixture did
not discriminate reliably. [`amendment-001-gate-session-wait.md`](amendment-001-gate-session-wait.md)
records the bounded Runtime repair, migration-byte restoration and successful live requalification;
[`activation-amendment-001.json`](activation-amendment-001.json) is its machine receipt. Counted gates
after that observation require the amended launch semantics.

## Outcome

The integrated EXP-15 failure cluster now runs without a model or operator repairing execution
lineage, workspaces, resources, gates, feedback delivery, supervisory wakes, promotion, review
publication, or restart cleanup. The final adversarial fixture passed in 11.85 seconds and left no
scratch schema, repository, worktree, Attempt runtime, lease marker, port holder, or child process.

This is a substrate result, not a claim that product work itself is correct. The substrate now makes
the candidate, evidence, failures, and interventions exact enough for an unattended product campaign
to be judged honestly.

## Exact verification

The closing run used base Git commit `fcc59659073a802ea24f78a394ecbf1517e58819` and Runtime source
digest `36a9319a6285a44e67190702e381bd068af95af8aa8dfffc5bb57f038d75156a`. The source digest is the
canonical content coordinate because the shared development tree contained authorised work from
concurrent sprints; it must not be mistaken for the clean Git tree at the base commit. Binary, image,
migration, and gate fingerprints are frozen in `activation-receipt.json`.

The following checks passed from the same source state:

- `cargo test --workspace --all-targets`: every ordinary workspace test passed; the six ignored tests
  are explicitly live, isolated tests and did not run implicitly.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check` over the Sprint 26 implementation and programme specifications: passed.
- `exact_execution`: two OrgIntel integration tests passed.
- `live_integrated_gate_leasing_cache_failure_and_restart_cleanup`: one ignored-by-default live test
  passed against the isolated `swift_arrival_npc_test` container; 226 unrelated daemon tests were
  filtered from that focused invocation.

The three visible `exit status 1` lines in the live fixture are expected evidence from its deliberately
failing engine-error, process-leak, and timeout gates. The test passed only because each was classified
as a failure and cleaned up.

## Integrated fixture evidence

One deterministic fixture proved the tickets together:

| Property | Closing evidence |
| --- | --- |
| Exact source | A pre-existing wrong-source worktree was refused before model execution. A symbolic `main` ref was frozen to its resolved commit and remained there after `main` moved. |
| Hermetic workspace | Root/mixed-owned source was normalised to uid/gid 2000. Godot cache state lived behind an external `.godot` symlink and left Git clean. A peer Attempt had a distinct worktree and cache. |
| Resource leases | Three concurrent callers for one exact gate launched one process. Two different concurrent gates received different ports. Every lease was released. |
| Gate truth | The exact gate ran over the Attempt worktree. A zero-exit engine log containing `ERROR`, a leaked child, and a timeout each failed. The leaked process group was killed. |
| Restart recovery | A boot-style orphan process group was discovered and reaped before scheduling. Runtime boot calls the staff orphan sweep before normal actor work begins. |
| Feedback | Ordinary feedback was delivered once to the same Attempt at a safe checkpoint and did not supersede productive work. |
| Supervision | 100 linked nonterminal progress artifacts produced zero lead wakes. Two terminal facts for one lead coalesced to one wake. Re-flushing was idempotent. One genuine blocker produced one prompt wake. |
| Promotion | A pending journal moved neither branch nor artifact alias. A committed journal fast-forwarded the exact branch, kept it clean, and published a content-addressed review target. Reusing that target for different content was refused. |
| Cleanup | Candidate checkpoint state survived while transient runtime state was removed. Post-run inspection found zero fixture schemas, repositories, worktrees, Attempt directories, marker files, `sleep 300` children, or process groups. |

The runtime portion launched seven governed gate processes across the distinct exact keys and negative
cases. The three same-key concurrent requests accounted for only one of those processes. Model turns,
lead wakes, and manual mid-run repairs inside the runtime fixture were all zero.

## Defects found before acceptance

The expanded fixture found two real defects before the final pass:

1. Ignoring `.godot/` did not ignore the `.godot` symlink itself. The workspace now excludes `.godot`
   and the test proves both clean Git state and external cache placement.
2. The deliberately wrong-source pre-existing Attempt was refused correctly but its transient runtime
   directory survived fixture teardown. Cleanup now includes refused Attempts and the post-run audit
   proves no residue.

These were fixed before this report. They are evidence that the integrated test was adversarial rather
than ceremonial.

## Measured closure

- Model turns required by the integrated runtime fixture: **0**
- Lead wakes caused by 100 progress artifacts: **0**
- Coalesced wake for two terminal facts: **1**
- Prompt wake for one genuine blocker: **1**
- Same-key gate callers / actual executions: **3 / 1**
- Manual repairs during the final fixture: **0**
- Retained fixture junk after cleanup: **0**
- Raw captures produced by Sprint 26: **0**

The test controller performs deterministic setup, assertions, fault injection, and teardown. Those are
the test itself, not operator intervention in a running campaign.

## Deletion record

The accepted path no longer depends on:

- default-to-`main` or prose-parsed mutable launch semantics;
- shared mutable actor worktrees or repository-local engine caches;
- guessed fixed ports, shared display assumptions, or pid-file folklore;
- model-reenacted gate sequences or success inferred from exit code alone;
- automatic Attempt supersession for ordinary feedback;
- per-progress supervisory wakes;
- model-mediated branch/artifact promotion; or
- mutable human-readable review directories.

The following boundaries remain deliberately:

- first-volume bootstrap ownership repair in `infra/company-image/entrypoint.sh` and browser/run
  ownership repair at boot;
- Runtime-owned normalisation of an exact repository and Attempt workspace to the company uid/gid;
- superseded and abandoned historical states, because recorded coordination history must remain
  readable;
- timeouts as a terminal safety envelope, never as evidence of success; and
- historical experiment documents and repair scripts as evidence, not as the accepted execution path.

## Residual risks and disposition

- The activation coordinate is a content digest over the Runtime build source rather than a clean Git
  commit because other authorised work shares this development tree. EXP-16 pins the digest and image
  ID; any semantic change forces reconciliation and requalification.
- The live fixture uses one Linux company container on this host. Cross-host and non-Linux transport
  differences remain outside Sprint 26 and must not be inferred as covered.
- The fixture proves exact mechanics, not provider availability, model quality, game quality, or human
  fun. EXP-16 owns those later questions.
- Host storage fell below the 30 GiB safety floor before image reconciliation. The documented bounded
  cleanup removed only regenerable Rust artifacts, recovered 53 GiB free, and preserved all company
  volumes and campaign state.

## Decision

Sprint 26 passes. EXP-16 may activate only with the exact adjacent receipt, the final frozen EXP-15
candidate, admitted Sol/Terra routes, and its declared spend ceiling. A later change to execution,
workspace, lease, gate, feedback, wake, promotion, or review semantics invalidates this receipt until
the focused qualification is repeated.
