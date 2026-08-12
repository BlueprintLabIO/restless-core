# T14 · Crash and restart harness

**Layer:** Cross-cutting — this is the sprint's acceptance harness.
**Serves:** §15 items 5 and 6, and the §10.6 smoke-test scenarios for stalled workers and OrgIntel outage. Recovery claims are worthless unverified.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T11 (needs a real run to interrupt).

## Build

Rewrite the legacy black-box scenario driver against this sprint's CLI. **Keep the scenario shape and the supervision wrapper; replace the driver.**

Three interruptions, each asserted separately:

1. **Kill the Exec mid-turn** → restarts, rehydrates, continues the milestone (T4's acceptance).
2. **Kill a staff process mid-turn** → detected, worktree preserved, resumed or reassigned (T9's acceptance).
3. **Restart `restlessd`** → running companies resume against recoverable coordination state; already-produced files and commits remain valid (§4.8). Agents already running continue internal work.

Plus the terminal cleanup proof and disposable-DB supervision from the legacy harness — the most portable assets in it.

## Acceptance

All three interruptions pass on a real Cosmon run, not a fixture. **No committed work is lost in any case.**

## Salvage

Black-box golden scenario shape. **Re-validation:** the scenario shape ports; the driver does not. Confirm the cleanup proof still proves cleanup against containers that are now persistent by design.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
