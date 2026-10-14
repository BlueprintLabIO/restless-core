# UXG-001 — Let the owner edit or remove any released brand statement, not only their own pillar text

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Brand & voice | high | missing-function | US-002, US-003 |

## Problem

The Direction editor only edits the owner's own four pillar texts (`source = owner_identity_editor`). Every statement Exec drafted and the owner promoted is read-only and cannot be deleted. On save the backend unconditionally carries every non-owner statement into the new release, so a wrong claim promoted once lives forever. This is the "there is no way to edit a created brand/voice" complaint.

## Evidence

- `crates/restless-orgintel/src/identity.rs:52-59` (non-owner evidence always carried forward)
- `web/src/lib/components/IdentityEditor.svelte:66-80` (`written` vs `others`), :209-222 (others rendered read-only)
- `web/src/routes/[companyId]/company/identity/+page.svelte:349-376` (released statements, no actions)

## Suggested direction

Make every released statement editable and removable in the next version (owner edits supersede via `supersedes_evidence_id`), keeping attribution and history.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
