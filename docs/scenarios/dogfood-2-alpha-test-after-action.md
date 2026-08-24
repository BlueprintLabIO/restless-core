# Dogfood 2 — alpha-candidate test-world after-action

**Status:** Completed; verdict is inconclusive

**Scenario:** [`dogfood-2.md`](./dogfood-2.md) v0.1
**Company:** `robotics_ai_alpha_test`
**Run date:** 24 August 2026
**Run mode:** isolated `_test` historical evaluation; never live market research

## Result

Restless captured a frozen pack of real public historical daily rows, then ran the predeclared
60-trading-day relative-strength candidate twice from that same pack. The two JSON outputs were
byte-identical:

`6c2f65b45a9a1161100a61cfe56662583e3b367c7cf54e779201709535ffd041`

The evaluator returned **inconclusive**. Although its arithmetic showed positive cost-adjusted
out-of-sample excess versus its broad-market comparison, the source pack cannot establish
point-in-time listing eligibility, delisting/survivorship coverage, corporate-action adjustment or
index membership. The result is therefore not alpha evidence and must not enter the live Dogfood 2
company's dossier.

## Observed run record

| Fact | Observed evidence |
| --- | --- |
| Frozen contract | `/company/outputs/robotics-ai-alpha-test/alpha-candidate-contract.json`; cutoff `2025-12-31`; universe AI, AUR, PATH, RKLB, SOUN and SYM; benchmark SPY |
| Raw sources | Seven NASDAQ historical-response files in `/company/outputs/robotics-ai-alpha-test/raw/`, captured at `2026-08-24T07:49:05.229Z` |
| Evidence manifest | `/company/outputs/robotics-ai-alpha-test/source-evidence-manifest.json`; seven `available_public` records plus controlled `rate_limited`, `unavailable` and `unverified_provider` records |
| Source-manifest probe | The scenario validator passed, preserving all four access states rather than flattening them to a connection flag |
| Evaluation artifacts | `/company/outputs/robotics-ai-alpha-test/evaluation.json` and `evaluation.md`; a second invocation wrote `repeat/evaluation.json` with the identical hash above |
| Runtime/browser probe | `restless doctor -c robotics_ai_alpha_test` reported the coordinator, OrgIntel, owner gateway, cockpit APIs, persistent runtime and browser transport available |

## Declared output

| Partition | Valid periods | Gross return | Cost-adjusted return | Benchmark return | Cost-adjusted excess |
| --- | ---: | ---: | ---: | ---: | ---: |
| In-sample | 16 | -14.44% | -19.76% | 13.83% | -33.59% |
| Validation | 11 | 382.90% | 362.08% | 23.55% | 338.52% |
| Out-of-sample | 11 | 29.42% | 23.84% | 17.05% | 6.78% |

These figures are a deterministic read of the declared frozen files, not an investable return claim.
They are recorded to make the candidate falsifiable and to show why its positive-looking portions do
not override the predeclared inconclusive conditions.

## What worked

- A small Runtime-local contract, raw inputs, input manifest and evaluator were sufficient; no signal
  language, provider adapter or backtest service was introduced.
- The evaluator verifies the contract and raw-file hashes before calculating. A missing historical
  volume is treated as an unknown that disqualifies that liquidity window, not as zero or usable data.
- Available public sources and controlled degraded-route states are inspectable in the same manifest.
- Repeating the calculation from the frozen pack produced an identical result.

## What this did not prove

- It did not prove alpha, validate the current Dogfood 2 universe, or provide a current price,
  valuation or trading conclusion.
- It did not establish a live authenticated provider lane, create a provider account or use a
  credential.
- It did not address survivorship, factor exposure, original listing dates, historical market
  capitalisation, corporate actions or point-in-time index membership well enough for an investable
  interpretation.

## Next informative action

Keep this test-world artifact isolated and run the live Dogfood 2 dossier using only current
inspectable public sources while the existing provider owner handoff remains unverified. The live
report may offer a direct non-personal Speculative Buy, Watch or Avoid stance only where its current
evidence supports one; it must label the provider lane and any source gaps explicitly.
