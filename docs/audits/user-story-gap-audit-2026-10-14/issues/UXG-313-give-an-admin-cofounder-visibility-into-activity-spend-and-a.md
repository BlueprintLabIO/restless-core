# UXG-313 — Give an admin cofounder visibility into Activity, spend and Apps

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | missing-function | US-203 |

## Problem

Admin differs from member only in managing setup (members, identity, schedules). The shell gives an admin the same five tabs as a member, and every Company page outside charter, identity, members and schedules is refused. A cofounder brought in to share the running of the company cannot see what the company sent, decided or spent, or which apps it uses, and cannot do the weekly review with the owner. Distinct from UXG-020 (delegating single items) and UXG-024 (review-gate roles).

## Evidence

- `web/src/lib/model/company-access.ts:21-27` (admin powers limited to setup
- shared sections list)
- `web/src/lib/model/company-access.ts:45-70` (admins refused everything else)
- `web/src/lib/model/company-access.ts:82-115` (admin gets member tabs, no Apps)
- `web/src/routes/[companyId]/company/members/+page.svelte:358` ("Admins can also invite and remove members")

## Suggested direction

Give admins read access to Activity, Limits and Apps, and let the owner scope approval rights for an admin explicitly.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
