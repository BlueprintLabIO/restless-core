# S19-T0 — Freeze the scenario envelope and Runtime baseline

**Layer:** Company Runtime plus evaluation.

**Observed friction served:** Dogfood 2 can reproduce one research evaluator, but agents still need to
invent package layout, evidence handling and tool probes for each materially different outcome.

## Outcome

One compact, file-owned scenario contract is written before the runner exists. It states exactly what
the runner can prove and what remains a lead/owner judgment.

## Acceptance

- A package contains a versioned JSON manifest, optional native phase commands, declared capability
  probes, evidence paths and a native review target.
- The contract explicitly distinguishes `blocked`, deterministic verification and human acceptance.
- No required command represents planning, Work state, retries, scheduling, staffing, effects or
  semantic completion.
- The implementation begins from observed Runtime facts: Node, Chromium, Docker and ffmpeg are
  available; Godot is not available until this sprint installs/probes it.
- A bad package and missing required capability have focused negative checks.

## Deletion target

Implicit command folklore and every one-off assertion that an installed host tool must exist inside a
Company Runtime.
