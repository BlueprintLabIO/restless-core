# EXP-02 P4 — independent critic result

**Disposition:** critic arm loses this matched cell; retain criticism as a judgement option, not a
standing step or new mechanism.

The frozen candidate `b61902eb3ebfac660262171a4cd5ca60c1a1dfe1` passed its declared P1 journey
but fresh native evidence contained a battle-proof failure and a missing 390×844 interaction prompt.
Both arms received the same contract, complete artifact and native evidence. Arm order was C1 then B0
from seed `EXP-02:P4:B0-C1:gpt56:2026-08-23`.

## Result

- B0 Sol alone found both planted failures, severity-ranked them and selected revision.
- C1 Terra found both, then Sol independently inspected the artifact, corrected the critic's
  overstatement that one failed combat proof established a broken whole battle loop, and selected
  revision.
- Fresh blind review preferred B0, scoring its final review 6/10 versus C1 5/10. C1 was better
  calibrated on combat but its final report depended on critic context not present in the blind review
  packet and dismissed a disputed generated-evidence concern too strongly.

| Measure | B0 Sol review | C1 Terra + Sol | C1 difference |
|---|---:|---:|---:|
| Both planted failures found | yes | yes | tie |
| Correct final action | revise | revise | tie |
| Fresh blind final-review score | **6.0** | 5.0 | C1 lower |
| Sequential wall | **227.5s** | 454.2s | +99.7% |
| Total input tokens | **1,876,015** | 2,457,051 | +31.0% |
| Cached input tokens | **1,736,704** | 2,238,720 | +28.9% |
| Newly processed input | **139,311** | 218,331 | +56.7% |
| Output tokens | **7,464** | 15,325 | +105.3% |
| Tool calls | **18** | 29 | +61.1% |

The clean negative-control critic did not invent either planted battle or phone defect. It instead
found a real previously missed contract issue: the accepted natural P1 artifact omits
`bridge.text` from its fresh serialised snapshot because the field is undefined until cavern entry.
That counterevidence is retained; a prior 18/18 label did not make the artifact universally clean.

## Decision

When the accountable strong lead receives the same complete runnable artifact and native evidence, a
mandatory independent critic duplicated discovery, doubled wall time and introduced claims the lead
had to unwind. This does not reject critics where they have genuinely independent evidence, taste,
domain expertise or an adversarial mandate. It rejects a standing critic step and any new critic
protocol for ordinary review.

## Evidence locators

- run result: ignored
  `experiment/coordination-lab/v2/workdir/exp02-p4-r1/p4-results.json`;
- B0 and C1 final reviews: ignored under that run's `b0/` and `c1/` candidate directories;
- blind review: ignored
  `experiment/coordination-lab/v2/workdir/exp02-p4-r1-blind/candidate/blind-review.md`;
- blind review SHA-256:
  `0da1fb70896be0ad6df59844689901fc3d3302bcb91b47854e769d58cde2b544`;
- negative-control review SHA-256:
  `f073e91428e7610dc82672c4b9e67809b0106d7f7303580584b1d0d7ca6b6eb7`.
