# UXG-244 — Keep the handover desktop connected after returning control

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | high | ux | US-139 |

## Problem

After Return control, a heartbeat failure or a status sync, the Inbox sets the desktop to /desktop/{company}/observe with no client_id. The owner API rejects that route with "client_id is required", so the viewer drops to a connection error right when the owner hands back and wants to watch the agent resume. The Company → Computer page instead keeps one stable observer connection and flips only a client-side gate, so the two desktop views behave differently for the same action.

## Evidence

- `web/src/routes/[companyId]/+page.svelte:453-459` (observedDesktopUrl lacks client_id
- controlledDesktopUrl has it)
- `web/src/routes/[companyId]/+page.svelte:471`
- `web/src/routes/[companyId]/+page.svelte:528`
- `web/src/routes/[companyId]/+page.svelte:549`
- `crates/restless-owner/src/owner_desktop.rs:331-333` (observe returns 400 without client_id)
- `web/src/routes/[companyId]/company/computer/+page.svelte:124-142` (stable connection, no URL swap)

## Suggested direction

Share one desktop session controller between the Inbox handover and the Computer page so control changes never re-route the connection.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
