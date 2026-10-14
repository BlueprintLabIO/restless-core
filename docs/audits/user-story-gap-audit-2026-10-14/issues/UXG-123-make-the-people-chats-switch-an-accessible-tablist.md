# UXG-123 — Make the People/Chats switch an accessible tablist

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | low | a11y | — (found while tracing adjacent paths) |

## Problem

role=tablist/tab without aria-controls, tabpanels or arrow keys; unread/working indicators are role-less spans with aria-label.

## Evidence

- `web/src/routes/[companyId]/people/+page.svelte:531-547,` :561-578, :602-606

## Suggested direction

Use a segmented control or real tab semantics; give indicators role="img" or sr-only text.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
