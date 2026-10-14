# UXG-029 — Reconsider the pixel office as the clear-Inbox home

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Inbox | medium | consistency | US-008 |

## Problem

When the queue is clear, the main surface is an animated office of every agent with live presence: close to the "roster of agents / status wall" the working agreement forbids, with no outcomes or next actions. It silently truncates at 36 members (no "+N"), and decorations persist only in localStorage.

## Evidence

- `web/src/routes/[companyId]/+page.svelte:1090-1109`
- `web/src/lib/office/officePlan.ts:40,` :369
- `web/src/lib/office/CompanyOffice.svelte:65,` :107-110

## Suggested direction

Lead with recent outcomes and next actions; make the office opt-in, show overflow if kept.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
