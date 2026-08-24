# S15-T3 — Make model ceilings exact and finite

**Layer:** Authority.

**Observed friction served:** `spend_ceiling_usd` is an unvalidated `f64`; `inf` parses and saturates
to `u64::MAX` in the hard-ceiling decision.

## Outcome

The model ceiling has one exact micro-USD representation at config, CLI and enforcement boundaries.

## Acceptance

- Reject non-finite, negative and unrepresentable ceiling values from TOML and CLI mutation paths.
- Convert a valid owner-facing decimal to exact micro-USD once, then compare integer values.
- Preserve sensible owner display in USD without using a float to make an authority decision.
- Focused checks prove `inf`, `NaN` and negative values are refused and a valid boundary blocks.

## Deletion target

Float saturation from the authority decision path.
