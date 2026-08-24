# S15-T4 — Finish command-domain transport decomposition

**Layer:** Daemon transport.

**Observed friction served:** Sprint 14 physically grouped fields but retained a flattened all-domain
payload and a nearly 2,000-line dispatcher. Invalid field combinations remain representable.

## Outcome

The stable JSON-line shell routes each command through a small typed domain input and handler.

## Acceptance

- Common authenticated envelope stays narrow; lifecycle/authority, OrgIntel and owner command inputs
  decode in their own domain view.
- A command rejects fields outside its view rather than silently accepting an all-domain bag.
- Dispatch modules own their current command families while preserving the existing CLI wire names.
- No universal command enum, new service, generic mutation writer or duplicate organisation state.

## Deletion target

Flat domain-field bag and duplicated CLI/daemon command-shape knowledge.
