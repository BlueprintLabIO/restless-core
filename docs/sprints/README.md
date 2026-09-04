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
- [`sprint-18.md`](./sprint-18.md) — ready for founder alignment: Dogfood 3 asks Restless to improve
  itself through one beta-ready release while independent demand, a real inbound `_test` event,
  changed requirements, terminal failure and process replacement test the whole company.
- [`sprint-19.md`](./sprint-19.md) — implementation complete: an ordinary scenario/evidence envelope,
  pinned Godot delivery lane, and controlled game plus non-coding `_test` dogfoods give agents a
  repeatable tool-discovery, execution and review path without adding workflow state.
- [`sprint-20.md`](./sprint-20.md) — draft: Dogfood 4 uses the founder-accepted Restless site and a
  writer, independent peer reviewer and non-producing publication lead to turn the complete experiment
  record into a rigorous native research publication.
- [`sprint-21.md`](./sprint-21.md) — draft with an EXP-09 activation gate: Dogfood 5 operates the
  research publication through real change, correction and no-op cycles with bounded standing
  authority and no per-cycle owner prompting.
- [`sprint-22.md`](./sprint-22.md) — owner-rejected revision: added ground-truthed web review and a
  mechanically complete Cloud candidate, but missed product-language fidelity, visual density and
  Blog measure.
- [`sprint-23.md`](./sprint-23.md) — implementation and independent critique complete; Bridge Light is
  now the public site's source of truth, with product-native graphics, settled motion and
  publication-grade Blog layouts awaiting owner review.
- [`sprint-24.md`](./sprint-24.md) — draft: Dogfood 6 builds a complete motion-first Svelte interaction
  foundry from a frozen Cult UI catalogue, applies a restrained subset to the Cloud site and makes
  every Blog article genuinely standalone.
- [`sprint-25.md`](./sprint-25.md) — active: separates the owner account plane, per-company cells and
  host fleet so credentials, company state and container lifecycle no longer share one failure domain.
- [`sprint-26.md`](./sprint-26.md) — complete: Attempt lineage, workspaces, native resources, gates,
  feedback, supervisory wakes, promotion and review evidence passed the integrated unattended fixture.
  Its spec prose is a 30 August 2026 reconstruction after the original draft was destroyed; the eight
  ticket files are original and authoritative where they differ.
- [`sprint-27.md`](./sprint-27.md) — planned: gives the account plane a supported network entry by
  verified identity assertion, and publishes a pinnable, self-identifying release. Independent of
  Sprint 26; gates every Restless Cloud sprint.
- [`sprint-28.md`](./sprint-28.md) — draft: makes Attention, Work, artifacts and consequential agent
  communication reader-friendly through source-backed semantics, accountable authorship, structural
  validation and deterministic presentation without forcing ordinary conversation into one template.
- [`sprint-29.md`](./sprint-29.md) — draft: gives every commissioned outcome one inherited ambition
  standard, makes the Exec and accountable lead adapt quality enforcement to consequence, and exposes
  one sane owner control plus honest frontier and limit reporting.
- [`sprint-30.md`](./sprint-30.md) — complete: preserves Exec/lead accountability without routine lead
  model tax, localises concurrent settlement, proves immutable review access before spend, exports
  honest decision telemetry and verifies terminal cleanup in one adversarial coherent/parallel fixture.
- [Company Identity programme](./company-identity-programme.md) — completed five-sprint sequence for one
  source-owned Company Truth, human Voice, native Visual Language and observable Culture, culminating
  in an executable constitution with approved learning and concrete drift control; the all-pillar
  Restless package and held-back Harbour Ledger transfer were independently accepted.
- [`sprint-31.md`](./sprint-31.md) — implemented: establishes immutable identity releases, owner-governed
  promotion, evidence provenance and one bounded outcome-specific Identity Brief.
- [`sprint-32.md`](./sprint-32.md) — implemented: produces human, factually stable and recognisably related
  writing across founder email, newsletter, support, transactional email, product UI and Blogs without
  homogenising named authors.
- [`sprint-33.md`](./sprint-33.md) — implemented: makes product-grounded composition, typography, imagery,
  motion and reusable primitives a versioned Visual Language verified in each native channel.
- [`sprint-34.md`](./sprint-34.md) — implemented: turns observed decisions, dissent, uncertainty, corrections
  and customer conduct into bounded Culture evidence without slogans, worker scoring or surveillance.
- [`sprint-35.md`](./sprint-35.md) — complete: integrates the four
  source-backed pillars, binds Work and artifacts to exact releases, and proves approved learning and
  concrete drift through accepted Restless and Harbour Ledger native release packages.
- [`sprint-36.md`](./sprint-36.md) — Core implemented and verified: one exact artifact can become a
  bounded, time-limited HTTPS/WebSocket or UDP service without exposing the company Runtime. The
  released v1 corpus passes Cloud compatibility; Cloud 14 still owns public ingress, real ENet and
  external-player acceptance.
- [`sprint-37.md`](./sprint-37.md) — draft for founder alignment: productises bounded publication into
  a prepared network experience through Restless-owned HTTPS/WSS and UDP endpoints, scoped admission,
  isolated service workloads and native review targets. Runtimes remain private; Cloud 14's external
  transport gates must pass before this sprint can claim the wider access fabric.
- [`sprint-38.md`](./sprint-38.md) — draft for founder alignment: installs one stable local Restless
  appliance, separates daily use from development, recovers schedules through OS-native wake signals
  plus durable misfire policy, and opens web, native and streamed artifacts from one owner surface.
- [`sprint-39.md`](./sprint-39.md) — implemented release candidate: extends existing Attention into a
  work-through conversation with accountable leadership and implements Restless Managed, Codex and
  Claude Agent at one controlled runtime boundary. Provider-backed three-harness qualification remains.

Sprint 26 is the completed first stage of the
[Harness → NPC → benchmark programme](../../experiment/HARNESS_NPC_BENCHMARK_PROGRAMME.md). EXP-16 and
EXP-17 also concluded; the final benchmark selects solo production at outcome parity for the four
observed bounded work shapes while preserving lead accountability as a product-governance invariant.

## Restless Cloud delivery plans

The detailed Cloud roadmap, Fleet/cell/multiplayer sprint plans and public-operation plan live in the
separate [restless-cloud repository](https://github.com/BlueprintLabIO/restless-cloud/tree/main/docs/sprints).
Core retains only its release-contract and cell responsibilities; these plans do not authorise a Core
deployment, provider connection or customer cell.
