# UXG-264 — Delete the never-rendered authority page and keep the harness anchor when redirecting

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | low | consistency | US-153 |

## Problem

company/authority has a +page.svelte whose only content is a comment saying it never renders, because its +page.ts redirects. The old harnesses and setup routes redirect to /company/provider without #harnesses, although CompanyProvider opens the harness section only for that hash, so old harness links land with it folded. CLAUDE.md treats deleting dead machinery as progress.

## Evidence

- `web/src/routes/[companyId]/company/authority/+page.svelte:1` (never rendered)
- `web/src/routes/[companyId]/company/authority/+page.ts:4-6`
- `web/src/routes/[companyId]/company/harnesses/+page.ts:3-5` (no hash)
- `web/src/routes/[companyId]/company/setup/+page.ts:3-5`
- `web/src/lib/components/CompanyProvider.svelte:137-139` (#harnesses opens the section)

## Suggested direction

Remove the dead page and redirect harness links to /company/provider#harnesses.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
