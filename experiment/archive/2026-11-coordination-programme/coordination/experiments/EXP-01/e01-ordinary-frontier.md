# EXP-01 E01 — C-LH ordinary frontier result

**Date:** 23 August 2026  
**Counted pair:** `exp01-e01r1-b1-terra` → `exp01-e01r1-b0-sol`  
**Disposition:** ordinary B1 loss; no accepted-quality winner

## Outcome

Both arms were structurally valid and produced clean candidates. Both then failed the corrected frozen
external evaluator at the same review-path boundary: 10/53 checks passed and 43/53 failed. Their
starter links used hash targets whose click handlers opened the tools, but loading the exact link URL
into a fresh page did not initialise either tool. The failure cascaded from the unreachable Atlas and
Workshop roots; the preserved game behavior passed.

Fresh blind artifact review scored B0 8.4/10 and B1 9.1/10, modestly preferring B1's presentation and
direct Workshop roster. It explicitly judged that difference too small to outweigh the identical
release-gate failure. Neither arm therefore has an accepted outcome and no speed claim is promoted as
an outcome win.

## Cost and coordination

| Measure | B1 ordinary team | B0 lead alone | Difference |
|---|---:|---:|---:|
| Run wall time | 1,216.4 s | 621.7 s | B0 48.9% shorter |
| Summed turn tokens | 5,491,039 | 1,587,803 | B0 71.1% fewer |
| Tool calls | 76 | 26 | B0 65.8% fewer |
| Actor turns | 4 | 1 | B1 added three turns |
| External evaluator | 10/53 | 10/53 | tie/fail |

B1 commissioned after 23.5 seconds and achieved genuine overlap. Terra produced a useful standalone
stylesheet commit in about 98 seconds, but the lead assigned only presentation CSS rather than the
preclassified independently valuable Atlas seam. The first producer turn committed cleanly but omitted
the terminal report. OrgIntel correctly marked it `unknown`; the lead later found the preserved commit
and one 15-second evidence-based repair reported it successfully. The lead integrated and refined the
stylesheet, then implemented both behavioural tools itself.

B0 implemented both tools directly in one turn. Both arms independently caught and removed eager
cross-tool module coupling. Both also relied on actor-authored click-path checks that missed the frozen
evaluator's fresh-navigation path.

## Attribution and routing

- **S2:** the lead assigned a below-altitude CSS seam despite a known independently valuable Atlas
  vertical slice.
- **E5:** useful specialisation did not repay the additional 594.7 seconds, 3.90 million tokens,
  50 tools, callback repair and integration work.
- **V4:** both arms' declared native proof did not reproduce the external review path.
- No `V3`: the missing terminal report was classified `unknown`, not success, and recovered from exact
  workspace evidence.

The measured `E5` trigger activates W04 (one brain, many hands) for the cheapest live screen after E02.
W07 is not activated by this cell because artifact recovery behaved correctly; W01 and W09 have no
`C1`/`C4` or `M1` trigger.
