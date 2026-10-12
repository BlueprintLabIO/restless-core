# Sprint 65 — Working together feels like a mature product

**Status:** landed; the S65-T7 browser run waits on `plane.localhost` resolving on the dev host
**Programme:** owner cockpit, collaboration
**Depends on:** Sprint 64 (Library, everyday files), the members-first-class and people-own-teams
work (`bf46b9c9`, `0eadcc8a`), native document collaboration (0053, 0061).

## Outcome

People who share a company work together as easily as they do in Notion. You hear about the
things that need you and nothing else; you can see who is on a page with you; you know what changed
since you last looked; sharing says plainly who can do what; and the editor has the everyday
affordances people reach for without thinking. Exec and Staff take part in all of it the same way
people do.

The benchmark is completeness and polish, not feature count: every path a person takes in a normal
day ends somewhere sensible, says what happened, and never strands them.

## Observed friction

Read on 13 October 2026 from `main` at `347c9d6e`, against Notion as the owner's benchmark.

| Friction | Evidence |
|---|---|
| The owner never hears about mentions or replies | the owner Inbox (`attention.rs`) carries handoffs and approvals only; mentions reach members (`MemberInbox`, `attention.mentions`) and hosted email (`notifications.rs`), never the owner's cockpit |
| Replies and comments on your pages go unseen | nothing projects "someone replied in a thread you are in" or "someone commented on your document" |
| No way to follow a page or see what changed | no follow concept; named versions record an author but nothing turns them into "what changed since you looked" |
| No sense of who is on a page | presence is company-wide (`presence.svelte.ts`); carets show only while someone types (`DocumentEditor`) |
| Sharing is only per-participant | `visibility` is `company` or `participants`; company documents give every person edit with no "can comment / can view" choice; no share-by-link affordance beyond copying the URL |
| Editor lacks everyday affordances | no `/` command menu, no block handle to move blocks, no images in documents, no reactions on comments |

## Design stance

**Core contract**

- **For you is a projection, not an outbox.** It is assembled from the durable objects that already
  exist: mentions (document and room), comment threads and replies, owner handoffs, and named
  versions. The only new state is what each person has read and which documents they follow.
- **Read state is per person and per item,** with "mark all as read" as a cursor. Opening the item
  marks it read on every device.
- **Following is implicit first.** Creating, editing, commenting on or being mentioned in a document
  follows it; anyone can follow or unfollow explicitly.
- **General access is one choice per document and sheet:** private, company can view, can comment,
  or can edit. Explicit participants still win. The company owner and the Exec can always manage
  sharing (`c9bd0b6f`).
- **Presence on a page is ephemeral** and comes from the live collaboration session's awareness;
  nothing is stored.

**Product hypothesis**

- One "For you" list with unread dots and a badge is enough; per-kind notification settings wait
  until someone asks to mute something.
- Faces on a page plus named carets are what make co-editing feel alive.

## Tickets

- [x] **S65-T1 For you** (OrgIntel + owner gateway + cockpit). Mentions, replies in threads you are in,
  comments on documents you follow, and changes to documents you follow, newest first, with unread
  dots, mark read and mark all read, for the owner and for members. An unread count on the Inbox
  tab and a desktop notice when it rises. Exec and Staff mentions of people included.
- [x] **S65-T2 Follow and what changed** (OrgIntel + cockpit). Implicit follows; a Follow toggle in the
  document header; a document's changes since you last opened it, with who made them.
- [x] **S65-T3 Faces on the page** (documents sidecar awareness + cockpit). Who is viewing or editing
  this document or sheet, at the top of the page, with their caret colour.
- [x] **S65-T4 General access** (OrgIntel + owner gateway + cockpit). The share panel says who can
  do what in one place: general access, people with explicit access, invite a member by name, copy
  link. Sheets the same.
- [x] **S65-T5 Editor basics** (cockpit + documents sidecar + OrgIntel validation). `/` command menu,
  block handle to drag blocks, images in documents (paste, drop or pick; stored as company files),
  reactions on comments.
- [x] **S65-T6 Completeness sweep** (all surfaces). Walk every everyday path for owner and member at
  desktop and phone width; fix dead ends, silent failures, missing confirmations, loading and empty
  states, keyboard paths and copy, removing anything that does not earn its place.
- [ ] **S65-T7 Verify** (evaluation). Two browsers, owner and member, on a `_test` company: mention,
  reply, follow, co-edit with faces, change access, insert an image, react; each reaches the other
  as it would in Notion.

## What this makes deletable

- `MemberInbox`'s separate mention list, folded into For you.
- The per-device "seen" logic still left in the rail once read state lives on the server.

## Results

Landed on `main` in three waves after `347c9d6e`; schema version 92 (migrations 0086–0092).

- **T1 For you.** One list for the owner and members (`8cd540bf`, `d3e269e9`, `d7c224d1`):
  mentions, replies, comments, changes, shares, review asks, Room threads, sheets and access asks
  (`650063a4`, `18f5b945`). Read, unread and archive marks are per person and follow them to every
  device (`a8c49fa7`); Undo on archive restores the item exactly (`697023b8`); a mention still
  unread after a while is emailed to hosted people (`650063a4`); desktop notices say when the
  browser blocks them (`64b123f0`). A mention opens its exact thread.
- **T2 Follow and what changed.** Implicit follows, a Follow toggle, and the changes to a document
  since you last opened it, with who made them (`6c6e0aea`, `bb106bf5`, `fdd6c343`). Sheets follow
  the same way (`5c0e3fde`).
- **T3 Faces.** Who has a document or sheet open, Exec and Staff included, with caret colours
  (`02146f40`, `56a5ab6a`, `5dc79580`, `3f6102a7`); a face jumps to that caret (`b5d166fc`).
- **T4 General access.** Private, or the company can view, comment or edit, for documents and
  sheets (`f591a952`, `4c9f5976`, `121243b7`); "(you)", who granted access, arrow keys and Undo on
  remove (`40b02cd0`, `64460bf5`); ask for access, and grant it from For you (`e7bb8c7b`,
  `19770085`). Typing the email of someone already in the company shares with them instead of
  inviting them again.
- **T5 Editor basics.** "/" menu anywhere, block handle, images in documents, reactions on
  comments, formatting over a selection, comments marked in the text, editing and taking back
  comments, reopening threads (`536b817e`, `db4205d6`, `9f0475a4`, `e7bb8c7b`).
- **T6 Completeness.** One toast with Undo for anything that can be taken back, and one
  confirmation dialog only for what cannot (`97918fcd`, `9ccb5eac`). People can leave a company,
  and the dialog says what that means (`4cc414f1`, `2003476e`). Role changes have Undo; admins can
  invite several people at once and resend invitations. Library rows offer Archived, Copy link and
  Share…, and show who edited last; pins follow you to every device (`759d6296`, `2309dc25`).
  Members get the same ⌘K, which finds conversations and comments (`a5dc005a`). Files can be
  attached in every room (`697be200`), and conversation messages show their actions as one row of
  icons (`45925def`).
- **T7 Verify.** `scripts/multiplayer-smoke` drives the owner and two members through a mention, a
  reply, a reaction, view-only access, the "/" menu and a picture in a group room. The DB suites
  behind it pass locally. The browser run needs `127.0.0.1 plane.localhost` in the dev host's
  `/etc/hosts`.

**Still open:** an Archived tab in For you; showing a pending share to someone invited but not yet
joined; live-edit attribution beyond named versions.
