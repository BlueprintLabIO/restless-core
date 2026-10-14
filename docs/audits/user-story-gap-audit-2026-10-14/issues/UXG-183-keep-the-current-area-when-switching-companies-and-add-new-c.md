# UXG-183 — Keep the current area when switching companies, and add New company to the switcher

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | low | ux | US-099 |

## Problem

Switcher entries link to `/{id}`, so switching from Limits lands on the other company's home; no New company, search or Inbox counts.

## Evidence

- `web/src/lib/components/AppShell.svelte:296-347` (href at :313)

## Suggested direction

Keep the sub-path; add "Start a company" and waiting counts.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
