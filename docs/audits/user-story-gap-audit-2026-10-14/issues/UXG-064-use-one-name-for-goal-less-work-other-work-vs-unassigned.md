# UXG-064 — Use one name for goal-less work ("Other work" vs "Unassigned")

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | consistency | — (found while tracing adjacent paths) |

## Problem

The sidebar says "Other work"; the detail breadcrumb and Goal cell say "Unassigned", which suggests no person is assigned.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:204,` :488
- `web/src/routes/[companyId]/work/[workId]/+page.svelte:423,` :768

## Suggested direction

Use one term in both places and link it to `?goal=other`.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
