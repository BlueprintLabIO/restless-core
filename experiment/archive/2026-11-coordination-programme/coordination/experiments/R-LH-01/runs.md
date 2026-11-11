# R-LH-01 run index

Status: counted B1 → B0 pair complete; ordinary B1 provisional loss

Infrastructure preflights and workload-classification sessions are not counted arms.

- First counted Run ID: `exp01-e02-b1-terra`.
- Matched Run ID after structurally valid B1: `exp01-e02-b0-sol`.

## Counted result

- B1: protocol valid, clean candidate `5b0a5c5cdddd6cf8e14bc8309f3b956c8f1af25e`, 647.7 seconds,
  2,611,420 summed turn tokens, 48 tools, external evaluator 27/36.
- B0: protocol valid, clean candidate `e2ee3cb5899445f4949f2cb8756dcc95dca59fb5`, 569.0 seconds,
  1,102,444 tokens, 25 tools, external evaluator 28/36.
- Blind artifact review: B1 9.1, B0 8.9; narrow B1 preference, insufficient to outweigh either failed
  gate.
- B1's worker did not receive the frozen source cards, substituted game-repository evidence, and had
  its apparently clean ledger rejected. The lead rebuilt the ledger completely. Codes: `C1`, `I3`,
  `E5`, plus shared `V4`.

See [`../EXP-01/e02-ordinary-frontier.md`](../EXP-01/e02-ordinary-frontier.md) and
[`blind-review-exp01.md`](blind-review-exp01.md).
