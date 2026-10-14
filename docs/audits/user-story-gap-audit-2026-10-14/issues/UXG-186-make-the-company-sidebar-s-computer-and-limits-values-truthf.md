# UXG-186 — Make the Company sidebar's Computer and Limits values truthful

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | consistency | — (found while tracing adjacent paths) |

## Problem

Computer reads "Running" whenever not asleep even if the desktop failed its live check; Limits reads "Not set" when no ask-first rule exists though a spend ceiling always exists.

## Evidence

- `web/src/lib/model/company-setup-rows.svelte.ts:172-181`

## Suggested direction

Base Computer on live status; show spend percent on Limits.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
