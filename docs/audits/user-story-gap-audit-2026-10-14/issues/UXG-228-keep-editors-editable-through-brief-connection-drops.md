# UXG-228 — Keep editors editable through brief connection drops

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | high | ux | US-130 |

## Problem

A document is editable only while its live session is synced, so any blip (reconnecting, offline, a slow first connect) makes the page read-only mid-sentence; after 10 seconds without sync it shows the "latest named version", which can be older than what the owner just wrote. The sheet grid also flips to read-only whenever its socket closes. Unsynced changes live only in the tab (no local store), so closing it while offline loses them. For an owner on café or mobile data this reads as lost work.

## Evidence

- `web/src/lib/components/DocumentEditor.svelte:154-156` (canWriteLive requires synced)
- `web/src/lib/components/DocumentEditor.svelte:519-535` (offline → degraded, disconnect)
- `web/src/lib/components/DocumentEditor.svelte:492-503` (10 s deadline → named version read only)
- `web/src/lib/components/DocumentEditor.svelte:1250-1266` (shows rendered named version)
- `web/src/lib/model/sheet-editor.ts:482-490` (sheet updateMode readonly on close)

## Suggested direction

Keep both editors writable on the local CRDT / pending queue while reconnecting, persist pending changes locally, and show "Saved on this device, syncing" instead of switching to older content.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
