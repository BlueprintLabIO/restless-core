# Sprint 02 — Three real runs, and the platform the runs already demanded

**Status:** DRAFT — written mid-sprint-01 from machinery-level evidence and the observed friction
backlog. **Finalize only after sprint-01's T11–T15 complete**; the run report
([sprint-01/run-report.md](./sprint-01/run-report.md)) is the evidence base this spec finalizes
from, and its PENDING sections may reorder everything below.
**Date:** 13 August 2026 (draft)
**Architecture refs:** §16.4 (platform work originates from the friction backlog), §16.5 (cadence),
§17 steps 3–4, §10.6 (smoke-test scenarios), §14 open questions 12 (dogfood company)

---

## Outcome (draft)

> **Cosmon, Aris, and Thymelake each complete one full owner-directive-to-artifact run on the
> sprint-01 machinery, surviving the crash harness — and the three platform failures the sprint-01
> build already observed are fixed: honest provider-failure surfacing, resource-exhaustion
> visibility, and boot-time company reconciliation.**

Sprint 01 built the skeleton and verified every property that does not require burning model credit.
It did not run a company end-to-end: the OpenRouter key's limit was exhausted at exactly the moment
the runs were to start. Sprint 02 is therefore not a new slice of architecture — it is the runs
themselves, plus the small set of platform fixes the build phase already proved necessary. If sprint
01's remaining evidence changes that picture, this spec changes with it.

### What this sprint is *for*

The sprint-01 spec stacks four claims (substrate / ontology / autonomy / negative claim). Sprint 01
settled the substrate mechanically and produced machinery-level evidence for the rest. Sprint 02
produces the missing evidence class: **behavioural** — what the Exec and Staff actually do across
multi-turn runs, three company shapes, and induced failures.

## Proposed ticket list (draft, for founder alignment)

Ticket files to be written once sprint-01 T15 lands. Each cites its friction-backlog item or the
sprint-01 carry-over it serves (§16.7).

### Carry-over: the runs themselves

- **S02-T1 · Cosmon run** (carries sprint-01 T11). Acceptance unchanged: owner directive → playable
  browser game loop, elapsed/cost/interventions recorded.
- **S02-T2 · Aris run** (carries T12). Acceptance unchanged, plus: persona adversarialness is the
  point — ghosts and non-converting price objectors must actually shape the Exec's behaviour.
- **S02-T3 · Thymelake run** (carries T13). Acceptance unchanged; the malformed early order is the
  evidence to watch.
- **S02-T4 · Crash and restart harness** (carries T14). All three interruptions on a real run, no
  committed work lost. The scenario driver rewrites against the current CLI.

### Platform slice: what the sprint-01 build already proved necessary

- **S02-T5 · Honest provider-failure surfacing** (friction F1). Deterministic classification of the
  provider error channel (status → quota/auth/rate/unknown) as a first-class wake outcome — never
  "unparseable termination" again — plus dedup/backoff on identical blocked-notifications to the
  owner. Kernel layer: this is the gateway/daemon boundary telling the truth. Acceptance: a 402
  replay (fixtures exist) produces one precise owner-visible message naming the cause, and zero
  duplicate mails across repeated wakes.
- **S02-T6 · Resource-exhaustion visibility** (friction F2). `restless status` reports host disk;
  the daemon turns ENOSPC-class failures into blocked-with-reason instead of silent stall. OrgIntel
  layer. Acceptance: fill a scratch filesystem, observe the exec's wake context carry the condition
  plainly. (Deliberately small — a probe and an honest error, not a capacity manager.)
- **S02-T7 · Boot-time company reconciliation** (friction F3). `restlessd` boot starts companies
  that should be running (it already sweeps orphans there; same shape). Runtime layer. Acceptance:
  restart Docker Desktop with three companies up; all three resume without owner action; the
  orphan sweep still blocks what cannot resume. **This makes the F7 expiry audible:** if companies
  auto-start, the trusted-as-sent TCP identity (F7) needs its named-risk review pulled forward —
  decide explicitly at alignment.

### Held open, deliberately

- **T16 judgement helper stays unbuilt.** The sprint-01 smell-family grep is clean; all three of its
  named call sites resolved to the model judging directly. It remains the standing rule: the first
  daemon-internal judgement call that appears in a run builds `judge!` through the gateway, not a
  heuristic. If S02-T1..T4 surface such a call, that ticket joins the sprint mid-flight — that is
  the mechanism working, not scope creep.
- **Exec mail read-management (F6)** — observe in the runs before choosing machinery vs playbook.
- **Codex sandbox friction (F10)** — characterize during S02-T1; fix only what the run proves.

## The primary measurement (carried)

Sprint 01's primary measurement — the marginal cost of companies 2 and 3 — was answered only for
platform cost (near zero: schemas + personas). Sprint 02 answers it for *run* cost: does Aris need
anything Cosmon didn't? Does Thymelake? The run report's per-company table is the deliverable.

## Acceptance (draft)

1. Three completed runs with elapsed/cost/intervention data in the run report.
2. The crash harness passes on a real run with no committed work lost.
3. A repeated provider-402 drill produces exactly one precise owner message.
4. The negative-claim assessment in the run report is finalized with run evidence — including an
   explicit verdict on whether the `spawn` envelope stayed a single verb.
5. The deletion pass executes: every code path no run exercised is removed before sprint 03 opens.
6. This spec is rewritten from draft to final using the completed run report, and the diff between
   draft and final is itself recorded — it measures how much of sprint planning was predictable from
   machinery-level evidence alone.

---

*Sprint 01 spec: [sprint-01-walking-skeleton.md](./sprint-01-walking-skeleton.md). Friction backlog
and run evidence: [sprint-01/run-report.md](./sprint-01/run-report.md).*
