# UXG-026 — Show what was asked, and by whom, on document items

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | ux | US-020 |

## Problem

Document items open straight into the editor. `request.summary` and `requested_by_actor_id` are never shown, so the owner does not know what the agent wants changed.

## Evidence

- `web/src/lib/components/AttentionDocument.svelte` (no summary/requested_by)
- `web/src/lib/model/view.ts:4-13`
- `web/src/routes/[companyId]/+page.svelte:1044-1051`

## Suggested direction

Add a slim request line (asker + summary) above the editor.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
