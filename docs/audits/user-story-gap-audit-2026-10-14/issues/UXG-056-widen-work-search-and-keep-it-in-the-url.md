# UXG-056 — Widen Work search and keep it in the URL

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Work | low | missing-function | US-030 |

## Problem

Search matches only title and outcome, skips owner/goal/outputs and stopped work, and is component state, lost on Back, despite the page's own comment that filters live in the address.

## Evidence

- `web/src/routes/[companyId]/work/+page.svelte:97-107,` :132-142

## Suggested direction

Put `q=` in the URL and match owner, goal and output labels.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
