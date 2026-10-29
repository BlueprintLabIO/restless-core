# Owner CPU and memory controls

30 October 2026 (Sydney). This is a bounded A0.3/A0.5 delivery; the full resource gate and Sprint 109 remain open.

## Account boundary

Resource pressure must slow or queue the owner's work, while identity, navigation, Inbox and Work stay usable. All companies in an account share its workload pool. Per-computer controls are additional ceilings within that pool; they do not increase account capacity.

Cloud already provisions one owner per VM (restless-cloud ADR 0006). Fleet Web/API, model relay and the provider are separate shared services. Existing workload placement and the aggregate CPU/memory pool protect the app, database and tunnel on each VM. This change retains those boundaries and the existing admission scheduler. No paid capacity change is included.

## Delivered behavior

The owner can edit CPU and RAM ceilings for the agent computer and browser computer in Resources, or restore the host defaults. Admins and members cannot read host resource details or change these caps. Values cannot exceed the operator's capacity, and zero cannot disable enforcement. Runtime RAM has a tested 768 MiB minimum, bounded by the operator's supplied maximum.

Desired values live in canonical company configuration. They survive daemon restart and container recreation, and refit down if the operator moves the company to a smaller host. Editing one target preserves the other; a stale edit is rejected. Saving does not start an absent computer.

The UI reports saved, effective and observed applied values separately. A live RAM reduction waits if current usage plus 64 MiB headroom does not fit. Existing maintenance retries it after usage falls. New managed work waits while a stricter Runtime RAM cap is pending. Docker enforces the eventual CPU/RAM ceiling, including zero workload swap. Actual measured usage can race a limit update; the account pool remains the outer containment boundary.

## Verification

Evidence is in `../evidence/sprint-109-resources/compute-controls/`:

- `limits.json`: real Runtime/browser edits, invalid values, stale edits, independent targets, daemon restart, container recreation, file retention, pending shrink and reset. Zero observed child OOMs. Exact daemon/image hashes are recorded; source was the worktree based on `34e4d952`, with this batch uncommitted.
- `shared-admission.json`: new work allowed, then waiting while a stricter cap was pending, then allowed after maintenance applied it. Clean fixture shutdown.
- `account-pressure.json`: four actual Restless jobs in one temporary account pool, narrowed to 768 MiB and half a CPU. 3,258 memory-high events, zero OOM kills, 4.902 CPU seconds over 9.742 seconds of busy work. All 128 health/identity/Inbox/Work requests succeeded: 64 on the pressured plane and 64 on a separate peer plane. Maximum response across both was 2,015 ms. The report's `ownerEntryChecks` is the combined count; `peerEntryChecks` is its peer subset.
- `pressure-pool-cleanup.json` and `peer-pool-cleanup.json`: limits restored, both fixtures removed, both temporary units removed. The pressured pool had zero max/OOM/kill events; the peer had zero memory-high events.
- `layout.json`: desktop 1440 px and phone 390 px; zero measured section movement for initial readings, pending changes and memory wait. Real editor screenshots retained. Screenshots precede the final account wording change; the account wording frontend build passed afterwards.

The two planes were isolated local fixtures without real model routes, not two newly provisioned Cloud accounts. Real owner/admin/member identity and coediting tests separately passed, including owner cap updates and 403 responses for both other roles. Engine cap tests (5), Runtime tests (47 passed, 4 ignored), workspace test compilation, and frontend checks passed.

The source-fingerprinted release smoke now includes the live cap API probe, followed by the normal-scheduler zero-idle-model-call probe. Full release qualification and production verification are still required before calling this batch deployed.

## Cleanup and remaining work

Disposable smoke Postgres containers now remove their anonymous data volume at teardown. Four old idle-test volumes were individually attributed to their test namespace before removal; no broad volume prune was used.

This batch does not provide editable aggregate account plans, storage/PID controls, historical charts or a priced catalogue. It does not establish production whole-task token savings, eliminate every potential service failure, or complete the full multi-owner Cloud release gate. Production Reno UI still needs an authenticated owner session for a final live walkthrough. Keep A0 open until its remaining measurements, AI budgets and equivalent-outcome checks pass.