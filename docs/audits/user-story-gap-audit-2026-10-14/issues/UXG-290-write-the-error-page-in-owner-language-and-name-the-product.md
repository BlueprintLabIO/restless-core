# UXG-290 — Write the error page in owner language and name the product in its title

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | ux | US-162 |

## Problem

The root error page shows page.error.message verbatim for any non-404 failure, which for an unexpected client error is a generic or technical string, contrary to failure.ts’s rule that raw transport text never reaches a screen. Its tab title is just "Not found" or "Something went wrong", without the product or company, so it is indistinguishable among tabs.

## Evidence

- `web/src/routes/+error.svelte:13-15` (title without product or company)
- `web/src/routes/+error.svelte:23-27` (raw page.error.message)
- `web/src/lib/model/failure.ts:1-12` (owner-facing failure policy)

## Suggested direction

Pass the error through describeFailure (or a fixed sentence) and use "<title> — <product>" for the tab.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
