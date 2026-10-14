# UXG-271 — Keep unsent text across session re-entry and update reloads, and say when re-entry gave up

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | ux | US-160, US-165 |

## Problem

A lapsed hosted session, a plane update or a failed lazy chunk all end in window.location.reload or replace, and composer text is not persisted anywhere, so a half-written direction to Exec is lost — while the startup dialog promises "Your open page and drafts are being kept in place". When the loop guard suppresses a second reload within 15 s, or the one-per-company retry was already used, nothing is shown at all and later writes just fail. Distinct from UXG-270, which is about the blocking dialog’s actions.

## Evidence

- `web/src/lib/model/session-reentry.ts:116-136` (reload
- guard returns silently at 123-126)
- `web/src/lib/model/session-reentry.ts:181-189` (preload failure also reloads)
- `web/src/lib/model/reenter.ts:231-244` (location.replace to /account)
- `web/src/routes/[companyId]/+layout.svelte:1505` (drafts promise)
- `web/src/lib/primitives/Composer.svelte:93-104` (only a hint counter is stored, not the draft)

## Suggested direction

Persist rail and room composer drafts per conversation in sessionStorage before any forced reload, and show a "Signed out — sign in again" notice when the guard blocks re-entry.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
