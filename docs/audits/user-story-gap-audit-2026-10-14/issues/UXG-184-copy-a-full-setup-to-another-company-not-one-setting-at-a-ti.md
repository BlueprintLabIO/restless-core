# UXG-184 — Copy a full setup to another company, not one setting at a time

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | missing-function | US-099 |

## Problem

"Copy from another company…" exists per setting and always sends one section; nothing for schedules, vault names or brand; copying the name creates duplicates; copies cannot be undone.

## Evidence

- `web/src/lib/components/CopyCompanySetting.svelte:29,` :103, :176-190

## Suggested direction

"Start from another company's setup" at creation with a section checklist.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
