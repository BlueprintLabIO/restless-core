# UXG-176 — Page external actions and include owner setting changes in Activity

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | spec-drift | US-095 |

## Problem

External actions are capped at the last 50 receipts with no paging, so paging decisions silently misses older external items. Spend-limit changes, charter revisions, archive/restore are absent though governance-relevant.

## Evidence

- `crates/restless-engine/src/company.rs:1256-1262,` :1312
- `web/src/routes/[companyId]/company/activity/+page.svelte:149-154,` :311-322

## Suggested direction

Page receipts from the owner API; add lifecycle and setting-change events.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
