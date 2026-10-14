# UXG-323 — Return the owner to where they started after an app sign-in, and wake the work that failed meanwhile

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | low | ux | US-207 |

## Problem

The Inbox "Sign in again" action and Exec's app requests send the owner into Apps, and the OAuth callback always lands on the app page. A successful sign-in resolves only pending app-request handoffs. Work whose attempt failed while the sign-in was expired gets no resume signal, and the Inbox item only promises tools for "new agent sessions". The owner has to go back to the Inbox and to Exec to restart what stalled.

## Evidence

- `crates/restless-owner/src/owner_tool_connections.rs:277-291` (callback redirects to app path)
- `crates/restless-owner/src/owner_tool_connections.rs:376-417` (only app_request handoffs resolved)
- `crates/restless-engine/src/attention.rs:776-784` (open-app action
- tools return only to new sessions)

## Suggested direction

Carry a return path through the sign-in, and on reconnect wake actors whose recent attempt failed on that connection.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
