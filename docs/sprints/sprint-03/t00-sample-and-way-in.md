# S03-T0 · The sample, and the way in

**Layer:** All — Aris does this itself
**Depends on:** nothing
**Blocks:** T5 (there is nothing to approve until there is something to send), T6

---

## Why this ticket exists

Aris has never had anything to give away, and no way for a stranger to ask for
it. Both gaps have to close before a single real email is worth sending, and
neither needs a provider, an ingress, or a credential.

It is also the sprint's cheapest failure. If Aris cannot produce a sample a
parent or tutor would actually want, we learn that **before** standing up
Resend, a sending domain, a webhook tunnel and an approval gate — rather than
discovering it after, with all of that built.

## Aris does this, not us

The temptation is to write the sample and the landing copy ourselves, because we
can and it is faster. Doing so would test our plumbing against our own
marketing and prove nothing about the product.

This is precisely the work `CLAUDE.md` says must route through the model —
judgement, language, taste — and precisely the work sprint 02 found the company
never doing for itself. So the ticket is: **ask, then look at what comes back.**

## Scope

1. **A free sample.** One genuinely useful 11+ practice artifact — the real
   thing, not a teaser. Aris chooses format and scope and says why.
2. **A way to ask for it.** A single static page: what it is, who it is for,
   and a form that captures an email address with explicit consent to be
   emailed about it. Built with the existing `web.deploy` capability, simulated
   this sprint.
3. **A channel strategy, written by Aris, from its own evidence.** Which
   segment, which channels, in what order, and why — as a `hypotheses/` record
   per `orgintel` §3.2.

## The strategy sub-task is the real experiment

Seed it with its **own** prior findings, already in its journal, and nothing
else:

> *DURABLE LOSS: P1 Priya — price vs incumbent Bond/CGP stack; concedes quality, not value.*
> *P8 Simon — CEM/Devon; wants a CEM product (does not exist).*

A price rejection against an entrenched incumbent, and a segment mismatch. Sprint
02 established that Aris recorded both faithfully and then kept selling to
parents; it never asked whether the segment was wrong.

Three outcomes, all informative — this is why the ticket is cheap and worth
doing first:

| Aris concludes | What it means |
|---|---|
| something like "shift to tutoring businesses; parents become inbound" | the capability was always there and we never asked. The value proposition is real and under-exercised |
| something materially worse | we know exactly what exploration machinery must supply, from a concrete gap rather than a spec |
| it cannot engage with the question | the value proposition needs *building*, not exposing — and that reshapes sprint 04 |

**Do not lead the witness.** Aris is not told the answer, the legal constraint,
or the channel table from the sprint spec. Those are compared against its output
afterwards. A hint here converts the experiment into a transcription exercise.

## Files it maintains (per `orgintel` §3.2 / §3.4)

Readable files with light metadata — **explicitly not** a state machine, per
§3.2's own instruction. Open records enter the wake context, the same pattern as
the effect ledger and the shared spine.

```text
/company/org/hypotheses/<slug>.md    question, prediction, cheapest test, stop criteria
/company/org/improvements/<slug>.md  observed problem, change, predicted effect, result
```

## Acceptance

1. A sample artifact exists in `/company/outputs/`, and the **owner** judges it
   good enough to put a name behind — recorded with a one-line rationale.
2. A request page exists and captures an address with explicit consent.
3. A `hypotheses/` record exists naming a segment and a channel order, with a
   prediction concrete enough to be wrong.
4. That record cites Aris's **own** prior evidence, not the sprint spec.
5. The strategy is compared against the sprint-03 channel table, and the
   difference is written down either way.

## What this makes deletable

Nothing yet. If the hypotheses/improvements convention earns its place, the
`add_goal` / `add_decision` / `add_artifact_ref` OrgIntel methods — storage for
concepts with no write path for any actor, flagged during the sprint-02 purge —
become the obvious next question: wire them, or delete them.
