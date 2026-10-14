# UXG-268 — Notify for every company and name the company in each desktop notification

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | missing-function | US-167 |

## Problem

Desktop notifications are computed inside the open company’s layout, so an owner with several companies is only told about the one in the current tab; the others stay silent. Each notification’s title is the bare item title with no company name, and the notification tag is that title, so two different items with the same title (e.g. two "Approve email" requests) replace each other. Distinct from UXG-028/UXG-030, which are about members and closed-tab delivery.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:911-935` (only this company’s attention list, owner only)
- `web/src/routes/[companyId]/+layout.svelte:786-807` (chat notices also current company only)
- `web/src/lib/model/desktop-notify.svelte.ts:66-74` (title only, tag: title)

## Suggested direction

Drive notifications from the portfolio-level company list, prefix the company name and tag by company+item id.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
