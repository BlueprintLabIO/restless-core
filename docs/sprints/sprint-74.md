# Sprint 74 — Comments on sheets

**Status:** landed
**Programme:** owner cockpit, collaboration
**Depends on:** Sprint 73.

## Outcome

People comment on a sheet's cells as they do on a page:
- a thread on a cell, with replies and @mentions;
- edit and delete of one's own comments;
- resolve and reopen.

Commented cells carry a small corner mark in the grid. The sheet's Comments panel lists every
thread and shows a thread's cell when it is chosen. Mentions and replies reach For you and open the
sheet on the thread. Everyone with the sheet open sees a comment within seconds.

Sheets gain the comment level documents have, both for the company and for people added by name.
Someone who may comment reads the grid, cannot change it, and takes part in its threads.

This closes the largest collaboration gap in `docs/maturity-vs-notion.md`.

## Observed friction

| Friction | Evidence |
|---|---|
| A question about a figure had nowhere to go but chat, away from the cell | sheets had no comments; Notion database pages carry them, and Google Sheets comments on cells |
| Sharing a sheet offered view or edit, nothing between | documents have offered comment since S65; the sheet share panel said sheets "have no comments" |

## Design

- **Storage** (migration 0097):
  - `native_sheet_comment_threads`: a thread names its tab by the grid's id and its cell as A1.
  - `native_sheet_comments`: plain text, the actors it mentions, edited and deleted marks.
  - Both send the body-free `library_changed` hint (S70), so open sheets and For you refetch.
- **Anchor.** The cell address stays where it was written. A row inserted above does not carry the
  thread down, so the panel names the cell and shows it on demand. This matches comments on
  Notion database rows more closely than Google's moving anchors. It is the one known limit.
- **Access:**
  - Anyone who can open the sheet reads its threads.
  - Comment or edit access takes part.
  - Only the author edits or deletes a comment.
  - Anyone who may comment may resolve; a reply reopens a resolved thread.
  - A mention of someone who cannot open the sheet is refused, with the reason in a sentence.
- **Comment level.** `company_access`, participants, access requests and pending shares accept
  `comment`. Asks are ordered view < comment < edit. Granting at least what was asked answers an
  ask, and an open ask keeps the most it asked for.
- **For you.**
  - Items: `sheet-mention:` and `sheet-thread:`, mirroring documents.
  - Commenting on a sheet, or being mentioned in it, follows the sheet.
  - The target carries the thread, and the link is `&inspect=comments&thread=`.
- **Grid:**
  - "Comment" opens the panel ready to write. It is in the grid's own right-click menu, also for
    readers, and on Ctrl+Alt+M (⌘⌥M).
  - Corner marks are drawn on a layer over the canvas, beside the engine's cursor tags. The layer
    is redrawn on every grid update and never takes a click.

## Tickets

- [x] **S74-T1 Storage and rules:** migration 0097, `sheet_comments.rs`, the comment level for
  sheets.
- [x] **S74-T2 Routes:**
  - `GET/POST /sheets/{s}/comments`
  - `POST …/{thread}/replies`
  - `POST …/{thread}/resolve`
  - `PATCH/DELETE …/{thread}/{comment}`
  - A 422 refusal reads as a sentence, without the error's prefix.
- [x] **S74-T3 For you:** sheet mentions and replies, links to the thread, read when the panel shows
  them.
- [x] **S74-T4 The panel:**
  - Comments in the sheet header, with the open count.
  - A comment box on the selected cell, with @ people.
  - Open and Resolved.
  - Edit, delete with one confirmation, resolve with Undo.
  - "Ask to comment" for a reader.
- [x] **S74-T5 Sharing:** Can comment in the sheet share panel, for the company, for people and for
  invitees; asks to comment granted from it.
- [x] **S74-T6 Verify:**
  - DB tests `sheet_comments.rs`: threads, idempotent retry, mentions in For you, reply, resolve
    and reopen, author-only edit and delete, refused mention, reader and outsider. Asks across the
    three levels.
  - `general_access.rs` now walks the comment level for sheets.
  - The multiplayer smoke adds a step: Alice, at comment level, comments on A1 mentioning the owner.
    The owner's open sheet shows it within seconds and marks the cell. The For you item opens the
    thread. The owner's reply and resolve reach Alice live.

## Results

- Multiplayer smoke: 22/22 steps on two consecutive runs, including the new sheet-comments step.
- DB suites: every `restless-orgintel` test passes; `restless-owner` 122/122; the engine's Sheets
  integration test passes. `pending_shares.rs` now refuses an unknown level for sheets rather
  than `comment`.
- Web: svelte-check 0 errors and 0 warnings; unit tests 127/127. The general-access options are now
  one list for documents and sheets.
- Schema version 97.
