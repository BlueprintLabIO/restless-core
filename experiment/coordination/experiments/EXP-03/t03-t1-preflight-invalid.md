# T03 — T1 excluded preflights

**Disposition:** infrastructure-invalid; excluded from EXP-03 comparisons
**Date:** 24 August 2026

Two launches revealed experiment-infrastructure defects before the first counted T1 arm.

## `exp03-t1-s1t-glm53-r1`

- The command omitted `--scenario-file`; the manifest recorded the full company mission hash
  `a81b6171abd6c7ba37b8b2145f3c1a10ed9166769ec3c62aee2158a8d6f48808`, not frozen T1 hash
  `44699970e702693657ecd59c66515eb804591446404167c8f896b3e5e275ab32`.
- The supervisor reasonably scoped the larger mission and commissioned one larger Work. This is not
  evidence of supervisor drift because it was given the wrong directive.
- The worker selector was then incorrectly rewritten from `zai/glm-5.3` to
  `openrouter/zai/glm-5.3`. It produced no model tokens and no artifact.
- The run was stopped and excluded before repair or outcome evaluation.

## `exp03-t1-s1t-glm53-r2`

- Provider-selector preservation was fixed and deterministically checked.
- The command again omitted the scenario file. The manifest exposed the same wrong hash.
- The run was stopped before any Work was commissioned and excluded.

## Systematic corrections

1. Qualified `zai/` and `openrouter/` selectors are preserved at every actor boundary.
2. `prepare` accepts expected scenario and evaluator hashes and fails before a run on any mismatch.
3. The negative check reproduced the original omission and was rejected with the exact expected and
   observed hashes.
4. A counted launch is not permitted until its manifest also confirms mode, actors, models, event
   policy and frozen hashes.

The lesson is broader than this cell: filenames in a command are intention; a content hash in the
run manifest is evidence. Experimental inputs need a fail-closed identity check at launch.
