# Sprints

Restless is built in sprints. The cadence (see `CLAUDE.md` → "How we work"):

> `ARCHITECTURE.md` (target) → **sprint spec** (founders align) → coding agents break it into tickets
> → founders align on tickets → implement as a goal-mode sprint on `dev`.

## Sprint specs

Each sprint is one file: `sprint-NN-short-slug.md`. A complete spec states:

- **Outcome** — the useful artifact, decision, or external outcome this sprint produces. A schema, API,
  or invariant suite alone is not a successful sprint (ARCHITECTURE.md §16.2).
- **Acceptance criteria** — how we judge it, headlessly where practical, with stated inputs and
  expected results (CLAUDE.md → "Verifying").
- **Slice per layer** — which Kernel / OrgIntel / Runtime concerns this sprint touches, and what is
  deliberately out of scope.
- **Tickets** — one file per ticket under `sprint-NN/`, indexed by a status checklist in the spec. Each
  cites the observed outcome or friction it serves, names its layer, and states what prior machinery —
  if any — it makes deletable (ARCHITECTURE.md §16.7). Tickets are files rather than GitHub Issues
  because agents are their primary readers: a file costs no tool call, is versioned with the code that
  implements it, and is reviewed in the same PR the founders align on. **Status lives only in the spec
  checklist** — one canon, not two.
- **Salvage** — which `docs/SALVAGE.md` lifts (if any) the sprint uses, each with its re-validation step.

## Working against the target architecture

- **Observe before modelling.** Grow entities, state machines, services, and protocols only after
  repeated real scenarios reveal the same need (§16.1, §16.6).
- **Slices before layers.** One effort owns all three layers end to end for the walking skeleton (§16.9).
- **Deletion is progress.** Each sprint identifies abstractions, adapters, tables, protocols, and tests
  that no longer improve a live outcome, and removes them (§16.5).
- **No speculative generality / no feature parity** with the prior implementation (§16.3, §17).

## First sprint

[`sprint-01-walking-skeleton.md`](./sprint-01-walking-skeleton.md) — draft, awaiting founder alignment.

A thin end-to-end skeleton of all three layers, running Cosmon, Aris and Thymelake from one directive
each. Governance is deliberately out; the skeleton is built against Cosmon and the other two are added
as configuration, so that **the cost of adding companies 2 and 3 measures whether the abstractions are
real or overfit**. This is ARCHITECTURE.md §17 step 2 with §17 step 5 pulled forward.

Note: this supersedes the single-outcome framing in `docs/SALVAGE.md` → "First outcome target", which
proposed Cosmon alone. Cosmon remains the company the skeleton is built against and the only one with a
hard green requirement.

## Current alignment drafts

- [`sprint-11.md`](./sprint-11.md) — trustworthy delegated execution from ACP control package through
  native owner evidence.
- [`sprint-12.md`](./sprint-12.md) — implementation evidenced: recoverable natural-team execution,
  including a real Cosmon outcome; connected-browser owner-surface visual sign-off remains open.
- [`sprint-14.md`](./sprint-14.md) — completed Rust consolidation, evidence integrity, and model-spend
  hardening; Sprint 12's connected-browser sign-off remains open.
- [`sprint-15.md`](./sprint-15.md) — active: trusted Runtime capability boundary, scoped model access
  and release-evidence repair.
- [`sprint-16.md`](./sprint-16.md) — draft: one research evidence-to-decision loop, a falsifiable
  alpha-candidate test in _test, and Dogfood 2 as its real acceptance run.
- [`sprint-17.md`](./sprint-17.md) — implementation complete: non-producing supervision, scoped hot
  sessions, durable inbound truth and a paired native owner review are proved; the founder explicitly
  deferred the remaining real-provider `_test` callback validation.
