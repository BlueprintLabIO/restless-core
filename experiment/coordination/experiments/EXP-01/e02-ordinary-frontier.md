# EXP-01 E02 — R-LH ordinary frontier result

**Date:** 23 August 2026

**Counted pair:** `exp01-e02-b1-terra` → `exp01-e02-b0-sol`

**Disposition:** ordinary B1 loss; no accepted-quality winner

## Outcome

Both arms were protocol-valid, produced clean one-commit research packs and retained all 32 frozen
source IDs. Neither passed the immutable external evaluator. B0 passed 28/36 checks and B1 passed
27/36. Both missed the same dossier-completeness, exact fact-preservation, primary-direction wording,
tension and artifact-link gates; B1 additionally omitted the exact 2.3× reduced-motion abandonment
ratio from its founder memo.

Fresh artifact-only blind review narrowly preferred B1, 9.1/10 versus B0 8.9/10. B1 had the more
faithful ledger and self-sufficient dossiers; B0 had the stronger operational sequence and stop
conditions, but introduced one unsupported statement about a current battle proof. The reviewer judged
the difference too small to outweigh either failed release gate. There is therefore no accepted-quality
team win.

## Cost and parallelism

| Measure | B1 ordinary team | B0 lead alone | Difference |
|---|---:|---:|---:|
| Run wall time | 647.7 s | 569.0 s | B0 12.1% shorter |
| Summed actor time | 784.0 s | 569.0 s | B1 overlapped 136.4 actor-seconds |
| Summed turn tokens | 2,611,420 | 1,102,444 | B0 57.8% fewer |
| Tool calls | 48 | 25 | B0 47.9% fewer |
| Actor turns | 3 | 1 | B1 added worker and callback turns |
| External evaluator | 27/36 | 28/36 | both fail; B0 +1 check |

B1 commissioned its analyst 19.5 seconds after the request and received a clean 32-row ledger commit
about 161 seconds later while the lead authored the four dossiers and final synthesis. This was genuine
parallel work. The contribution was not usable: the worker had received the Work description and source
IDs but not the frozen source-card corpus that existed only in the lead's request context. It silently
substituted repository README/code/test claims for the source cards and then reported that result as a
frozen-corpus ledger.

The lead inspected the exact commit, detected the semantic substitution, rejected the artifact and
rebuilt the complete ledger from the supplied corpus. That preserved provenance and outcome quality,
but eliminated the delegated work's value and duplicated its implementation.

## Attribution and routing

- **C1:** necessary owner evidence was lost at the Work context boundary; the worker received source
  identifiers without their source cards.
- **I3:** the lead had to rediscover and completely reimplement the delegated ledger after inspection.
- **E5:** 136.4 actor-seconds of overlap did not repay the rejected contribution, 78.6 extra wall
  seconds, 1.51 million extra tokens and 23 extra tools.
- **V4:** both arms' native self-checks passed while the frozen external review contract still failed.

`C1` activates W01 session mitosis. E01's `E5` already activated W04 one-brain-many-hands. W07 and W09
remain untriggered because callback/artifact observation was correct and status narration was not the
bottleneck.

E01 and E02 agree: ordinary B1 did not produce an accepted outcome and cost more than B0. E03 is
therefore **not triggered**. No B1 arm won, so E04 and larger-team testing are **not triggered**.
