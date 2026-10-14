# UXG-185 — Add a company-wide freeze of outside actions and payment-allowance controls

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company | high | spec-drift | US-100 |

## Problem

Authority is read-only lists; no control to change what asks first, set allowances, or freeze all outside actions. Spec §8.3 requires "freeze all consequential external effects". The only freeze is per tool connection.

## Evidence

- `web/src/lib/components/CompanyLimits.svelte:360-417`
- `docs/specs/owner-cockpit.md:970-985`
- `crates/restless-owner/src/owner.rs:1558`

## Suggested direction

Emergency "Pause outside actions" switch with resume, and an Authority-backed allowance editor.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
