# Sprint 75 — Keyboard only, and what a screen reader hears

**Status:** landed
**Programme:** owner cockpit, accessibility
**Depends on:** Sprint 74.

## Outcome

A keyboard-only walk now reaches every control on every everyday screen:
- Inbox
- Library
- a document, and its comments
- a sheet, and its comments
- People
- Members
- Work

Focus is visible at every stop. Only the editors are the exception, where the caret is the cue,
as in Notion.

The sheet grid no longer keeps the keyboard. Menus and the command palette open and close by
keyboard and give focus back where it was. Chat messages say who wrote them, conversation titles
are headings, and counts read as words.

## Observed friction

| Friction | Evidence |
|---|---|
| Tab never left a sheet's grid, so its header, Share and the new comments panel were out of reach by keyboard | the walk stuck at the grid's cell on both sheet screens; Tab moves between cells, as in any spreadsheet |
| The Library breadcrumb read as an unnamed link when a side panel was open | its label is hidden in a narrow container |
| The Exec rail's conversation switcher showed no focus | its container clips overflow, and the ring was drawn outside |
| The document "…" menu opened from the keyboard but left focus on its button, and had no arrow keys | the walk: Enter opened it, focus stayed outside |
| Menus elsewhere opened by Enter or Space kept focus on the trigger | arrows worked, Enter did not move in |
| The sheet comment box showed no focus | its focus style used a variable that does not exist |
| Your own chat messages were read without an author | the name was `display: none` |
| The chat list read each room as "number sign SR room" | the `#` mark was text |
| A conversation's title was not a heading, so it could not be reached by heading navigation | the accessibility tree: `strong`, no heading |
| A count of one read "1 items" | "Company, 1 items" in the top bar |
| Closing the ⌘K palette restored focus while the dialog was still open and the page inert | focus came back only by the browser's own fallback |

## Changes

- **The sheet grid:**
  - Tab still moves between cells. Escape, then Tab or Shift+Tab, leaves for the next or previous
    control outside the grid.
  - The grid's cell is named "Cell". It describes how to leave and how to comment (Ctrl+Alt+M).
- **Menus:** opened by Enter or Space, a menu focuses its first item. The document "…" menu gains
  ArrowUp and ArrowDown, Home and End, and opens on ArrowDown. Escape returns focus to the trigger.
- **The palette** gives focus back once its dialog has closed.
- **Focus:** the rail switcher draws its ring inside; the sheet comment box has the cockpit's ring.
- **Names:**
  - the breadcrumb is "Back to the Library";
  - your own messages carry "You", hidden only visually;
  - the chat list's marks are hidden from screen readers;
  - the open conversation's name is an `h2`;
  - "1 item".

## Verification

- A keyboard walk with Playwright, tabbing every everyday screen from the top. For each stop it
  checks:
  - that it has a name;
  - that it is visible;
  - whether focus is shown, by comparing pixels with the stop focused and blurred;
  - whether focus gets stuck.

  Every screen starts at "Skip to content". The only stops with no ring are the document title,
  the document body and the sheet's cell, where the caret and the grid's own selection show focus.
- Escape-then-Tab from the grid reaches the comments panel's Close and then its comment box.
  Escape-then-Shift+Tab reaches Sheet actions.
- Menus by Enter: the document menu lands on its first item, and the profile menu on "Account".
  Escape returns to the trigger. The palette returns focus to "Document actions".
- Accessibility tree of the top bar, a room and a sheet's comments: every control is named, the
  room title is a heading, and every message names its author.
- svelte-check 0 errors and 0 warnings; web unit tests 127/127; the multiplayer smoke as below.

## Results

- `web/scripts/keyboard-walk.mjs` keeps the walk. Pointed at a `_test` company on a dev stack, it
  passes on all nine screens: Inbox 22 stops, Library 65, document 24, document comments 28,
  sheet 24, sheet comments 26, People 19, Members 39, Work 26.
- Multiplayer smoke: 22/22. The sheet-comments step now also checks that Tab moves between cells
  and that Escape, then Tab, leaves the grid.
- svelte-check 0 errors and 0 warnings; web unit tests 127/127.
- No migration; schema stays 97.
