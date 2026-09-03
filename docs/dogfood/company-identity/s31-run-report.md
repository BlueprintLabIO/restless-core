# Sprint 31 run report — source-owned Company Identity kernel

**Run date:** 31 August 2026
**Company:** `company_identity_test`
**Control:** `company_identity_control_test`
**Corpus SHA-256:** `55900fd6ffa69384e16367ed54f65bf3f6ca6fa80c64d7f799b12cee06be4627`

## Accepted release

- Release: `c81bf5ec-eb98-4642-ad0e-3ac43da66e17`
- Authority record: `authority:1467`
- Evidence: four source-owned statements, one in each typed pillar
- Compiled brief digest: `ca09d60fa4ff13568d0771ceb9bd76b299cc106af8315f7028d33e020dfe678c`
- Compiled size: 920 bytes; omission account: `none`

The owner promoted the proposal through Company → Identity. Refresh and a real daemon restart returned
the same release, evidence set, Authority reference and lineage. The CLI compiled the same bounded brief
from the persisted release after restart.

## Contract evidence

The live Postgres integration scenario proved:

- a legacy company starts with no invented identity;
- conflicting claims fail closed;
- a non-owner cannot promote a proposal;
- an owner promotion atomically creates the immutable release and current pointer;
- two different Work nodes bind the same effective release across restart;
- a correction creates a successor release, preserves historical bindings and marks affected earlier
  bindings stale;
- scope filtering, deterministic ordering, byte bounds and omission accounting survive round-trip.

The full Rust workspace suite passed against live Postgres. The Svelte owner surface passed type, static
analysis and production build checks. Browser review found one real narrow-canvas defect: the Company
sub-navigation and Exec rail left less content width than the viewport breakpoint assumed, collapsing a
proposal heading word by word. The surface now uses its own container width and stacks its ledger,
proposal and evidence layouts at the actual content boundary.

## Matched Restless output gate

The control and treatment used the same company mission, model, reasoning effort, outcome standard,
budget and exact owner task. The treatment alone had the released Identity Brief. No comparable copy
output was accepted because the model plane did not produce a valid pair:

1. offset `300` collided with a Docker Desktop loopback listener; company containers reached the wrong
   service through `host.docker.internal` and received a 404;
2. Moonshot's catalogue described Chat Completions while Restless advertised Responses in the Runtime
   model contract;
3. after both harness defects were repaired, the configured Moonshot account returned the same upstream
   server error for K3, K2.5 and the stable v1 model;
4. the configured ZAI account reported its weekly/monthly quota exhausted; and
5. the prior Codex OAuth credential had already been disabled by an older daemon configuration, so it
   was not silently substituted.

This gate is **provider-blocked, not a tie and not a quality pass**. Restless recorded identical failed
attempts on both arms. No output was invented, copied from a direct model call or credited to Company
Identity.

## Adjustments made from the run

- The Runtime relay now probes the exact loopback alias before starting. A collision fails immediately
  with a `RESTLESS_PORT_OFFSET` repair instead of booting a false-green plane.
- Moonshot is emitted as a Chat Completions provider in the Runtime model contract, with regression
  coverage alongside the existing GLM route rule.
- Comparative dogfood remains a terminal gate for the programme. It must be rerun when one admitted
  provider is healthy; the missing pair remains explicit in this report.

## Quality judgement

The source-owned substrate is materially better than prompt folklore: authority, evidence, lineage,
bounded compilation and historical Work binding are now inspectable and fail closed. The owner surface
is usable and truthful. Sprint 31 does **not** yet prove that the thin four-statement release improves
copywriting quality. That claim remains deliberately unmade until the provider-blocked matched run can
return two real outputs.
