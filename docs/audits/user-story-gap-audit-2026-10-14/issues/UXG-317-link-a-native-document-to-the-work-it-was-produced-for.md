# UXG-317 — Link a native document to the Work it was produced for

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | ux | US-196, US-208 |

## Problem

The document review API returns `work_dependency.work_id`, but no component reads it, and the document page, inspector and Library viewer have no Work link. An owner reading a delivered proposal in the Library cannot jump to the Work, its lead, its attempts or its review state, and from search the document and the job look unrelated. Distinct from UXG-097 (document-to-document backlinks).

## Evidence

- `web/src/lib/model/documents.ts:247-258` (DocumentWorkReviewDependency.work_id)
- `web/src/lib/model/documents.ts:261-264` (DocumentReviewView carries work_dependency)
- `web/src/lib/model/document-queries.svelte.ts:470` (only use of DocumentReviewView
- work_dependency unused in any .svelte)

## Suggested direction

Show a "For: <Work title>" link in the document header or inspector when a Work dependency or artifact reference exists.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
