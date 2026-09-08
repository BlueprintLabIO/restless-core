# Sprint 52 — Docs as company work

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 19
**Depends on:** Sprints 47–51

## Outcome

A human and agent collaborate around one real Doc linked to a Room and Work outcome: block/document
comments route focused Attention, an agent submits a reviewable revision against a named base, a human
accepts or rejects it, and the accepted version becomes searchable and attributable without silent
live overwrite.

## Scope

- Add document- and block-level comment Threads with revisions/tombstones, mentions and resolve/reopen.
- Route Doc/comment mentions to recipient-relative Attention with exact Doc/block, question, evidence,
  deadline/fallback and Work return path.
- Add review state and acceptance that creates a named version tied to accepting Actor, Work/Decision
  and material unresolved comments.
- Add agent tools for read, named-version read, comments, comment creation, revision proposal, version
  creation and export.
- Make agent edits proposals by default: whole-Doc or stable block scope, named base, summary, author,
  proposed JSON and proposed/accepted/rejected/stale/withdrawn state.
- Apply accepted proposals through the document service into Yjs, create a named version and fail stale
  or diverged bases visibly. Allow live editing only in explicit, visible `work_with` collaboration.
- Add rebuildable Postgres text search and typed backlinks across Rooms, Work, Decisions, Attention,
  Docs, Artifacts and Actors under the same access rules.
- Support side-by-side/block/plain-text comparison sufficient for human judgement; do not promise full
  character-level tracked changes.

## Acceptance

1. A block comment remains anchored while unrelated blocks change; a removed block yields a clear
   orphaned-anchor state rather than silently attaching elsewhere.
2. Mentioning an agent in a comment creates one Attention, wakes later if Runtime sleeps and returns a
   reply/result to the same Doc/Thread.
3. An agent proposal against the current named version is attributable and reviewable; acceptance
   updates live content and creates one named version under retry.
4. A changed base marks the proposal stale and never auto-merges or overwrites current rich text.
5. Live agent edit requires explicit `work_with`, remains visibly attributed and can be stopped or
   restored by a human.
6. Search/backlinks rebuild after projection deletion and never disclose restricted Doc existence.
7. The accepted Doc version can promote/link a Decision or existing Work responsibility without
   duplicating either lifecycle.

## Ticket outline

- [ ] C52-T0 — comments, anchors and mentions
- [ ] C52-T1 — review/acceptance and named-version semantics
- [ ] C52-T2 — agent read/proposal tool contract
- [ ] C52-T3 — stale detection, apply and visible work-with
- [ ] C52-T4 — search, backlinks and comparison
- [ ] C52-T5 — human-agent document outcome dogfood

## Deletion and exclusions

Delete silent agent live overwrites, fake tracked changes and second-write-path imports. Do not build
arbitrary text-range comments, semantic auto-merge, plugins/macros, public guest sharing, a universal
search database or agent cursors in every Doc.
