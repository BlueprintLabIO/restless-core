# UXG-288 — Reserve app route names so a company id cannot be shadowed

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | low | consistency | US-162 |

## Problem

Company ids only need to be lowercase letters, digits and underscores, so a company created through the API or CLI as "account", "gallery", "connections", "documents" or "company" is accepted, but the SPA’s top-level static routes win and that company’s cockpit can never be opened. The UI generator avoids this by using company_<hex>, but nothing enforces it.

## Evidence

- `crates/restless-engine/src/runtime.rs:1211-1227` (name rule has no reserved words)
- `crates/restless-owner/src/owner.rs:3538-3541` (create accepts any valid name)
- `web/src/app.html:27-33` (top-level route names the SPA claims)
- `web/src/lib/components/CreateCompany.svelte:47` (UI uses company_ ids)

## Suggested direction

Reject the SPA’s top-level route names in validate_company_name.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
