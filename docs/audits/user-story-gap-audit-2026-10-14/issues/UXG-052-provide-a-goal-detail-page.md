# UXG-052 — Provide a goal detail page

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | medium | spec-drift | US-035 |

## Problem

Choosing a goal only filters the list; its body, finish line and due date sit in an InfoTip. Spec §6.4 lists a goal page with definition of done, evidence, decisions and cost. The goal breadcrumb is not a link.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:270-274,` :530
- `web/src/routes/[companyId]/work/[workId]/+page.svelte:423`
- `docs/specs/owner-cockpit.md:666-680`

## Suggested direction

Render the goal's success contract and progress at the head of the filtered view; link the crumb.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
