# v11 — real delegation, useful artifact, unsafe false completion

## Change under test

Give every fresh Exec wake the durable owner directive and README from the exact `candidate` commit;
remove native repository tools from Exec; remove `base_ref` from `commission`; keep scoped perception
and production tools with Staff.

Deterministic preflight: Pi harness 7/7; coordination/recovery fault suite 30/30; SQLite quick check OK.

## Evidence

- Seed/candidate: `514b7b3d0a65e093af608b08ca142344412181f4`
- Models, all live-proven zero-price at launch:
  - Exec: `nvidia/nemotron-3-super-120b-a12b:free`
  - world-content: `cohere/north-mini-code:free`
  - artifact-critic: `nvidia/nemotron-3.5-lightning:free`
  - gameplay-systems: `poolside/laguna-s-2.1:free`
- Wall time: 757 seconds before the experiment stopped on the false-completion invariant
- Aggregate recorded usage: 637,370 input/cache-read + 26,773 output tokens; 69 tool calls; $0
- Exec commissioned one bounded reconnaissance Work in one coordination call and quiesced
- world-content produced commit `4913ba76a8142d1e6be125a60fb9c026d7c30ef4`:
  `docs/world-spawn-explanation.md`, 6,255 bytes / 139 lines,
  SHA-256 `a5db670f0954fa26709c9b96a3f43e6a9ab8138e8e8328bc2bc4d672e2d544d3`
- An independent direct `test -s` passes on that exact file
- Its declared gate was malformed as one argv element containing shell syntax. Literal execution
  failed with exit 127, so the useful Work was marked failed and then abandoned
- The critic's first model request failed with an upstream idle timeout before any tool call
- The generic finalisation continuation then inspected only clean Git state and declared the seed
  fully compliant. OrgIntel accepted unchanged base commit `514b7b3...` as a produced review even
  though expected `artifact-critic-review.md` does not exist and the README explicitly lists major
  missing biomes, trainers, boss, exploration abilities, and spacecraft
- Exec subsequently commissioned relevant elemental-world Work, but the model hit an upstream 429;
  its fallback ended without a callback, leaving the Attempt truthfully `unknown` and files preserved
- Durable telemetry was 26,347,003 bytes because non-text `toolcall_delta` records still copied growing
  partial messages

## Score

Raw outcome score: **35/100**. Final score: **29/100** under the false-completion cap.

| Dimension | Points | Evidence |
| --- | ---: | --- |
| Accepted outcome /30 | 10 | exact useful analysis commit exists; no playable game increment or valid declared gate |
| Coordination /20 | 9 | bounded delegation/quiescence worked; later review lacked dependency and discarded useful input |
| Recovery/truth /15 | 2 | useful files survived and gameplay became unknown; critic completion was false |
| Review/evidence /15 | 0 | critic reviewed no native outcome and produced no expected review artifact |
| Efficiency/attention /10 | 4 | no polling or owner intervention; 69 calls and forced continuations were excessive |
| Harness/control /10 | 10 | exact launches, free proofs, scopes, streaming, errors, usage, and stops were recorded |

## Dominant failure

The controller tried to repair missing callbacks by forcing another model turn to “finalise.” After a
provider failure, that continuation had neither completed work nor producer context. Clean Git state
was mistaken for evidence that the outcome was met. The report path also allowed an unchanged base
commit to satisfy new Work.

## Decision

False completion is an invariant failure, so stop the run and simplify:

- remove automatic finalisation turns;
- if an actor exits without a terminal callback, mark the Attempt `unknown`, preserve its workspace,
  and wake Exec for an explicit repair/reassignment decision;
- reject `outcome_met` when HEAD has not advanced beyond the Work input;
- validate gate argv shape before accepting Work;
- retain the useful reconnaissance commit as evidence but do not promote it to candidate.

The next sprint is a focused live recovery probe, not another full game run.
