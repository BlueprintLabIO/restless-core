# Outcome: maintain the strategy decision ledger across changing evidence

You own `DECISION_LEDGER.json` for the strategy lead. Read `DECISION_CONTRACT.md` and every source in
`signals/initial.json`. Create a source-backed initial ledger that is directly usable for the three
named decisions. Preserve explicit unknowns and never turn simulated evidence into certainty.

Run `node evaluate-visible.mjs`, repair all failures and write `RESULT.md` with the current decision
state, evidence handled and exact validation result. Further signals may arrive later; update the same
ledger rather than creating summaries or parallel copies. Do not inspect paths outside this fixture,
use network access or perform any external effect.
