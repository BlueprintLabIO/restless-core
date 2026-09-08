# Sprint 49 — Durable Actor context and session continuity

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 18
**Depends on:** Sprints 45–48

## Outcome

A persistent Exec or lead can be stopped, moved to a fresh model session and resumed from durable
identity, current responsibility, a compact source-linked checkpoint and the triggering Room/Work
context without replaying the whole company or losing material progress.

## Scope

- Replace the giant centrally assembled prompt path with a minimal versioned bootstrap envelope and
  explicit retrieval tools. Preserve stable Actor identity, current organisational slice, current Work,
  scoped Room/Doc context, session memory and archive as distinct layers.
- Make every focused wake state who the Actor is, what matters, success evidence, material change,
  available tools/skills/resources and the exact return path.
- Prefer stable IDs, links, backlinks, current summaries, full-text search, file paths and Git refs.
  Load full histories, schemas, skills and artifacts only when requested.
- Add durable source-linked Actor checkpoints before graceful compaction, transfer or termination and
  use them for fresh-session recovery.
- Enforce one active primary session per persistent Actor. Queue extra inputs in its inbox; represent
  explicit parallel work as delegated Attempts/temporary workers with separate attribution.
- Preserve organisational altitude: context expands downward; accepted outcomes, decisions, risks and
  implications compress upward.
- Retain epistemic labels, provenance, staleness and untrusted-source boundaries through context
  compression. Context never conveys Authority.
- Instrument context size/cost, reconstruction time, stale-context errors, restart loss, handoff
  first-pass success and high-level Actor time spent in low-level detail.

## Acceptance

1. A persistent Exec restarts under a different admitted model and retains role, priorities, current
   Work, decisions and accepted style without provider transcript replay.
2. A mention in a long Room sends the trigger Thread, Room anchors, linked Work and key sources—not full
   history—and the Actor can retrieve one older source on demand.
3. A coding Actor checkpoints, loses its process/session ID, resumes the same worktree and does not
   repeat already evidenced work.
4. Two simultaneous `@exec` mentions enter one inbox and cannot launch contradictory sovereign primary
   sessions; an explicit child research Attempt remains separately attributable.
5. A stale local summary conflicts with a newer Decision; Core refreshes only the affected branch and
   current authority wins.
6. An independent critic receives objective, rubric, constraints and artifact without producer
   reasoning, then returns source-linked evidence.
7. Raw credentials and external untrusted instructions never enter the trusted identity/directive
   portion of the packet.

## Ticket outline

- [ ] C49-T0 — current context-path inventory and minimal envelope
- [ ] C49-T1 — scoped retrieval and Room/Doc anchors
- [ ] C49-T2 — checkpoints, compaction and fresh-session resume
- [ ] C49-T3 — single-primary-session and queued inbox semantics
- [ ] C49-T4 — provenance, staleness and untrusted-content boundaries
- [ ] C49-T5 — comparative context/restart dogfood

## Deletion and exclusions

Delete full-company context injection, standing transcript dependence, repeated summary-of-summary
copies and duplicated actor-memory truth. Do not require a vector database, universal embedding
pipeline, global context synchronizer, provider-specific permanent session or central reasoning model.
