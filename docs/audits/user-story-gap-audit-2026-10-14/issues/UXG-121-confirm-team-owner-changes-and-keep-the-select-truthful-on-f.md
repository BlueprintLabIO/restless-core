# UXG-121 — Confirm team-owner changes and keep the select truthful on failure

| Surface | Severity | Type | Stories |
|---|---|---|---|
| People | medium | ux | — (found while tracing adjacent paths) |

## Problem

"Owned by" posts on change with no confirm or toast; errors show far away; on failure the select keeps the rejected value. A select under every team header turns the sidebar into an admin form.

## Evidence

- `web/src/routes/[companyId]/people/+page.svelte:68-88,` :653-668, :714
- `web/src/routes/[companyId]/company/members/+page.svelte:152-154`

## Suggested direction

Move ownership into a team/profile menu with toast and Undo; reset on error.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
