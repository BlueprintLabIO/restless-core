# UXG-046 — Allow renaming a Work item and editing its scope or goal

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | missing-function | US-032 |

## Problem

Title, brief, expected output and goal are read-only, and the owner API has no work-update route. Fixing a wrong title or moving work to the right goal requires asking in chat.

## Evidence

- `web/src/routes/[companyId]/work/[workId]/+page.svelte:427,` :584-601, :763-769
- `crates/restless-owner/src/owner.rs:1340-1369`

## Suggested direction

Add an owner edit route (title, goal, brief) visible to Exec, with inline editing.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
