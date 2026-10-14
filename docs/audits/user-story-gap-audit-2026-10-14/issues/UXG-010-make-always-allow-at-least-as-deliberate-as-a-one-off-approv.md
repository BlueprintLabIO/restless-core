# UXG-010 — Make "Always allow" at least as deliberate as a one-off approval

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | high | ux | US-009 |

## Problem

A one-off approval needs hold-to-confirm, but "Always allow", which hands over standing authority, is a plain single-click button. The broader, riskier grant is the easier one to trigger.

## Evidence

- `web/src/lib/components/InboxDetail.svelte:601-617`

## Suggested direction

Put grant-always behind the hold, or a short confirm naming exactly what will stop asking.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
