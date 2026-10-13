# Sprint 78 — Find a sheet by what is in it

**Status:** landed
**Programme:** owner cockpit, search
**Depends on:** Sprint 77.

## Outcome

⌘K finds a sheet by the words in its cells, not only its title, and says where they are:
"Fruit orders · C7 · Kiwi crates". Comments on sheets are found by their words alongside comments
on documents, and open the sheet on their thread. Only sheets the person may open are searched.

## Observed friction

| Friction | Evidence |
|---|---|
| A sheet could be found only by its title; a figure or a name in it could not | ⌘K matched sheet titles against the Library list in the browser |
| Comments on sheets (S74) were never found | comment search covered documents only |

## Design

- **Cell text** (migration 0098):
  - `native_sheets.search_text` holds one line per cell, "Tab!A1 content", as last saved.
  - Formulas are left out: their text is code, not words. Values typed as numbers are kept.
  - One SQL function, `native_sheet_text(workbook)`, computes it. It runs when a sheet is created
    and each time a change is accepted, which already carries the engine's workbook.
  - Existing sheets are backfilled from their newest checkpoint, or the workbook they began as,
    and catch up on their next change.
  - The column is not one of the Library's change-hint columns, so typing hints no one.
- **Search:**
  - `GET /companies/{c}/sheets/search?q=` searches the sheets the person can open (the Library's
    rule).
  - Every word must match, case-insensitively, with `%` and `_` taken literally.
  - It returns up to 8 sheets, each with its first matching tab, cell and text, and up to 6 sheet
    comments.
- **⌘K:**
  - A sheet found by content shows its cell and text, and is not listed twice when its title also
    matched.
  - Sheet comments join the Comments group: "You on Forecast FY27".

## Verification

- Unit tests: the matched line names its tab and cell, including tabs with spaces; wildcards are
  literal.
- DB test `sheet_search.rs`:
  - a two-word match names Q3 plan · A4;
  - a formula is not found;
  - a private sheet is not searched for someone not on it, but is for its owner;
  - `%` finds nothing;
  - a sheet comment is found;
  - once the sheet is private, neither its cells nor its comments are found by a member.
- Live on a `_test` company:
  - the migration backfilled existing sheets ("Ads" finds a sheet saved two sprints of testing
    ago);
  - a value written by a sheet operation is found at once;
  - ⌘K shows "Fruit orders · C7 · Kiwi crates".
- Every `restless-orgintel` test, `restless-owner` and `restless-engine` (see Results).
  svelte-check 0 errors and 0 warnings; web unit tests 131/131.

## Results

- Schema version 98.
- `restless-orgintel`: every test passes. `restless-owner`: 122/122.
- `restless-engine`: 389 passed and 5 failed in the full parallel run, in Telegram, finance, company bootstrap and the tool gateway. Those areas are untouched by this sprint. The 5 pass on their own and together (6/6), so they fail under full-suite load, not because of 0098.
