# UXG-247 — Stop a viewer from resizing the shared desktop while an agent works in it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | ux | US-141 |

## Problem

A tab that only observes still claims the display lease when nobody holds owner control, and the proxy then lets its SetDesktopSize through. Opening the computer on a phone or a narrow window resizes the agent's live desktop to that viewport mid-task, reflowing pages the agent may be acting on. Looking should not change what the worker sees.

## Evidence

- `crates/restless-owner/src/owner_desktop.rs:735-770` (first observer is granted the display lease when no owner controls)
- `crates/restless-owner/src/owner.rs:10918-10936` (resize allowed for the lease holder without control)
- `web/src/routes/[companyId]/company/computer/+page.svelte:462` (canResize passed to an observer)
- `web/src/lib/components/DesktopViewport.svelte:218-219`

## Suggested direction

Grant resize only with owner control, or while no agent browser session is live; otherwise scale the view locally.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
