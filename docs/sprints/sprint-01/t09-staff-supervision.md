# T9 · Staff spawn and supervision

**Layer:** OrgIntel + Runtime — the decision to spawn is OrgIntel, the process is Runtime.
**Serves:** Multi-agent friction — handoff, crash recovery, duplicate work — is where sprint 02's OrgIntel design comes from. A single-agent sprint would leave that evidence unobtained.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T1, T3, T5.

## Build

- The Exec requests a spawn; `restlessd` starts an ACP process in the company container.
- **A dedicated Git worktree per code-producing staff member** (§5.4 rule 1, §9.7) — not one shared mutable working tree.
- Health from process liveness plus last-activity.
- **Cap of two per company.** Enough to produce handoff and crash friction without tripling token burn across three companies.
- Crash → detected → worktree preserved → work resumed or reassigned. Useful files and commits are never discarded.

## Frame 2 note — read before implementing

*"Is this staff member stalled, or just thinking?"* is **judgement + enumerable**: a finite output over an open-ended input. It gets a model call, not a timeout threshold. A stall detector built from an elapsed-time constant is exactly the misread LLM_CURE.md frame 2 describes — finiteness read off the output rather than the input.

Process liveness *is* deterministic and stays deterministic. Do not confuse the two signals.

## Acceptance

Kill a staff process mid-turn. The crash is detected, its worktree survives intact, and the work is resumed or reassigned without discarding commits.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
