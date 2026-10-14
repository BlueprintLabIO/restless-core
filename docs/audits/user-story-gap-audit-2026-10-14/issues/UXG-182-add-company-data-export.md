# UXG-182 — Add company data export

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | medium | missing-function | US-098 |

## Problem

No route or UI exports a company's charter, documents, decisions, receipts or settings.

## Evidence

- `crates/restless-owner/src/owner_documents.rs:96` and web/src/lib/model/sharing.ts:47 are the only exports

## Suggested direction

"Export company" producing an archive of files and the governance record.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
