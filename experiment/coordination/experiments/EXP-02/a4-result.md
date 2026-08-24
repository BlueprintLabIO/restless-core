# EXP-02 A4/P5 — recovery capsule result

**Disposition:** provisional-loss as an efficiency affordance; retain only the minimum factual product
recovery boundary.

**Valid run:** `exp02-a4-p5-r2` on 23 August 2026.

## Invalid launch

`exp02-a4-p5-r1` ended before model work because `codex exec` rejected the simultaneous
`--approve-for-me` and explicit `--sandbox workspace-write` flags. The launcher removed only the
redundant sandbox flag and restarted under a fresh ID. This is infrastructure-invalid, not evidence
about either arm.

## Frozen comparison

- Arm order: A4 then R0 from seed
  `EXP-02:A4:P5:R1:gpt56:2026-08-23` (SHA-256
  `6e3f5a9796457b260bf655b7655fd427c1c7a37fd45f57755a77254c3f5e927d`).
- Lead: `gpt-5.6-sol`; blind reviewer: `gpt-5.6-terra`.
- Both arms inspected byte-identical producer candidate
  `9748029525d1df6aacdb6105b75fb94ac3cbc8843b7a9294709d2005b75de0c0`.
- R0 received generic `unknown + workspace`; A4 additionally received exact Work/Attempt/actor,
  start/end Git observations and a worktree evidence handle.
- No semantic timeout, producer rerun, network source or consequential effect occurred.

## Native outcome

Both leads:

- preserved the producer outcome as unknown;
- inspected the actual dossier and frozen Q01–Q08 records;
- detected the unsupported “80% of the market” claim;
- chose the bounded `revise` action;
- left the producer artifact byte-identical; and
- launched no duplicate producer.

Fresh artifact-only review scored A4 9.8/10 and R0 9.2/10, preferring A4 because its candidate digest
matched the reviewed artifact and its actor/Work/Attempt provenance was more reproducible. The reviewer
also noted that A4's external workspace/commit assertions were not all independently reviewable from
the submitted files.

## Cost and churn

| Measure | R0 generic | A4 capsule | A4 difference |
|---|---:|---:|---:|
| Wall time | 55.342s | 61.507s | +11.1% |
| Input tokens | 94,311 | 115,329 | +22.3% |
| Cached input tokens | 76,288 | 95,488 | +25.2% |
| Newly processed input | 18,023 | 19,841 | +10.1% |
| Output tokens | 1,663 | 1,908 | +14.7% |
| Tool calls | 5 | 6 | +20.0% |

Both arms found the same file and defect without reimplementation. A4's extra tool established an
exact candidate hash and stronger provenance; it did not reduce discovery, wall time, tools or token
cost.

## Decision

The frozen rule required non-inferior quality **and** materially lower targeted recovery churn. A4
improved provenance quality but failed the churn condition, so it does not earn cross-shape
replication or promotion as a richer lead-context mechanism.

Sprint 12 still needs the deterministic floor that made both arms possible: preserve the workspace,
keep outcome unknown, link the notice to the exact Work/Attempt, address the accountable lead and
expose artifact/process observations on inspection. Product code should keep that notice compact. It
must not grow into a recovery checklist or inject all observations eagerly merely because the facts
exist.

## Evidence locators

- ignored conformance result:
  `experiment/coordination-lab/v2/workdir/exp02-a4-conformance-r1/a4-conformance-results.json`;
- ignored valid result:
  `experiment/coordination-lab/v2/workdir/exp02-a4-p5-r2/a4-run-results.json`;
- ignored blind review:
  `experiment/coordination-lab/v2/workdir/exp02-a4-p5-r2/blind-review/candidate/blind-review.md`;
- decision SHA-256: A4
  `19a4ab672ebcc182c2722eec2fda2a9fefa4c565741709c7f309e708616ac72c`, R0
  `3a332de1d7d8ceed5b39ef13bc7430d71e4e962acbe38a0f22c62ae5803c1b11`.

## Learning

A strong lead plus a self-describing persistent workspace already recovers surprisingly well. The
high-value deterministic ingredient is **preservation and exact attribution**, not a verbose recovery
brief. Supply a compact proof handle; let the lead inspect the native artifact.
