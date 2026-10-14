# UXG-251 — Let the owner drop a file into the live desktop during a handover

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | missing-function | US-145 |

## Problem

When the company brings a web form that needs the owner's own document (ID scan, signed contract), the owner has no way to put that file where the company browser's upload dialog can find it. The desktop clipboard carries text only, and the handover composer attaches files to a message for the lead rather than to the desktop. Distinct from UXG-250, which is about getting files out.

## Evidence

- `web/src/lib/components/DesktopViewport.svelte:543-613` (clipboard send is text via clipboardPasteFrom)
- `web/src/routes/[companyId]/+page.svelte:564-583` (handover files go to the lead conversation)
- `crates/restless-engine/src/runtime.rs:3808-3823` (attachments land in a root-owned directory, not a user-visible folder)

## Suggested direction

Accept a drop or file pick on the desktop that lands in a visible folder (for example ~/Downloads) and says where it went.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
