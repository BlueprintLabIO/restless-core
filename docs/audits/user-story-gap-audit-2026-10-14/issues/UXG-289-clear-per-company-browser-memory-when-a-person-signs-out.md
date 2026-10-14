# UXG-289 — Clear per-company browser memory when a person signs out

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | security | US-161 |

## Problem

Recents store document, sheet and conversation titles per company and person in localStorage, along with remembered company names and rail topics, and signing out is a plain form post that clears none of it. On a shared computer the next person can read titles of confidential documents and the names of companies the previous person ran.

## Evidence

- `web/src/lib/model/recents.ts:112` (per company and actor key)
- `web/src/lib/model/recents.ts:134-136` (titles written to localStorage)
- `web/src/routes/[companyId]/+layout.svelte:251-266` (company name cached)
- `web/src/routes/[companyId]/+layout.svelte:291-317` (rail topics cached)
- `web/src/lib/components/AccountFrame.svelte:171` (sign out with no client cleanup)

## Suggested direction

Remove restless:* keys on sign-out (and on a 401 re-entry to a different account) before leaving the page.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
