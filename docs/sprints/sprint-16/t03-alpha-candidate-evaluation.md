# S16-T3 — Reproduce one alpha-candidate evaluation in a test company

**Layer:** Company Runtime + evaluation.

**Observed friction served:** Dogfood 1 offered actionable research judgment but no historical,
point-in-time evidence for an alpha inference.

## Outcome

A dedicated robotics_ai_alpha_test company runs one declared, reproducible alpha-candidate
evaluation from a frozen pack of real historical inputs and reports its result without contaminating
the live research company's evidence base.

## Acceptance

- Before running calculations, record the universe/eligibility rules, historical data cutoff, data
  availability timestamps, causal hypothesis and exact signal, entry/rebalance/holding rules,
  benchmark, liquidity floor, transaction-cost/slippage assumptions, out-of-sample segment and
  rejection criteria.
- Preserve normal source files and a simple input manifest in the _test Runtime. Use real historical
  data, not fabricated market prices; label all resulting artifacts as test-world outputs.
- Make the calculation deterministic under the frozen pack and reproduce the same output from the same
  declared inputs.
- Report raw and cost-adjusted results, benchmark comparison, known survivorship/look-ahead/factor or
  data-coverage limitations, and supported, rejected or inconclusive for the candidate.
- Never call the outcome alpha proof, attach it as live-company market evidence, or create a generic
  signal language, scorer, portfolio optimiser or backtest service.

## Deletion target

One-off manual return calculations and untestable “quant-like” claims.
