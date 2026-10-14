# UXG-040 — Bring the prepared ReviewTarget to the Work detail page

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | high | spec-drift | US-027 |

## Problem

"Open" appears only for Library-known artifacts or public http(s); runtime/localhost previews are excluded, so a site built on the company computer cannot be opened from its Work. The prepared review target flow lives only in the Inbox, contrary to "Outcome-native review".

## Evidence

- `web/src/routes/[companyId]/work/[workId]/+page.svelte:382-393,` :639-645
- `web/src/routes/[companyId]/+page.svelte:352-399`

## Suggested direction

Reuse the Inbox review-target opener for the latest attempt's candidate.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
