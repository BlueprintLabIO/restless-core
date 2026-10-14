# UXG-250 — Let the owner see and download files that exist only on the company computer

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | high | missing-function | US-144 |

## Problem

The Computer page promises "Your team's shared browser, files and applications", yet no owner route lists, previews or downloads arbitrary files from the computer; the desktop routes cover windows, display and control only. A file the company browser downloaded (an invoice, a bank export, a government receipt) or a scratch output not registered as Work cannot reach the owner except by asking an agent to register it. The clipboard bridge carries text only.

## Evidence

- `web/src/routes/[companyId]/company/computer/+page.svelte:500-503` (copy promises files)
- `crates/restless-owner/src/owner.rs:1635-1653` (desktop and browser routes, none for files)
- `web/src/lib/components/DesktopViewport.svelte:27-29` (clipboard is text only, 64,000 characters)
- `crates/restless-engine/src/runtime.rs:3808-3823` (owner files only flow inbound to a sealed directory)

## Suggested direction

Offer a read-only file view of the company home (Downloads first) with download, scoped to the owner.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
