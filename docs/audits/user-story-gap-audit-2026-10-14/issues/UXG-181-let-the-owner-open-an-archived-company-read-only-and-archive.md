# UXG-181 — Let the owner open an archived company read-only and archive from inside Company

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | ux | US-097 |

## Problem

Archived rows have `entry: null`, so their charter, Library and Activity are unreadable; Restore restarts it. No lifecycle controls under Company.

## Evidence

- `web/src/routes/+page.svelte:110-120`

## Suggested direction

Read-only view of archived companies; a lifecycle section in Company.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
