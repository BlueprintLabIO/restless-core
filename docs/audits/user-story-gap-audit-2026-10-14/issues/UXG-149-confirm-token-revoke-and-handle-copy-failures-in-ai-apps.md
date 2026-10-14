# UXG-149 — Confirm token revoke and handle copy failures in AI apps

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Apps | low | ux | US-083 |

## Problem

Revoke runs in one click; a failed clipboard write throws unhandled; "Done" discards the only copy of the secret without warning; no loading state; only a Claude Code snippet despite promising Desktop and Codex.

## Evidence

- `web/src/lib/components/AccountAiApps.svelte:58-73,` :108-109, :114-131
- `web/src/lib/components/AccountSectionPage.svelte:28`
- `web/src/lib/model/mcp-access.ts:37-40`

## Suggested direction

Confirm revoke, catch copy errors, warn before Done, add per-client snippets.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
