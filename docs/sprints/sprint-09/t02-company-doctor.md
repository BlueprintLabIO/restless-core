# S09-T2 · Give the general doctor its own Company page

**Layer:** Runtime, Authority lifecycle record and owner surface.

**Observed friction:** A stopped, stale-image or partially broken company is easy to create, but the
owner must currently reconstruct the fault and recovery command from CLI output and raw supervisor
state.

## Outcome

Company doctor answers whether Authority, OrgIntel, persistent Runtime storage, the current image,
supervised services and browser/desktop are healthy, why a check is degraded and which bounded repair
is available. Company computer remains an uncluttered, full-canvas entrance to the persistent desktop.

## Acceptance

- The owner view and `restless doctor -c <company>` agree on Runtime facts.
- Unknown never renders as healthy and an unavailable source never renders as an empty inventory.
- Start, restart and reconcile state their consequence before execution and preserve the company
  volume/browser profile.
- Repair is re-probed; the resulting lifecycle receipt and observed state agree.
- The doctor remains a composition of existing checks, not a scheduler or workflow engine.
- Diagnostics and recovery do not compete visually with the centered **Enter computer** action.

## Deletion

Makes owner-facing raw supervisor instructions and ad hoc Runtime status copy deletable.
