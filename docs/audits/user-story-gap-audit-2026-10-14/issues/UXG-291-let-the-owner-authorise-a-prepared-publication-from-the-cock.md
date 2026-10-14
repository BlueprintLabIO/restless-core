# UXG-291 — Let the owner authorise a prepared publication from the cockpit

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | high | missing-function | US-178, US-204 |

## Problem

When the company prepares a demo or game-server build for publishing, the company projection lists it as a resource awaiting owner authorization of its audience, expiry, resource limits and immutable build. Authorising it is an owner-only command, and the only door is the terminal. There is no owner HTTP route for it, and Resources only lists items that can be launched, so the waiting item has no visible action. This is the case the product promises to handle as a prepared last mile: the company has done the work and the owner only needs to say yes, but the yes is a CLI command.

## Evidence

- `crates/restless-engine/src/company.rs:1033-1042` (published_service resource, "await owner authorization" detail)
- `crates/restless-engine/src/publication.rs:1-6` (only the local owner principal can authorize)
- `crates/restless-engine/src/wire.rs:1306` (publish-authorize is OWNER_ONLY)
- `crates/restless/src/main.rs:665-669` (CLI publish authorize)
- `crates/restless/src/main.rs:2696-2698` (CLI sends publish-authorize)
- `web/src/routes/[companyId]/company/resources/+page.svelte:21` (only items with launch are listed)
- `web/src/routes/[companyId]/company/resources/+page.svelte:96-107` (rows offer Open/Launch only)

## Suggested direction

Add an owner route for publication authorise and raise it as an owner_judgement Inbox item that shows audience, expiry, limits and the exact build, with Authorise / Decline.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
