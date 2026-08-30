# Sprint 26 — An exact substrate for unattended product work

**Status:** Draft — awaiting founder alignment on tickets

**Date:** 30 August 2026

> **Reconstruction notice.** The original `sprint-26.md` was an untracked draft and was destroyed by an
> agent overwrite on 30 August 2026. It is not recoverable. This file was rebuilt the same day from the
> eight surviving ticket files in [`sprint-26/`](sprint-26/), the sprint's one-line summary in
> [`README.md`](README.md), and the EXP-15/EXP-16 entries in
> [`experiment/coordination/REGISTRY.md`](../../experiment/coordination/REGISTRY.md). The tickets are
> original and untouched; **this spec's prose is a reconstruction and may not match the founders'
> original wording, framing or emphasis.** Treat the tickets as authoritative where they differ, and
> revise this file freely — nothing here should be preserved out of deference to a lost draft.

**Target:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §5, §16 ·
[`docs/COORDINATION_THEORY.md`](../COORDINATION_THEORY.md) ·
[`docs/specs/evaluation-dogfood.md`](../specs/evaluation-dogfood.md)

**Blocks:** EXP-16 (embodied NPC playtesting) may not activate until S26-T8 records a passing substrate
baseline.

---

## Why this sprint exists

EXP-15 ran a supervisor-led studio through a long campaign of evidence-led loops, and it worked — but
the controller kept it working by hand. Across the campaign it used manual branch resets, ownership
repair, process inspection, port cleanup, repeated gate runs and artifact relinking to keep useful work
moving.

Every one of those is a human standing in for a substrate that is not exact enough:

- **Lineage was prose.** Work carried its intended candidate in a brief while its executable base
  stayed `main` at an older commit. A lead had to notice, abandon useful Work and recreate it.
- **Workspaces were shared and mixed-owner.** Godot wrote `.godot/` into a shared product repository
  and integration refused a dirty tree; a copied tree kept uid 501/root ownership while the actor ran
  as uid 2000, so promotion failed despite correct product work.
- **Scarce resources were guessed.** Two verifier modes both took fixed port `24632`; one failed to
  bind while a client attached to the survivor, producing mixed evidence that read as a product
  failure.
- **Gates were re-enacted from prose** in producer, verifier and lead contexts, so identical suites ran
  repeatedly, attribution blurred, and a timeout or a zero exit could pass for product evidence.
- **Feedback interrupted.** New information arriving after an Attempt snapshot froze caused automatic
  supersession, multiplying one small change into five or six attempts.
- **Supervision woke on facts, not decisions.** The lead woke after progress events, spent about a
  model turn each time, and repeatedly concluded that Staff was still active.
- **Promotion needed a model.** Valid files existed with unlinked artifact references, costing actor
  turns purely for bookkeeping; a review directory presented as immutable was later reused for
  different content under the same human-readable name.

These are one failure, seen seven ways: **the coordination substrate is not exact enough to run
unattended.** Each gap is individually survivable with an operator watching. Together they are the
reason a campaign needs one.

## Outcome

An Attempt runs from exact coordinates, in an isolated workspace, over leased resources, verified by
gates executed once, supervised by wakes that carry decisions, and promoted transactionally — without
an operator repairing the substrate mid-campaign.

The test is not that each mechanism works in isolation. It is that the EXP-15 failure cluster, replayed
as one deterministic fixture, no longer needs a human.

## Acceptance criteria

The sprint's acceptance is the union of its ticket acceptances, proven together in T8's integrated
scenario rather than separately. In summary:

1. An Attempt launches only from frozen, typed coordinates, and a mismatch costs no model tokens.
2. Every Attempt gets a writable candidate tree owned by its actor, with caches outside Git custody.
3. Scarce resources are leased with identity and cleanup; concurrent gates cannot cross-connect.
4. A gate definition executes once per candidate tree, gate digest and toolchain fingerprint.
5. Ordinary feedback lands at a safe checkpoint; stopping productive work is explicit and authorised.
6. Leads wake for decisions, not for progress.
7. Promotion is atomic, content-addressed and write-once, with no model in the bookkeeping path.
8. The integrated fixture runs clean, concurrent and crash/restart variants with zero manual repair.

## Slice per layer

- **OrgIntel** — Attempt coordinates and lineage, feedback queueing and delivery, supervisory wake
  classification and coalescing.
- **Runtime** — hermetic workspaces, resource leases, the gate executor, review-target publication.
- **Authority Plane** — lease ownership, authorised interruption, promotion custody.
- **Evaluation** — the deterministic fixture, the measured baseline and the deletion record.
- **Out of scope** — anything EXP-16 needs that this substrate does not; broadening the frozen
  vertical-slice contract.

## Ticket decomposition

Status lives only in this checklist; ticket files record scope and closure evidence, not a second
status system.

| Status | Ticket | Slice | Outcome or friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [ ] | [S26-T1 · Bind Attempts to exact execution coordinates](sprint-26/t1-exact-attempt-coordinates.md) | OrgIntel + Runtime | Intended candidate in a brief, older commit on disk | Prose-parsed base refs; default-to-`main` launch; manual lineage repair |
| [ ] | [S26-T2 · Materialise hermetic actor workspaces](sprint-26/t2-hermetic-workspaces.md) | Runtime | Engine caches dirtied the tree; mixed ownership blocked promotion | Repo-local caches; shared mutable worktrees; privileged ownership repair |
| [ ] | [S26-T3 · Lease scarce runtime resources](sprint-26/t3-resource-leases.md) | Runtime + Authority | Two verifiers on one fixed port produced mixed evidence | Guessed ports; shared display assumptions; pid-file folklore |
| [ ] | [S26-T4 · Execute declarative gates once](sprint-26/t4-gate-executor.md) | Runtime + Evaluation | Suites re-enacted from prose in three contexts | Prompt-authored gate sequences; unkeyed reruns; exit-code pass claims |
| [ ] | [S26-T5 · Separate feedback from interruption](sprint-26/t5-feedback-checkpoints.md) | OrgIntel + Authority | One small change became five or six attempts | Automatic supersession on any new message |
| [ ] | [S26-T6 · Coalesce supervisory wakes](sprint-26/t6-lead-wake-coalescing.md) | OrgIntel | Paid lead turns concluding "still active" | Unconditional per-event lead wake |
| [ ] | [S26-T7 · Promote evidence transactionally](sprint-26/t7-transactional-promotion.md) | OrgIntel + Runtime + Authority | Actor turns spent on artifact bookkeeping | Model-mediated promotion; partial promotion; mutable review directories |
| [ ] | [S26-T8 · Prove recovery and delete the escape hatches](sprint-26/t8-recovery-and-deletion.md) | Evaluation | Any one fix can pass while the system still churns | Known-instance repair scripts; operator process/filesystem archaeology |

T8 is the closing ticket and runs only after T1–T7 land. Its deletion audit is part of the work, not
optional cleanup.

## Non-goals

No new company capability, product surface or owner-facing feature is claimed by this sprint. It makes
existing work exact. A test-count increase is not its progress measure; the measure is model turns,
lead wakes, gate executions, manual interventions and retained junk in T8's fixture.
