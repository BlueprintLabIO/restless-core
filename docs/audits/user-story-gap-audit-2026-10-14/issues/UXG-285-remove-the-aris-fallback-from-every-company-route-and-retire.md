# UXG-285 — Remove the 'aris' fallback from every company route and retire the Aris knowledge route

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | consistency | US-163 |

## Problem

Twenty files default the company to 'aris' when the route has no id, including shared components (RoomConversation, CompanyLimits) and the Company, Work and Apps layouts, and /company/knowledge/* is a whole route that searches the 'aris' company and prints Aris-specific copy on every install. Distinct from UXG-093, which covers only the Library routes; this is the cross-cutting remainder plus a shipped customer-specific route.

## Evidence

- `web/src/routes/[companyId]/+layout.svelte:89` ('aris' fallback)
- `web/src/lib/components/RoomConversation.svelte:77` ('aris' fallback)
- `web/src/lib/components/CompanyLimits.svelte:16` ('aris' fallback)
- `web/src/routes/[companyId]/company/+layout.svelte:18` ('aris' fallback)
- `web/src/routes/[companyId]/apps/+layout.svelte:43` ('aris' fallback)
- `web/src/routes/company/knowledge/[...filePath]/+page.svelte:10-52` (Aris-only legacy route)

## Suggested direction

Read companyId as required from the route params and delete the Aris compatibility route.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
