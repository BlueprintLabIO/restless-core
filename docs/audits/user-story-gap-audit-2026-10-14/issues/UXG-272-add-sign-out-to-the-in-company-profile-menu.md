# UXG-272 — Add Sign out to the in-company profile menu

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | missing-function | US-161 |

## Problem

The profile circle that every owner and member sees inside a company offers Account, Appearance, Desktop notifications and Keyboard shortcuts but no Sign out. Signing out exists only in the account frame’s menu, and only on a hosted plane served from the issuer’s own origin, so a person must first leave the company and find it there. On a shared or borrowed computer that is the one action they most need to find quickly.

## Evidence

- `web/src/lib/components/ProfileMenu.svelte:44-73` (menu contents, no sign out)
- `web/src/lib/components/AccountFrame.svelte:164-173` (Sign out only when issuer === origin)
- `web/src/lib/components/AccountFrame.svelte:171` (form POST /account?/signOut)

## Suggested direction

Show the same Sign out form in ProfileMenu whenever the plane is hosted, and hide it locally where there is no session.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
