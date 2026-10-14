# UXG-219 — Insert links to documents, Work and files as titled chips, and show where links go

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Doc & sheet editors | medium | missing-function | US-124 |

## Problem

A reference chip that opens a document, Work item, file or person exists in the schema, but a writer can only get one by copying an existing chip; pasting a cockpit URL or using ⌘K produces a bare path link with the address as its text. There is no "/" entry or [[ search for another page, no title unfurl for outside links, and the link's address is not shown on hover unless the owner opens the link bar. Pages therefore cannot be woven into the company's work the way agents' references are.

## Evidence

- `web/src/lib/model/document-editor-schema.ts:286-306` (documentReferenceHref supports document/work/artifact/actor)
- `web/src/lib/model/document-editor-schema.ts:330-340` (reference parsed only from pasted markup)
- `web/src/lib/components/DocumentEditor.svelte:796-807` (companyRelative keeps a cockpit link as a plain path)
- `web/src/lib/components/DocumentEditor.svelte:809-833` (applyLink inserts the href as text)
- `web/src/lib/model/document-editor-schema.ts:437-445` (link renders only a title attribute, not its address)

## Suggested direction

Turn a pasted cockpit link into a reference chip with its title, add a "/" link-to-page search, and show a small hover card with the address and Open / Edit.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
