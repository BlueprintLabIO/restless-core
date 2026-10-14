# UXG-231 — Stop a refused sheet change from discarding every edit queued behind it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | high | ux | US-132 |

## Problem

When the server refuses a change (for example a 31st tab or an oversized structure), or a paste is too large to send, the sheet reopens at its saved state and clears every pending revision, including later edits that were fine. The notice says only that "your last change" was not saved, so the owner does not know other typing was lost too.

## Evidence

- `web/src/lib/model/sheet-editor.ts:272-285` (oversized change: "reopening drops it locally, with the pending edits queued behind it")
- `web/src/lib/model/sheet-editor.ts:318-337` (reload clears pending)
- `web/src/lib/model/sheet-editor.ts:411-420` (server ERROR → reload)
- `services/native-sheets/src/worker.mjs:49` (30-tab limit refusal)

## Suggested direction

Drop only the refused revision and replay the rest, or at minimum count and name the discarded edits in the notice; check tab and size limits before sending.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
