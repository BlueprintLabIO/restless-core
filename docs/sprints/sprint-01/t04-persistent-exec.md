# T4 · Persistent Exec identity + file-based continuity

**Layer:** Runtime + OrgIntel — the process is runtime, the identity and continuity are OrgIntel.
**Serves:** Greenfield #7. The legacy Exec cold-starts every turn; this is the difference between a harness and a company.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T1, T3, T5.

## Build

- `/company/org/exec/` holds `mission.md`, `current-plan.md`, and `journal/NNNN.md`.
- **The Exec identity is durable; the ACP session is disposable.** Identity is an actor row plus that directory. The model session is recreated on each wake and rehydrated — it is not the thing that persists.
- Rehydration on wake: mission + current plan + recent journal + open commitments + inbox.

## Acceptance

Kill the Exec mid-turn. It restarts and **continues the milestone rather than restarting it.** Asserted concretely:

- no duplicate plan is written to `current-plan.md`;
- the journal continues rather than beginning again;
- open commitments are not reset to `proposed`;
- committed work in `/company/repos` is intact.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
