# Sprint 05 — Clear the launch queue and make first centre contact

**Status:** Draft for founder alignment — thesis selected from the Sprint 04 run; ticket shapes still
need founder review before implementation.
**Date:** 15 August 2026
**Spec refs:** `owner-cockpit` §5 / §2.7 / §4.1 / §14.6, `authority-plane` §8.2 / §8.3,
`cross-layer-contract` §3.1 / §4.5, `ARCHITECTURE.md` §16.2 / §16.10 / §690

---

## Outcome

> **The owner clears one real commercial launch queue — merge, production evidence and four exact
> first-contact grants — and Aris sends the prepared tutoring-centre sample outreach through governed
> effects, then records the first reply, objection or bounded non-response as the next revenue
> decision.**

> **Success contract:** after the owner merges the prepared centre-offer PR and grants authority for
> the selected canonical parties, the production centre page is observed live and at least four real
> first-contact emails are accepted by the provider with receipts. The company observes replies for a
> bounded five-business-day window; a reply/objection advances the offer, while zero replies is
> recorded as channel evidence rather than narrated as success.

This is a real-company outcome. The owner surface is a supporting slice, not the sprint's product.

## Why these two moved out of sprint 04

Sprint 04's own acceptance list asked for the CLI to become the complete control surface. It was pulled
back to a bounded version (AC9: the owner answers AC7's four questions without `psql`) and the rest
moved here, on one rule:

> **§16.2** — A slice is complete only when it produces a useful artifact, decision, or external
> outcome. *A schema, API, or invariant suite alone is not a successful slice.*

A sprint that ships verbs and a coverage test produces no company outcome. This repo has run that
experiment: `orgintel` §6.3 sat unimplemented for two sprints while a comparison harness was built and
run four times **to measure its absence**.

There is also a loop worth naming. `restless receipts` over three email sends is a different command
than one over a shipped PR with suite results and per-role costs. Sprint 04's learning list says AC7's
shape *"is a guess, and the run should correct it."* Building the surface first ships the guess.

**So the sequencing is the point:** sprint 04 produces the work and records every reach for `psql`;
sprint 05 builds the surface those reaches describe.

## Tickets carried in

| ✓ | Ticket | Layer | Evidence (observed friction) | Depends |
|---|---|---|---|---|
| [ ] | **S05-T1 · The attention queue, as a projection** | Owner surface | The owner's ask already exists (`approval.rs:96`) and is `bail!`ed back to the blocked *agent* (`effect.rs:146`), with an untyped copy as mail that `inbox` marks read and forgets | S04-T10 |
| [ ] | **S05-T2 · The CLI is the complete control surface** | Owner surface / Authority | Three companies exist and all three were created by hand-writing TOML; `CompanyConfig::load` tells the owner to *"see `companies/` in the repo"*. Granting authority is a command, withdrawing it is an editor action. Nothing checks that a daemon capability has an owner verb | S04-T10 |
| [ ] | **S05-T3 · Launch gate → first real centre outreach** | All | Sprint 04 ended with two compare links, four double-verified parties and tailored drafts, but the owner must reconstruct the launch sequence from mail while production probes are red | S04-T4, S05-T1 |

T1 and T2 depend on **S04-T10**, which landed in Sprint 04. T3 is the outcome slice; T1 exists to make
its human boundary calm and explicit. T2 is useful but is below the line if it does not help this run.

## What the Sprint 04 run decided

1. **Attention is genuinely a queue.** One run produced two merge links, four exact party approvals,
   a production-health blocker, an older copy verdict and five older send approvals. Ordinary mail is
   not a workable owner queue.
2. **The original T1 projection is too narrow.** `approval_required - approval_granted` can render
   email grants, but it cannot represent the observed compare-link decision or production gate. T1
   must be revised before coding from the proven categories, without jumping to all eight hypothetical
   categories in `owner-cockpit`.
3. **The four owner questions were right.** `people`, `spend` and `receipts` answered role, model,
   cost and output without `psql`. Keep those reads; do not redesign them in Sprint 05.
4. **The generic runtime door held.** `attach`, `doctor` and `up --reconcile` covered the company
   computer without one API per Linux command. T2 must not grow runtime inspection verbs.
5. **Durability means written state.** Sixty-seven ephemeral browser/tool calls vanished across
   continuations because the task wrote only at the end. Incremental ordinary-file checkpoints fixed
   the workload; this does not earn a workflow engine.

## Run inputs

From Sprint 04's run report:

1. The owner used zero `psql` while operating or assessing the run.
2. The owner needed role, model, cost, output, receipts, compare links, exact parties and live health
   probes. The first four now exist; the latter three are the launch queue.
3. Four verified parties and tailored drafts exist now in the persistent company volume. No prospect
   generation is needed before Sprint 05 can attempt a real external outcome.

## The bound that already survived one round

`S05-T2` states the principle as *every act of **control*** — not *anything the owner can do*. A company
contains a full Linux computer and no verb set enumerates a filesystem. Three surfaces, kept apart:

| What | Surface | Why |
|---|---|---|
| Coordination, authority, configuration | **CLI verbs** | Finite, enumerable, checkable |
| The company computer itself | **`restless attach`** | Unbounded. Build a door, not a verb for `cat` |
| Judgement about what is in there | **The Exec, via `tell`** | Language, not enumeration |

The tempting fourth option — *the CLI asks the Exec to report on the runtime* — is right for row three
and unacceptable as the general answer: it makes the owner's only view of the machine the account of the
actor under review. `owner-cockpit` §2.7 is titled *evidence before self-report*, and this repo has paid
for it twice (three wakes spent on a landing page produced by a *simulated* `web.deploy`; a journal
claiming revenue receipts put at £18). The CLI's obligation toward the runtime is **checkable pointers**
— commit SHA, PR URL, receipt, suite exit code — not a rendering and not a narrative.

## Carried in from earlier sprints

- **S03-T8 (owner wire contract)** — the general fix remains: `BlockKind` flattened to prose at the
  boundary, writes returning state instead of `{accepted, status}`, idempotency on §4.5's five classes.
  `S05-T1` discharges its item 6 for the attention projection specifically.
- **The owner SPA** — `web/` renders from fixtures. The surface principle applies to it identically the
  moment it is wired, and `S05-T2`'s completeness test is written against the daemon's capabilities
  rather than the CLI's argument parser, so the SPA inherits it rather than needing a second one.
- **The reply leg** — blocked on one owner MX record for `reply.blueprintlab.io`.

## Open, for the founders

- Which proven non-email item enters the first attention slice: the compare-link merge, the production
  health blocker, or both? Choose the smallest projection that lets T3 run without mail archaeology.
- Does sprint 04's run make the SPA the more urgent client than the CLI? If the answer is yes, S03-T8
  stops being carried and becomes this sprint's spine.
