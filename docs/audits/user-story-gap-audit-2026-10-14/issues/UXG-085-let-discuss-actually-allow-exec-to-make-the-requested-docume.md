# UXG-085 — Let Discuss actually allow Exec to make the requested document edit

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Library | high | spec-drift | US-050 |

## Problem

ARCHITECTURE §4.3 says Exec applies bounded edits under existing document edit access. Discuss grants Exec only `comment`, and only on docs the owner created, so "tighten this paragraph" cannot be applied without finding Share and upgrading Exec.

## Evidence

- `web/src/routes/[companyId]/library/documents/+page.svelte:255-272`
- ARCHITECTURE.md:334-337

## Suggested direction

Offer an edit grant (with clear confirmation) when asking Exec to change the doc, or route via visible proposals.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
