# UXG-246 — Show when an agent is using the computer, and who, before the owner's click takes over

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | ux | US-141 |

## Problem

The browser status endpoint returns only the owner lease, which the Runtime records as either owner or unclaimed; the "agent controls" branch in the toolbar label can never be reached. While an agent is mid-action in the shared browser the owner sees "Viewing only", and the portal invites "Click or type on the desktop to join in", where the first click silently claims control and pauses that agent. The live agent session the owner API already reads to attribute leases is never shown.

## Evidence

- `web/src/routes/[companyId]/company/computer/+page.svelte:75-91` (agent branch reads control.controller === 'agent')
- `crates/restless-engine/src/runtime.rs:3268-3292` (only owner or unclaimed is written)
- `crates/restless-owner/src/owner.rs:10992-11005` (browser_status returns generation, browser, control only)
- `crates/restless-owner/src/owner.rs:10750-10778` (live_browser_agent_session exists)
- `web/src/routes/[companyId]/company/computer/+page.svelte:500-503` (join-in copy)

## Suggested direction

Include the live agent session (display name, Work) in browser status and confirm the first takeover when an agent is mid-task.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
