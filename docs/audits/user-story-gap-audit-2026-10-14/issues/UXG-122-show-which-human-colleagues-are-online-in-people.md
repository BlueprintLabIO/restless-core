# UXG-122 — Show which human colleagues are online in People

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | missing-function | — (found while tracing adjacent paths) |

## Problem

Presence is collected but appears only as non-clickable initials in the top bar; human rows read "Colleague".

## Evidence

- `web/src/lib/model/presence.svelte.ts:1-44`
- `web/src/lib/components/ProfileMenu.svelte:28-42`
- `web/src/routes/[companyId]/people/+page.svelte:183-184`

## Suggested direction

Presence dot on human rows and DM header; make facepile faces open the DM.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
