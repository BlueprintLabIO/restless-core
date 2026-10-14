# UXG-063 — Hide run internals and raw enum words from the default Work view

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | consistency | — (found while tracing adjacent paths) |

## Problem

The rail always shows the raw workspace path in monospace; rows and detail show raw states ("changes requested", "superseded", "stale") and "Attempt 3 · revision 2". CLAUDE.md says IDs and logs appear only on request.

## Evidence

- `web/src/routes/[companyId]/work/[workId]/+page.svelte:312-314,` :710-718, :812-825
- `web/src/routes/[companyId]/work/+page.svelte:335,` :342
- `web/src/lib/work/status.ts:15-17`

## Suggested direction

Map states to owner words; move Workspace into Run details.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
