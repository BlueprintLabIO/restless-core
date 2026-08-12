# T5 · OrgIntel core — actors, goals, commitments, messages, artifact refs

**Layer:** OrgIntel — recoverable coordination state, explicitly outside the constitutional trust boundary (§4.9).
**Serves:** Three companies × multiple actors × messages × schedules is the point at which files stop being the cheaper option. Named because an earlier draft of this sprint had no database at all; the three-company shape is what changed the answer.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** Nothing structural. Can proceed in parallel with T1/T3.

## Build

Postgres, one schema per company, sqlx + migrations. The deliberately small §4.4 ontology:

`actors`, `goals`, `commitments`, `messages`, `artifact_refs`, `decisions`, `events`.

- **Commitment states are a real enum** — proposed, active, blocked, completed, abandoned. Deterministic and enumerable, which is the one quadrant where a state machine is correct (LLM_CURE.md frame 2). This is not a licence for more state machines elsewhere.
- `artifact_refs` are **references only**: path, repo+commit, worktree+branch, or URL. No export/import/materialise/reattach, no custody state machine (§6.3).
- `events` is an operational stream for UI, debugging and awareness. **It is not a ledger** — it may be compacted, repaired or regenerated (§4.4).

## Guard on this ticket

This is the ticket most likely to over-model. **Any table no company writes to during the sprint does not survive the deletion pass.** Do not add an entity because the architecture document mentions it; add it because a run needed it.

## Acceptance

The Exec and staff coordinate a full Cosmon run through this schema, and the CLI can render goals, commitments and inboxes from it.

## Salvage

Directed messaging concept from `communication.rs`. **Re-validation:** strip the universal-command envelope; validate against real Exec↔staff traffic.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
