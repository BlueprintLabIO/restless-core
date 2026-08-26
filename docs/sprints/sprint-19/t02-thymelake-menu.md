# S19-T2 — Dogfood a non-coding Thymelake menu launch

**Layer:** Company Runtime plus evaluation.

**Observed friction served:** A coding-only harness can look general while silently assuming source
code, compilation and test suites are the only meaningful evidence.

## Outcome

A controlled menu-source package produces a validated configuration, a QR-ordering preview and a
human-readiness target without treating generated prose or a green file write as operational success.

## Acceptance

- The package uses ordinary menu/source files and no application-code build as its primary production
  step.
- Its deterministic validation catches duplicate item identifiers, invalid prices and source conflicts.
- Missing allergy/price facts remain explicitly unknown or blocked; the package cannot choose a value
  to make validation pass.
- The run collects source, normalized configuration, validation report, preview HTML and a screenshot
  or render as ordinary evidence paths.
- The review note asks for human operational/visual judgment and says it is a controlled test-world
  configuration, not a restaurant launch or customer result.

## Deletion target

Prose-only “menu readiness” claims and game-specific assumptions in a supposedly reusable scenario
loop.
