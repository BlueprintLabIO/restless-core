# UXG-269 — Show what needs the owner in the tab title or favicon

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | ux | US-167 |

## Problem

The Inbox and People badges exist only inside the page; titles are "Inbox — Acme" or "Work — Acme" whatever is waiting, and the favicon is a fixed SVG. An owner who keeps the cockpit in a background tab (the common case for a product that runs the business for you) cannot see at a glance that a decision is waiting. The Inbox title also falls back to the raw company id while loading.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:809-829` (badge counts computed for tabs only)
- `web/src/lib/primitives/CompanyTitle.svelte:16` (title has no count)
- `web/src/routes/[companyId]/+page.svelte:588` (Inbox title falls back to companyId)
- `web/src/app.html:84` (static favicon)

## Suggested direction

Prefix the document title with the needs-you count and optionally badge the favicon, from the same source as the tab badge.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
