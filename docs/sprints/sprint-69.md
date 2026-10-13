# Sprint 69 — On a phone, and when the answer is no

**Status:** landed
**Programme:** owner cockpit, collaboration
**Depends on:** Sprint 68.

## Outcome

The everyday screens read cleanly at phone width, and a page that cannot be opened says so at
once, instead of spinning while it asks the server again and again.

## Observed friction

Read on 13 October 2026 from `main` at `666eba07`. A phone-width walk (390 × 844, touch) covered
the Inbox, Library, a document with its Share, Comments and History panels, a sheet and its
Share, an unavailable sheet and document, People and Members. It took a screenshot of each and
checked for anything wider than the screen; nothing was.

| Friction | Evidence |
|---|---|
| A document that cannot be opened never says so, and asks the server without end | on a 404, `purgeDeniedDocumentState` removed every query for the document, including the read that had just been refused. The page recreated it and asked again, forever, showing a skeleton. One open tab made tens of thousands of requests. |
| An invitee's row in Share hides that they are invited | the email truncated and took "(invited)" with it |
| A level select wraps at phone text size | "Can comment" broke onto two lines in a fixed 8.5rem column |
| The Comments sheet on a phone has a stray close button on its own row | the panel's own × repeated the sheet's back arrow |
| The Share search is cut off | "Add people by name or emai" |
| An unavailable sheet squeezes into a row on a phone | the compact empty state put its title in a narrow column and hid its explanation in a tooltip |
| A sheet's "Ask to edit" had no place on a phone | it sat in the header with no narrow fallback |

## Tickets

- [x] **S69-T1 A refusal shows.** The read that was refused is kept, with its error. Readable
  data hides what it held, so nothing leaks. Any query still in flight is skipped, for a document
  and for a company-wide denial alike. A regression test fails without the fix.
- [x] **S69-T2 Share at phone width.**
  - The level column fits its longest name.
  - "invited" stays beside a truncated address.
  - The search placeholder reads "Name or email".
- [x] **S69-T3 One way back.** Below 900px the Comments and History panel hides its own × as
  Share already hides its header.
- [x] **S69-T4 An unavailable sheet is a page, not a row,** with its explanation, "Ask for
  access" and the way back, as a document's is.
- [x] **S69-T5 "Ask to edit" on a phone** moves into the sheet's menu, as Follow does.

## Results
