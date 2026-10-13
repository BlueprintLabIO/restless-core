# Sprint 68 — Nothing a member does ends at a wall

**Status:** landed
**Programme:** owner cockpit, collaboration
**Depends on:** Sprint 67.

## Outcome

Every everyday path a member takes ends somewhere useful:
- A page shared with them appears as it happens, as it does for the owner.
- A sheet they cannot open can be asked for, as a document can.
- An empty Inbox still leads to what they archived.
- A colleague they type into Share who is not yet in the company is explained, not "nobody".
- A duplicate keeps its pictures.

## Observed friction

Read on 13 October 2026 from `main` at `78588231`, auditing every company API family the cockpit
calls against the member gate, then walking a member's empty and refused states.

| Friction | Evidence |
|---|---|
| A member's screens do not update live | the company change stream (`/changes`) was owner-only, and the layout opened it only for the owner; members waited for each list's poll |
| A private sheet's link is a dead end | "Sheet unavailable" offered only "Back to the Library"; documents have "Ask for access", sheets had nothing |
| A view-only sheet cannot be asked to edit | documents show "Ask to edit"; sheets did not |
| A member's empty Inbox hides Archived, and an empty Archived traps them | the member Inbox replaced For you, filter and all, with "Nothing waiting for you" whenever For you was empty, even when Archived or Unread was the empty view |
| A member typing a newcomer's email into Share is told "Nobody else by that name" | only admins may invite, and nothing said so |
| A duplicate made by someone else lost its images | images are shown where their uploader may edit; a copy made by someone else was not such a document (S67 known limit) |

## Tickets

- [x] **S68-T1 Live for everyone.** Members may open the change stream. Its hints name a kind of
  change and nothing else, and every refetch is answered only from what that person may read.
- [x] **S68-T2 Ask for a sheet.**
  - `native_sheet_access_requests` (migration 0095) adds request, list and dismiss in OrgIntel.
  - Granting closes the ask, and an ask set aside stays quiet for a day.
  - The ask reaches whoever may share the sheet in For you, and opens its sharing, where
    "Let them view / edit" and Dismiss sit above the people list.
  - "Ask for access" on an unavailable sheet; "Ask to edit" for a reader.
- [x] **S68-T3 Member Inbox.** The empty state applies only to "All" and offers Archived.
- [x] **S68-T4 Share explains invitations.** A non-admin typing an email hears that an admin can
  invite them from Members.
- [x] **S68-T5 Duplicates keep their images.**
  - `native_documents.duplicated_from` (migration 0095) records the original; only Duplicate
    sets it.
  - An image counts as placed by an editor when a document in that lineage shows it and its
    uploader may edit that document.
  - Pasting someone else's file id into a copy still shows nothing.
- [x] **S68-T6 Verify.**
  - DB suites: `sheet_access_requests`, `document_editor_basics` (duplicate images, including
    the pasted-id case), `document_copies`, `for_you` and `general_access`.
  - Member gate tests.
  - A new multiplayer smoke step: a member asks for a private sheet, and the owner grants it
    from For you.

## Results
