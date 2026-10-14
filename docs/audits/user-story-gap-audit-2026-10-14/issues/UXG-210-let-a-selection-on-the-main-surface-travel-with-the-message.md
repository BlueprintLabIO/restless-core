# UXG-210 — Let a selection on the main surface travel with the message to Exec

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Exec chat | medium | missing-function | US-114 |

## Problem

The context chip links only the current URL path, and pages publish only a title. An owner reading a long document, a sheet range or one Inbox paragraph cannot say "explain this" about a selection; Exec receives the page and must guess. This weakens the rail's role to focus, explain and act on the main surface.

## Evidence

- `web/src/lib/model/attention.ts:427-442` (context is pathname+search only)
- `web/src/lib/model/page-context.svelte.ts:4-13` (pages publish a title only)
- `web/src/lib/components/ExecutiveRail.svelte:1386-1400` (chip toggles page link only)

## Suggested direction

When text or rows are selected, offer the chip as "Selection in …" and send the excerpt alongside the path.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
