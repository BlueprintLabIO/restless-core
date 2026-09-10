# S43-T3 — Bound desktop authentication, state and updates

**Layer:** Authority + application lifecycle  
**Serves:** Vendor desktops bring personal login, plugins and auto-update behavior that can bypass runtime policy.

## Work

- Keep host-native login on the host or implement an explicitly approved Runtime enrolment boundary
  without exposing reusable root/provider credentials to agents.
- Inventory declared app state, session history, plugins, connectors, MCP, approval caches, browser
  storage and update helpers for the counted build.
- Mark personal capabilities interactive-only and prevent their inheritance by certified harness launches.
- Disable or mediate silent auto-update; stage, verify, activate, probe and roll back exact builds.
- Define retain/export/remove behavior for personal and company application state.

## Acceptance

Adversarial inspection finds no credential crossing, silent policy augmentation or undeclared state.
Update failure returns to known-good behavior, and removal leaves or deletes only the state the owner chose.

## Makes deletable

Shared vendor homes, hidden personal MCP inheritance and uncontrolled desktop auto-update.
