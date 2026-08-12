# T4 · Persistent Exec identity + file-based continuity

**Layer:** Runtime + OrgIntel — the process is runtime, the identity and continuity are OrgIntel.
**Serves:** Greenfield #7. The legacy Exec cold-starts every turn; this is the difference between a harness and a company.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T1, T3, T5.

## Build

- `/company/org/exec/` holds `mission.md`, `current-plan.md`, and `journal/NNNN.md`.
- **The Exec identity is durable; the ACP session is disposable.** Identity is an actor row plus that directory. The model session is recreated on each wake and rehydrated — it is not the thing that persists.
- Rehydration on wake: mission + current plan + recent journal + open commitments + inbox.

## Termination — the Exec must be able to stop

Nothing else in this sprint defines when a milestone ends. T6 wakes the Exec, T4 keeps it going, and without a stopping condition an autonomous Exec runs until the T2 spend fuse trips — **which turns the fuse into the design rather than the safety net.** That is not acceptable; a fuse should never be load-bearing.

At the end of each turn the Exec resolves one of:

- **continue** — more work to do, schedule the next wake;
- **blocked on owner** — a genuine judgement, authority or ambiguity point; surface it and stop (§4.6, "owner attention reserved for genuine judgement");
- **done** — the milestone's stated outcome is met;
- **abandon** — the milestone is not worth continuing; say why.

**This is judgement + enumerable** (LLM_CURE.md frame 2): a finite output over an open-ended input. It is a model call. **It must not be a turn-count cap, an elapsed-time limit, or a keyword check on the last message** — those are the misread frame 2 describes, and they will terminate good runs and continue bad ones.

Turn count and elapsed time may be *reported* to the Exec as context for its judgement. They may not *make* the decision.

## Acceptance

Kill the Exec mid-turn. It restarts and **continues the milestone rather than restarting it.** Asserted concretely:

- no duplicate plan is written to `current-plan.md`;
- the journal continues rather than beginning again;
- open commitments are not reset to `proposed`;
- committed work in `/company/repos` is intact.

Separately, on a normal run: the Exec reaches **done** or **blocked on owner** on its own, without the spend ceiling being involved. A run that only ever stops because it ran out of money has failed this ticket even if the artifact is fine.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
