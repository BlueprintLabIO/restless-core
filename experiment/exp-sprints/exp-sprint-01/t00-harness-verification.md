# T0 — Experiment area and harness verification

## Observed friction

The experiment tree moved from `scratch/` to `experiment/`, and the interrupted v28 run proved that a
clean artifact can survive a missing callback. Starting cognitive comparisons before proving the new
paths, ignored state, first-party session route and artifact recovery would confound coordination with
harness failure.

## Outcome

A clean checkout can run the deterministic substrate from `experiment/`, generated state stays
ignored, Exec dispatch/availability remains observable, and one Terra producer completes a bounded
tool-plus-terminal-handoff probe whose exact artifact is visible to the lead.

## Ownership

Experiment Runtime/harness. This ticket changes only the measurement substrate; it does not add a
production OrgIntel workflow or team router.

## Acceptance

- Storage and credential preflights are recorded without exposing secrets.
- Static path checks, deterministic architecture/fault checks and SQLite integrity pass.
- One first-party Terra feasibility probe performs real workspace work and returns an observable
  terminal artifact handle.
- Exec is available after dispatch and can accept an independent second request.
- No counted E01/E02 arm starts until this ticket closes.

## Deletion

Remove obsolete path shims, provider-specific launch assumptions and any fixed semantic timeout found
on the counted path. Preserve historical reports and ignored recovery state.
