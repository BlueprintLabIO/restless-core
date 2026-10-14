# UXG-142 — Make a disabled skill a visible state and expose per-agent/team assignment

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | medium | missing-function | US-081 |

## Problem

Turning off "Everyone" says only agents given it directly can use it, but the UI can never give it to anyone (API supports team/actor scope; UI sends only company). An off skill still looks in use.

## Evidence

- `web/src/routes/[companyId]/apps/[app]/+page.svelte:403-422`
- `web/src/lib/model/skills.ts:314-325`
- `web/src/lib/model/apps.ts:364-367`

## Suggested direction

Explicit Off state in lists plus "Give to…" a person or team.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
