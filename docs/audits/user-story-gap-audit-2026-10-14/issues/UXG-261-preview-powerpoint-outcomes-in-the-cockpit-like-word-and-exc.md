# UXG-261 — Preview PowerPoint outcomes in the cockpit like Word and Excel

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | ux | US-152 |

## Problem

Word and Excel outcomes open read-only in place, but a .pptx ReviewTarget falls into the download-only card telling the owner to open it in PowerPoint or Keynote. Slide decks are a common company deliverable, and CLAUDE.md asks that review bring the native outcome rather than a setup step. Owners without Office, or on a phone, cannot review the deck at all.

## Evidence

- `web/src/lib/model/library-files.ts:136-139` (officeFormat covers docx and xlsx only)
- `web/src/routes/[companyId]/+page.svelte:402-406` (pptx is download-only)
- `web/src/routes/[companyId]/+page.svelte:710-721` (download card copy)

## Suggested direction

Render slides to a viewable form (images or PDF) for review, keeping download beside it.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
