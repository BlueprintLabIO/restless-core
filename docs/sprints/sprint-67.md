# Sprint 67 — Members can do what owners can, and the smoke proves it

**Status:** landed
**Programme:** owner cockpit, collaboration
**Depends on:** Sprints 65 and 66.

## Outcome

Everyday collaboration works the same for a member as for the owner. Co-editing keeps each
person's caret on their own line. The three-person network smoke runs end to end on any
development host, with nothing changed on that host. Copying a document or sheet works as it
does in Notion.

## Observed friction

Read on 13 October 2026 from `main` at `c53700f8`, driving `scripts/multiplayer-smoke` end to end
for the first time on this host.

| Friction | Evidence |
|---|---|
| A member cannot attach a file to a message or put an image in a document | the member gate refused `POST /attachments` with "this membership may collaborate but may not perform owner operations", shown raw under the composer |
| A member cannot share their document with an invitee | `pending-shares` was missing from the member gate (S66 was verified only as the owner) |
| When a colleague adds a line below yours, your caret jumps into it | Tiptap UniqueID moves a block's id to the new block whenever the first part of a split is empty. The empty line's Yjs element then becomes the new line, and every caret anchored there follows it. Three people adding lines at the end of a page ended up typing into one paragraph. |
| Another person's caret label traps the browser caret | the label was an ordinary inline element, so End and clicks could place the browser's caret inside someone else's name |
| A reader sees the block grip | a view-only document showed the drag handle beside every block |
| The smoke could not run without editing `/etc/hosts` | `scripts/multiplayer-smoke` refused unless `plane.localhost` resolved through the OS |
| The smoke drifted from the product | renamed invite field, faces as buttons, 204 from marking read, the reader body's label, keys pressed faster than the browser reports the selection |
| No Duplicate | neither the Library nor a document or sheet offered a copy, an everyday Notion action |

## Tickets

- [x] **S67-T1 Member gate.**
  - Members may upload attachments. The file is kept under the person who added it, and reading
    it is decided where it is used.
  - Members may use pending shares; OrgIntel decides who may share.
  - Listing everyone's kept files stays owner-only.
  - The refusal now reads "Only the company's owner can do that."
- [x] **S67-T2 Co-editing keeps everyone where they are.**
  - Block identity: a split block keeps its id on the part that is still that block (the part
    with content, or the first when both or neither have any). UniqueID keeps the attribute and
    fills ids when a document opens; its transaction plugin is replaced.
  - Unit-tested for Enter at the end, at the start, and on an empty line. Probed with three
    browsers adding lines one after another and at once: every line lands where its author typed
    it.
  - Remote carets are non-editable, with word joiners either side, as Tiptap's own are. Readers
    see no block grip.
- [x] **S67-T3 Smoke on any host.**
  - Where `plane.localhost` does not resolve, `scripts/multiplayer-smoke` re-runs itself in private
    user and mount namespaces with its own hosts file.
  - The inner namespace maps back to the caller's uid, because Core hands its uid to the
    Documents container.
  - The journey follows the current UI. A new step checks that a document shared with Bob's
    address before he joined is his when he joins, and only his.
- [x] **S67-T4 Duplicate.**
  - Library rows, the document menu and the sheet menu offer Duplicate. It makes a copy the person
    owns, with the body as it reads now and the same general access, but not its people,
    comments or history.
  - The copy opens with "Copy of …" selected, ready to rename.
- [x] **S67-T5 Correct the S66 deploy note.** Core grants the sidecar its functions whenever it
  opens a company, so precise edit attribution starts on the first start after release.

## Known limits

An image in a duplicated document shows to people who can open a document that its uploader can
edit. Someone who can open only the copy, made by someone else, sees it missing. Widening the rule
would let anyone who knows an attachment id read it, so the limit stays.

## Results
