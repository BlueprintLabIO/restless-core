# Sprint 71 — Built for everyone who uses it

**Status:** landed
**Programme:** owner cockpit, accessibility
**Depends on:** Sprint 70.

## Outcome

The everyday screens pass an automated accessibility audit (axe-core) at desktop and phone width.
Each page has one main region and a level-one heading, and every control has a name, as a
production product's do.

## Observed friction

Read on 13 October 2026 from `main` at `ecfe384f`. axe-core ran on ten screens at 1280 × 860 and
390 × 844:
- the Inbox
- the Library
- a document, and its Share and Comments panels
- a sheet, and its Share panel
- People
- Members
- Work

| Finding | Impact | Where |
|---|---|---|
| A list whose first child is not a list item | critical | the Library's file list contained its sort header |
| Form controls without names | critical | the spreadsheet engine's font size and "rows to add" inputs |
| Tab order above zero | serious | the spreadsheet engine's two cell composers |
| A main landmark inside another, and more than one main | moderate | eight screens: pages drew their own `<main>` inside the shell's |
| No level-one heading | moderate | the Inbox, a document, and an editable sheet, whose titles are fields |

## Tickets

- [x] **S71-T1 The Library list.**
  - The sort header sits above the list.
  - The list holds only its rows, each a list item (a new optional `role` on `Item`).
  - "Load more" follows the list.
- [x] **S71-T2 The spreadsheet engine's controls.** An observer on the sheet surface names the font
  size and "rows to add" inputs, and resets any positive tab order to 0, however often the engine
  redraws them. The surface element is reactive, so this runs once the surface exists.
- [x] **S71-T3 One main region.** Pages inside the company shell use `<section>`; the shell's
  `main#main-content` is the page's only main region.
- [x] **S71-T4 A heading on every page.**
  - The Inbox names itself to screen readers.
  - A document's print title doubles as its screen-reader heading. Print undoes the hiding.
  - An editable sheet names its title.

## Results

- Desktop: no findings of moderate or higher impact on any of the ten screens.
- Phone: one finding remains.

**Known limit:** with a sheet's Share panel open on a phone, the panel covers the sheet's header,
so that view has no level-one heading.
