# UXG-241 — Add page breaks and page numbers to printed documents

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | low | missing-function | US-137 |

## Problem

Print sets only page margins: there is no page-break block and no page numbers or document title in the footer, so a printed or PDF'd contract or report breaks at arbitrary points and cannot be referenced by page.

## Evidence

- `web/src/lib/components/document-print.css:8-11` (@page margin only)
- `web/src/lib/components/DocumentBlockMenus.svelte:119-125` (no page-break insert)

## Suggested direction

Add a "Page break" block (break-before in print only) and page-number / title margin boxes in @page.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
