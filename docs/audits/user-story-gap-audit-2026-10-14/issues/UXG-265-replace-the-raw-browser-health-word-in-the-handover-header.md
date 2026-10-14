# UXG-265 — Replace the raw browser health word in the handover header

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | low | ux | US-139 |

## Problem

The handover header prints the attention source-health value for the browser verbatim in a monospace chip, before "You control" or "Observe only". Owners see engine words such as "available" or "unknown" with no meaning at the moment they need to know whether the session is ready.

## Evidence

- `web/src/routes/[companyId]/+page.svelte:798-805` (view.sourceHealth.browser in a mono span)
- `crates/restless-engine/src/attention.rs:58-63` (SourceHealth.browser is a free string)

## Suggested direction

Show a plain readiness phrase, or nothing when ready, and keep the raw value for diagnostics.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
