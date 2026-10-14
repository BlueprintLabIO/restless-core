# UXG-283 — Take the product name from brand config in the shell, offline page and manifest

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | consistency | US-176 |

## Problem

brand.ts says nothing else hard-codes the product name, but the stalled-navigation heading, the service worker’s offline page, the update failure sentence, the legacy knowledge route title, app.html’s application-name and the web manifest all say "Restless" literally. Distinct from UXG-188, which covers a single model-catalog tooltip; these are shell-level surfaces a rebranded install would still show. The offline page is also light-only regardless of theme.

## Evidence

- `web/src/lib/brand/brand.ts:1-12` (single source claim)
- `web/src/routes/+layout.svelte:231` (heading)
- `web/src/service-worker.ts:55` (title)
- `web/src/service-worker.ts:77` (body copy)
- `web/src/lib/model/failure.ts:169` (update sentence)
- `web/src/app.html:79` (application-name)
- `web/static/manifest.webmanifest:2-3` (name, short_name)

## Suggested direction

Import PRODUCT_NAME in the shell and failure copy and generate the manifest, app.html meta and worker fallback from the same config at build time.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
