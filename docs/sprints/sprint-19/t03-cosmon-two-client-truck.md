# S19-T3 — Dogfood the Cosmon two-client truck skeleton

**Layer:** Company Runtime plus evaluation.

**Observed friction served:** The proposed multiplayer first-week loop has no reproducible Runtime
proof: agents cannot currently probe the engine, start two actual clients, preserve a network profile,
or hand a reviewer the exact resulting state.

## Outcome

A small Godot fixture uses real ENet server/client processes to complete one crate-through-truck
mission. It is a test-world walking skeleton and delivery-tool proof, not a claim of gameplay quality
or a production game architecture.

## Acceptance

- One server and two distinct client processes connect through a scenario-owned, deterministic delay/
  loss proxy or another observed network profile; the resulting profile and observed proxy facts are
  included in evidence.
- The server report records two peers, crate pickup, driver entry, truck movement, unload and mission
  completion. The assertion fails on a missing/incorrect event rather than accepting client narration.
- The package preserves a compact input trace, process logs, final-state render and server report.
- A Windows export is generated with the installed Godot templates and included in the evidence
  manifest.
- The native review target clearly says this is an isolated technical walking skeleton. A lead must
  judge whether to evolve the game; deterministic checks do not decide fun, polish or market value.

## Deletion target

“Multiplayer works” claims based on one process, mocked client counts, unrecorded network settings or
unreproducible local commands.
