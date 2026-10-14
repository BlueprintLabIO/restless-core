# UXG-239 — Use the company's locale and currency for sheet formats

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | low | ux | US-135 |

## Problem

The model is built without any locale or currency settings, so number, date and currency formats follow the engine's defaults rather than the company's country; server messages also format counts as en-US. A UK or EU owner must change formats cell by cell or in the engine's own settings for every sheet.

## Evidence

- `web/src/lib/model/sheet-editor.ts:449-456` (Model config has no locale)
- `services/native-sheets/src/worker.mjs:19` (count uses en-US)

## Suggested direction

Pass the company's locale and default currency to new sheets, editable per sheet.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
