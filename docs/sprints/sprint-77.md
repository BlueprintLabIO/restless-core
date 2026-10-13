# Sprint 77 — Compare versions, and re-import a sheet

**Status:** landed
**Programme:** owner cockpit, history
**Depends on:** Sprint 76.

## Outcome

- **Documents.** History compares the version you preview with any older version, not only the
  one before it. "Compare with" lists them, and the default stays the version before.
- **Sheets.** History says what a saved version changed, cell by cell, against the version before
  it:
  - the old value struck through, then the new one;
  - added and cleared cells;
  - tabs added, removed or renamed.
  A long list says how many more cells changed.
- **CSV import.** Importing a CSV into a tab that already holds some of the same values works.
  Before, it was refused.

## Observed friction

| Friction | Evidence |
|---|---|
| A document's History compared only neighbouring versions; seeing what changed across a week meant reading every version in between | the diff took the previous version as its only base |
| A sheet's History listed versions with no way to see what changed, short of downloading two workbooks | the version menu offered Recover and Download only |
| Re-importing an updated CSV into its tab failed with "Spreadsheet command UPDATE_CELL rejected: NoChanges" | found while seeding two versions for this sprint. Any cell already holding the imported value made the engine refuse the whole import. Only empty-to-empty cells were skipped. |

## Changes

- `DocumentInspector`: "Compare with" offers the older versions while a version is previewed. The
  heading says "What changed since version N". The choice resets when another version is
  previewed, and arriving from "changed since you looked" still picks its own base.
- `sheet-diff.ts`: compares two saved workbooks by tab id, so a renamed tab is the same tab. It
  reads the cells as typed, so a formula shows as its formula. Results run in A1 order down the
  sheet, capped at 200 with a count of the rest.
- Sheet History: "What changed" in each version's menu opens a panel beneath the list. It compares
  against the version before, or shows everything in the first version.
- `native-sheets` worker: a cell whose content would not change is skipped, unless its format
  changes. That covers re-imports and agents writing the same value again.

## Verification

- `sheet-diff.test.ts`: four cases: changed, added and cleared cells; renamed, added and removed
  tabs; the cap; the first version.
- `services/native-sheets` adds a test: importing a CSV again changes only what differs, and an
  identical import sends nothing. It failed before the fix with the same "NoChanges" refusal,
  and passes after.
- Live, on a `_test` company:
  - Documents: a version-3 document compared with version 1 shows the change across both edits.
  - Sheets: two CSV imports, each saved as a version. "What changed" lists B2 40 → 45 and A4, B4
    added.
- `restless-engine` sheet integration and `restless-owner` sheet tests pass. svelte-check 0 errors
  and 0 warnings; web unit tests 131/131.

## Results

- No migration; schema stays 97.
