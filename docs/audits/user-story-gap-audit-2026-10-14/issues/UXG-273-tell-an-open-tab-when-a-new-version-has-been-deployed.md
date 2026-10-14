# UXG-273 — Tell an open tab when a new version has been deployed

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | ux | US-165 |

## Problem

There is no version polling or "updated" check, so a tab left open across a deploy keeps running the old client against the new daemon until something breaks. The service worker skips waiting, claims clients and deletes the previous shell cache, so the next lazy route chunk can 404; that surfaces as a silent forced reload (with the draft loss in UXG-271) or, for a missing API route, as "Restless needs a restart to finish updating" on a pane. The owner gets no calm, chosen moment to refresh.

## Evidence

- `web/svelte.config.js:10-20` (no kit.version.pollInterval)
- `web/src/service-worker.ts:11-33` (skipWaiting, clients.claim, old caches deleted)
- `web/src/lib/model/session-reentry.ts:181-189` (preload error triggers reload)
- `web/src/lib/model/failure.ts:162-171` (missing API route read as needing a restart)

## Suggested direction

Enable SvelteKit version polling and show a quiet "New version ready — Reload" notice that saves drafts first.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
