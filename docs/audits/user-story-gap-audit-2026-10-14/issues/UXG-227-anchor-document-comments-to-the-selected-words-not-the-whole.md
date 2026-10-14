# UXG-227 — Anchor document comments to the selected words, not the whole block

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | ux | US-129 |

## Problem

A comment is stored against a top-level block id only. Selecting one phrase inside a bullet, or one cell in a table, attaches the comment to the whole list or table, and the highlight marks the whole block. Reviewers cannot point at the sentence they mean, which is the core of commenting in Notion and Google Docs; the maturity note rates comments "at par" without this.

## Evidence

- `web/src/lib/components/DocumentEditor.svelte:616-630` (selectedBlock takes the closest top-level [data-block-id])
- `web/src/lib/model/documents.ts:583-593` (thread carries only block_id)
- `web/src/lib/model/document-editor-schema.ts:573-580` (has-comments decorates the whole node)
- `docs/maturity-vs-notion.md:27` (rated at par)

## Suggested direction

Store a text range (relative Yjs positions) with the thread and highlight just that range, keeping block anchoring as the fallback.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
