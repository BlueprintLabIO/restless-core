# Maturity against Notion

A living assessment of the cockpit's collaboration against Notion, the owner's benchmark. It is
not a feature count: each area asks whether the everyday path is complete, polished and works for
every person in the company, members as well as the owner. Update it with each sprint.

**Read:** 13 October 2026, after Sprint 75.
**Evidence:**
- the three-person multiplayer smoke (`scripts/multiplayer-smoke`): 22 steps, owner and two
  members over network entry
- the DB suites
- an axe-core audit of ten screens at desktop and phone width
- a keyboard-only walk of nine everyday screens (`web/scripts/keyboard-walk.mjs`) and their
  accessibility trees
- a phone-width walk of the everyday screens

**Levels:**
- **At par:** a person moving from Notion would not miss anything on the everyday path.
- **Close:** the path works end to end; a named gap remains.
- **Gap:** a whole everyday path is missing.

## Areas

| Area | Level | Evidence | Remaining |
|---|---|---|---|
| Co-editing a page | At par | three people type in one document at once and every line survives reload; carets stay on each person's own line through splits (S67); named carets and faces (S65) | none on the everyday path |
| Comments, mentions, replies, reactions | At par | anchored threads, mentions reach only the named person, replies come back, reactions shared, edit and take back, reopen (S65, S66 wave 3, smoke) | none on the everyday path |
| Sharing and access | At par | general access (private, view, comment, edit) for documents and sheets; people by name; share with an invitee by email, given when they join (S66); ask for access on documents and sheets, granted from For you (S68); Copy link | link-only guest access to someone outside the company is out of scope by design (accounts are company members) |
| Inbox / notifications | At par | For you: mentions, replies, comments, changes, shares, asks; unread, archive with Undo, Archived view; desktop notices; email when away (S65–S66) | per-kind mute settings (waiting until asked for) |
| Live updates | At par | Library, For you, sharing and comments update within seconds for everyone (S68, S70); rooms live; typing indicators (S73) | none |
| Library and organisation | Close | list, kinds, review, Archived with restore and delete, pins on every device, Duplicate (S67), Edited by (S72), ⌘K search including comments | nested pages and a page tree (Notion's sidebar hierarchy) are not modelled: the Library is flat by kind |
| Sheets | At par | co-editing, sharing at view, comment or edit, history, CSV/Excel in and out, Duplicate, ask for access; comments on cells with mentions, replies, resolve, corner marks and For you (S74) | a thread stays on the cell address it was written on when rows are inserted above it |
| Chat (People rooms) | At par | group and direct rooms, threads, files and pictures, reactions, mentions, typing (S73), search | none on the everyday path |
| Version history | Close | named versions, restore, what changed since you last looked, who edited | a side-by-side diff view of two versions is not offered |
| Search (⌘K) | Close | documents by title and body, sheets, people, conversations, comments, recents | full-text search inside sheets |
| Members and access lifecycle | At par | invite (several at once), resend, role changes with Undo, leave with consequence stated, removal closes live sessions at once (smoke) | none |
| Phone width | Close | every everyday screen fits 390 px with no overflow (S69); panels become sheets | a native app is out of scope; touch editing of sheets is the engine's own |
| Accessibility | Close | axe: no findings of moderate or higher impact on ten screens at desktop; one main region, headings, named controls (S71, S72); every everyday screen walks by keyboard alone with visible focus, the sheet grid lets the keyboard out, menus and the palette return focus, and messages name their authors (S75) | a pass with a real screen reader (NVDA, VoiceOver) by a person; the accessibility tree was read, not listened to |
| Failure and edge states | At par | refusals say so at once (S69, S70); one toast with Undo, one confirmation for the irreversible; empty states lead somewhere (S68) | none known |

## Next, in order

1. **A page tree:** nesting documents under one another, the largest structural difference from
   Notion. It needs a design decision on how nesting meets general access.
2. **A version diff view.**
