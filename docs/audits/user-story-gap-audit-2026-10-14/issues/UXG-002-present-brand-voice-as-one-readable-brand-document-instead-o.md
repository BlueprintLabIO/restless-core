# UXG-002 — Present Brand & voice as one readable brand document instead of fragmented statement rows

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Brand & voice | high | ux | US-005 |

## Problem

The page breaks the identity into atomised "statements" and shows them up to four times: the Direction rows, "Released statements" split across four folds with kind labels, each proposal's expansion, and every Version history fold. Proposals are list items titled by their rationale. The owner cannot read their brand top to bottom, and the card/row-per-statement shape is a data-model view, not an owner one. Brand is a living document, so the Library's native document editor (comments, history, sharing) is the natural home.

## Evidence

- `web/src/routes/[companyId]/company/identity/+page.svelte:204-261` (proposals as Items), :324-376 (Current version + Released statements folds), :398-415 (history repeats statements)
- `web/src/lib/components/IdentityEditor.svelte:156-240`

## Suggested direction

Render the current identity as one document (four headed sections) edited in place, with proposals shown as suggested edits/diffs on that document; keep statement-level evidence behind a details toggle.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
