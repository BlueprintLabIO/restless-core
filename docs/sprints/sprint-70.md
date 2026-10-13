# Sprint 70 — Live means live

**Status:** landed
**Programme:** owner cockpit, collaboration
**Depends on:** Sprints 68 and 69.

## Outcome

When a colleague does any of the following, everyone's open cockpit shows it within a second or
two, not at the next 15-second poll:
- makes, renames or archives a document or sheet
- shares one
- comments on one
- asks for access

A document a member cannot open tells them so at once.

## Observed friction

Read on 13 October 2026 from `main` at `5834b065`. The multiplayer smoke gained a step: a member's
open Library must show a page the owner just made, well inside the Library's 15-second poll.
Production showed a member a refused document.

| Friction | Evidence |
|---|---|
| Nothing in the Library or For you updated live, for anyone | change hints came only from database triggers on Work, handoffs, schedules, rooms and mentions. Documents, sheets, sharing, comments and asks had none, so a new page took about 12 s to appear even for the owner (measured by one tab watching while another created). Sprint 68 opened the stream to members, but the stream was silent. |
| A member shown a refused document waited on a skeleton forever (found in production, after S69 ended the request storm) | the cache purge on refusal could still remove a read that a page was watching. The page was left waiting on a read that no longer existed. |
| A new migration alone did not reach a locally built daemon | `sqlx::migrate!` embeds migrations at compile time. With no build script, adding a migration file did not rebuild `restless-orgintel`. CI builds from clean, so releases were not affected. |

## Tickets

- [x] **S70-T1 Library change hints (migration 0096).**
  - A body-free `library_changed` hint fires on documents, sheets, participants, comment threads,
    comments, comment reactions, access requests (documents and sheets) and pending shares.
  - Typing rewrites document and sheet rows constantly, so those rows hint only when what a list
    shows changes.
  - Personal marks stay out.
  - The scheduler ignores the kind. Identical hints in one transaction arrive once.
- [x] **S70-T2 A refusal always shows.** The purge on refusal removes only reads nothing on screen
  is watching. A watched read meets the refusal itself, and readable data hides what it held.
- [x] **S70-T3 `restless-orgintel/build.rs`** reruns the build when migrations change.
- [x] **S70-T4 Verify.** The multiplayer smoke adds two steps, both passing:
  - A page made for the company reaches a member's open Library within 6 s.
  - A missing document says so to a member within 15 s, with at most two reads.

## Results
