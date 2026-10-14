# UXG-154 — Settle naming of know-how vs skills and describe local tools accurately

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | low | consistency | — (found while tracing adjacent paths) |

## Problem

A local tool is "run on your machine… outside the company computer" in one tooltip and "in its own sandbox from the company computer's image" on its page. The same thing is "Skills", "Know-how" and "skills that come with Restless".

## Evidence

- `web/src/lib/model/apps.ts:300-304,` :431-434
- `web/src/routes/[companyId]/apps/[app]/+page.svelte:216-218,` :376
- `web/src/routes/[companyId]/apps/+page.svelte:440`

## Suggested direction

One owner word ("know-how") with "skill" in a tooltip; one accurate description.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
