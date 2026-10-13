# Sprint 72 — Who touched it, and members own their pages

**Status:** landed
**Programme:** owner cockpit, collaboration
**Depends on:** Sprint 71.

## Outcome

A document's or sheet's menu says who edited it last and when, as Notion's page menu does. A
member keeps a page of their own as the owner would: duplicate, rename, archive, restore and
delete. The multiplayer smoke now proves this. The phone heading gap left by Sprint 71 is closed.

## Observed friction

Read on 13 October 2026 from `main` at `217b4bb6`.

| Friction | Evidence |
|---|---|
| A page does not say who changed it last | the Library row says "Edited by"; the document and sheet pages did not, and a document's read carried no editor |
| Member page management was untested | the smoke covered sharing, comments and access but never a member duplicating, renaming, archiving, restoring or deleting a page |
| A sheet's Share panel on a phone had no level-one heading | Sprint 71 known limit: the panel covers the sheet and its heading |

## Tickets

- [x] **S72-T1 "Edited by … · when" in the page menu.**
  - A document's read carries `last_editor_actor_id`, worked out as the Library works it out:
    the people editing when the live body was saved, else whoever saved the version.
  - The document and sheet menus open with a quiet line naming them, and when.
  - `ActionMenu` gains a shared `menu-note` style.
- [x] **S72-T2 A member manages her own page.** A new multiplayer smoke step, run as Alice
  through the member gate:
  - She duplicates a company page into a copy she owns.
  - She renames, archives, restores, archives again and deletes the copy.
  - The owner can no longer open the copy, and the original is untouched.
- [x] **S72-T3 A heading while a panel covers the sheet.** The page adds a screen-reader heading
  outside the covered stage while a panel is open.

## Considered and set aside

**Typing indicators in group rooms.** They would ride the room's live stream. Routing them
through the shared database notification channel would make every open cockpit refetch on each
keystroke, so they need an in-process channel of their own. They are left for a sprint that
builds that channel.

## Results
