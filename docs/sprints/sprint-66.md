# Sprint 66 — Nothing waits on a workaround

**Status:** landed
**Programme:** owner cockpit, collaboration
**Depends on:** Sprint 65 (For you, general access, faces, editor basics).

## Outcome

The three gaps Sprint 65 left open are closed the way Notion closes them. You can find what you
archived. Sharing with someone who is invited but not yet a member works, and they find it waiting
when they join. And "who changed this" names the people who changed it, not everyone who had it
open with edit access.

## Observed friction

Read on 13 October 2026 from `main` at `f70d989a`.

| Friction | Evidence |
|---|---|
| Archived For you items cannot be found again | `for_you.rs` only filtered archived items out; nothing listed them, so Undo was the only way back |
| Inviting someone from Share shares nothing | `invitedFromShare` told the sharer to "share this with them here once they join"; nothing remembered the intent |
| Live edits are credited to anyone who could write | a checkpoint named everyone holding a write session since the previous version, including people who only read with edit access |

## Design stance

- **Archived is a view of For you, not a new store.** The same per-person marks decide it: an item
  is archived when this person archived it and nothing has happened there since.
- **A pending share is the sharer's intent, kept under the invitee's email.** It waits under the
  address the invitation holds. When that person first enters, it is given through the ordinary
  participant path, as the person who shared it. That means the same permission checks, the same
  events, and a "shared with you" item in their For you. Core learns the address only from the
  issuer's signed entry, and uses it for nothing else.
- **Who edited is who the sidecar stored edits from.** The documents sidecar notes the writers
  whose changes reached it. Core accepts a note only for someone holding a write session on that
  document. When a sidecar notes nobody, the earlier rule stays: everyone who held a write session.

## Tickets

- [x] **S66-T1 Archived in For you** (OrgIntel + owner gateway + cockpit). `?archived=true` lists
  what this person archived; "Move back" with Undo; news there brings an item back by itself.
- [x] **S66-T2 Share with an invitee** (identity + engine + OrgIntel + owner gateway + cockpit).
  - Migration 0093 adds `pending_shares`.
  - The entry assertion carries an optional verified `email` (ADR 0009 amended).
  - `apply_pending_shares` runs on entry.
  - Share lists invitees as "(invited)", with their level, a level change and Stop with Undo.
- [x] **S66-T3 Who actually edited** (documents sidecar + OrgIntel + engine grant). Migration 0094
  adds `native_document_live_editors` and `orgintel_native_document_note_editors`. The sidecar
  notes writers after each successful store, and checkpoints name them.
- [x] **S66-T4 Verify.**
  - DB suites: `for_you`, `pending_shares`, `documents`, `native_document_live_content`,
    `document_completeness`, `general_access`, `access`, the Library suites, `people_own_teams`
    and `room_attachments`.
  - The sidecar boundary test checks that only the writer is noted.
  - Local smoke: a pending invitee in Share; level change; Stop, then Undo.

## Deploy order

Core must ship before an identity release that sends `email`, because Core refuses unknown claims.
Core grants the sidecar its new function whenever it opens a company, so existing companies pick
it up on their first start after the release. Until then the sidecar's notes are refused quietly,
and attribution falls back to write sessions.

## Results

Landed on `main`; schema version 94.

A clear Inbox keeps "Archived" beside "Set aside" in its corner, so archived items stay
reachable when nothing is current.
