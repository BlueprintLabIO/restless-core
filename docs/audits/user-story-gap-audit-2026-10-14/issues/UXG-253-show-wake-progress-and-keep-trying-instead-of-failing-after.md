# UXG-253 — Show wake progress and keep trying instead of failing after 20 seconds

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | ux | US-146 |

## Problem

Waking a sleeping computer starts the container, rebuilds the bridge and then waits up to 20 seconds for the desktop. If the desktop is slower, the owner gets "not ready yet. Try again in a moment" and the page stops; the button reads only "Waking…" with no sense of how long it usually takes. Retrying too soon after a failed wake can hit the start backoff and return the same generic sentence. Distinct from UXG-186, which is about the sidebar's Computer value.

## Evidence

- `crates/restless-owner/src/owner_desktop.rs:133-141` (503 when the desktop is not ready)
- `crates/restless-owner/src/owner_desktop.rs:220-231` (20 second bound)
- `web/src/routes/[companyId]/company/computer/+page.svelte:71-74` (label only)
- `web/src/routes/[companyId]/company/computer/+page.svelte:154-170` (single attempt, no retry)
- `crates/restless-engine/src/runtime_sleep.rs:340-346` (backoff refuses wake)

## Suggested direction

Show staged wake progress with a typical duration, and retry the ticket automatically until a bounded deadline.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
