# Sprint 03 — Aris email goes live (the first real provider)

**Status:** Draft for founder alignment
**Date:** 14 August 2026
**Spec refs:** `authority-plane` §2.2 / §2.6 / §6 / §8 / §9 / §19 / §20.2,
`company-runtime` §11.1 / §11.4 / §11.5, `cross-layer-contract` §3.1,
`evaluation-dogfood` § (Aris), `ARCHITECTURE.md` §10.7.2 / §10.8, sprint-02 T5 / T6 / T9 (carried).

---

## Outcome

> **Aris puts a free sample in front of real people who asked for it, and follows up when they
> reply — through a real provider behind the effect surface, with the world pushing back through a
> governed event ingress. The runtime stays closed; the Authority Kernel proper stays deferred.**

The technical outcome is the same rail the earlier draft described. What changed is the **success
contract**: this sprint is judged on whether real people engage with a real offer, not on whether a
loop closes. A sprint that proves the plumbing and reaches nobody has proved the plumbing.

> **Success contract:** at least **10 real people receive the sample**, at least **one replies**, and
> the owner judges at least one reply to indicate **genuine interest** — recorded with a one-line
> rationale (`evaluation-dogfood` §3.2, §21.2). Revenue is explicitly *not* the bar this sprint;
> demand evidence is.

### Why this sprint, why now

Sprint 02's risk register named a single trigger twice: **"the first live provider."** It is the
expiry date for two deliberately-deferred things — *no governance on effects*, and the *provider
credential inside the company container* regression. Going live on email fires that trigger. It is
also `ARCHITECTURE.md §10.8`'s planned progression reaching its third and final stage: scripted
simulation → noisy behavioural simulation → **small controlled real run**. Sprints 01 and 02 lived
entirely in stages one and two.

There is a deeper reason it cannot wait. An autonomous company that runs a business **must receive
asynchronous pushes from the world** — a reply, a payment clearing, an order, a ticket. A company
that can only poll outbound is always slightly behind the world it is supposed to be running. Polling
is a tax; an event ingress is the natural shape. And it is unavoidable regardless of email: payments
are webhook-native (`company-runtime §11.4` — Stripe), so the ingress rail is needed the moment any
company touches real money. Building it for email first means email rides the same rail every later
provider will, rather than email getting a one-off receive mechanism that gets rewritten when
payments arrive.

Sprint 01 settled the substrate; sprint 02 asked whether OrgIntel earns its place. Sprint 03 asks the
question the product actually exists for: **can a real provider run through the effect surface, and
can the world push back** — with the runtime still closed and the kernel still deferred.

## Two tracks, one of which is the point

**Track A — the live loop.** Make the first real provider work end to end, both directions.
**Track B — the boundary that "first live provider" trips.** The event ingress, the credential
indirection, the recorded posture change. Boundary work, included for the same reason sprint 02's
Track B was: *boundaries are cheap early and expensive late; features are the opposite.*

---

## Acceptance criteria

Headless with stated inputs and observed output (CLAUDE.md → "Verifying"). Nothing is described as
working until it has been run.

### Track A

1. **A real email sends through the effect surface.** `restless effect email.send` to a real
   Resend-verified address yields a receipt with `provider: "resend"` (not `"simulated"`), and the
   email arrives at the recipient. The request shape is identical to the simulator (`§19`).
2. **A real reply arrives and wakes the company.** A reply to that email is delivered by Resend as a
   signed webhook; the ingress verifies the signature, writes an authoritative effect/event, projects
   an OrgIntel message, and the scheduler wakes Aris — **without the owner typing**. Aris reads the
   reply via `restless inbox`.
3. **The loop closes autonomously, once.** Aris, woken by the real reply, composes and sends a real
   follow-up through the effect surface. One full send → reply → wake → respond cycle, observed.
4. **Recipients arrive by request, not by cold send.** Every address Aris emails came from someone
   asking for the sample, or from a list the owner has a lawful basis to contact and has explicitly
   named. **The company never sources its own recipients this sprint.**
5. **The business outcome is verified against the provider, not the company.** Sends, deliveries and
   replies are counted from Resend's own record and the ingress events — never from Aris's journal.
   Aris has already once reported £45 of revenue that its receipts put at £18 confirmable, with its
   single named customer recorded as a loss.
4. **A repeated or ambiguous send does not double-fire.** A webhook redelivered does not wake twice;
   a send retried with the same idempotency key replays the stored receipt and never re-sends
   (`§9.1`, `§9.3`).
5. **A repeat send to the same party is surfaced.** The sprint-02 T7 party-reconciliation signal
   holds on the real provider: emailing the same party twice under different keys is flagged, not
   silent (`§9.4`).

### Track B

6. **The event ingress is its own failure boundary.** The webhook listener is not on the
   coordination listener's path. A slow, malformed, or flood-abused request cannot stall the
   scheduler or OrgIntel — the F12 lesson (one company's hung Docker took down all three). Demonstrated
   by a deliberately stalled ingress not blocking a concurrent scheduled wake.
7. **No consequential credential enters the company container.** The Resend key and the webhook
   signing secret live host-side; a grep of container env and filesystem finds neither. The credential
   regression is closed for email, and the path is clear for sprint-02 T5 to close it for the model
   key (`§6.2`, `§11.5`).
8. **The network posture change is recorded, not silent.** A short note (amending `company-runtime`
   §11.1, or an ADR) states: *the control plane exposes an authenticated event ingress; the runtime
   remains closed by default.* The new public surface does not appear unmarked.
9. **Approval is a minimal typed check, not a kernel.** The first live send to a new party raises
   owner attention; sends within an owner-approved envelope do not. No policy DSL (`§6.4`, `§6.5`).
10. **The owner surface speaks one vocabulary, and failures are typed.** The attention item T5 raises
    crosses the wire under one name with a stable `attention_item_id`; a credential failure and an
    unreachable daemon arrive as different `error.kind` values, not different prose. Verified by two
    headless calls (T8).
11. **Idempotency is scoped, not universal.** An owner approval replayed with the same `operation_id`
    resolves once; a `tell` sent twice without one delivers twice. Both observed. `§4.5` requires
    stable identity for five classes and explicitly forbids requiring it everywhere (T8).
12. **The owner API stays local-only.** The event ingress (T2) remains the only public port. A grep of
    listeners finds the coordination surface bound to a Unix socket or localhost, never `0.0.0.0`.

---

## Tickets

Each names its layer, the observed friction it serves (`ARCHITECTURE.md §16.7`), and what prior
machinery — if any — it makes deletable.

| ✓ | Ticket | Layer | Evidence (observed friction) | Depends |
|---|---|---|---|---|
| [ ] | **S03-T1 · Provider dispatch + Resend send adapter** | Authority (effect service) | `effect.rs` hardcodes the simulator: `provider: "simulated"` for every capability. `§19` requires Mock/Real behind one interface; only Mock exists | — |
| [ ] | **S03-T2 · Event ingress (authenticated, idempotent)** | Authority (effect service) | There is no inbound surface at all. The company can only act on what it polls; payments are webhook-native (`§11.4`) so this rail is needed regardless of email | T1 |
| [ ] | **S03-T3 · Inbound email → OrgIntel message projection + wake** | Cross-layer (Authority authors; OrgIntel projects) | The scheduler already wakes on new mail, but only on *simulated* mail via `restless message`. No real reply has ever closed a loop. `§3.1`: projection, not a second writer | T2 |
| [ ] | **S03-T4 · Credential indirection + host-side secret** | Authority (credential plane) | Model key regressed into the container via `docker exec -e` (sprint-02 register). `§2.6` / `§11.5`: consequential credentials stay outside the runtime | T1 |
| [ ] | **S03-T5 · Minimal approval check for live sends** | Owner surface / Authority | `§6.4`: approvals are rare exceptions. A real send to a real person is materially external; the first one needs a human yes | T1, (sprint-02 T9 or stand-in) |
| [ ] | **[S03-T0 · The sample and the way in](./sprint-03/t00-sample-and-way-in.md)** | All (Aris does this itself) | Aris has never had anything to give away and no way for a stranger to ask. Cold-sending is unlawful here, so demand has to be pulled, not pushed | — |
| [ ] | **[S03-T7 · Test/live split](./sprint-03/t07-test-and-live.md)** | Runtime / OrgIntel | Going live removes the ability to smoke-test on the live thing. `drop_schema` has been unused since sprint 01 and was nearly purged | T1 |
| [ ] | **[S03-T8 · The owner wire contract](./sprint-03/t08-owner-wire-contract.md)** | Cross-layer (owner surface) | T5 needs an attention item to raise an approval against, and there is no owner-facing noun on the wire. `web/` is 9,841 lines of SPA already rendering `DeskView` from fixtures — the vocabulary is being reconciled, not chosen | — |
| [ ] | **S03-T6 · The one-real-loop run + report** | All | `§10.8` stage 3 has never run; `§20.2` Aris acceptance ("schedules follow-up using the authoritative receipt") is untested against the real world | T1–T5 |

**Status (2026-08-14):** draft, pre-implementation. Sprint-02 T4 (the A/B/C comparison) is still
running and is not blocked by anything here.

**T0 comes first and Aris does it, not us.** A sample worth requesting and a page to request it
from is *the company's own work* — exactly the judgement-and-language work the product exists to
route through the model. If we write it, we have tested our plumbing against our own marketing, not
Aris's. It also fails cheaply: if Aris cannot produce a sample a parent would want, that is the
finding, and it arrives before a single real email is sent.

**If only three tickets land, they are T1, T2 and T3** — the adapter, the ingress, and the
reply→wake projection — exercised by T6. That is the irreducible thesis: a real provider behind the
effect surface, and the world pushing back through a governed front door. T4 and T5 are what make
doing it to real people acceptable; they are not optional for a live run, but the *question* is
answered by the loop.

**T8 is small and goes before T5, not after.** It is not on the critical path to the thesis, but T5
cannot raise an approval without an item to raise it against, and whatever name that item gets is
inherited by the CLI, the notification path and the SPA simultaneously. Doing it after T5 means doing
it three times.

### Notes per ticket

**T1** turns `request_effect`'s hardcoded simulator branch into one provider of N. Dispatch is per
`(company, capability)`: `email.send` → Resend when configured, simulator otherwise, so Aris's
`web.deploy` and the other companies stay simulated untouched. The Resend call runs host-side in the
daemon (the email adapter is an Authority-Plane component, `§2.6`), so the agent's path
(`restless effect email.send …`) is unchanged and the key never enters the container. **Deletes:** the
simulator-as-the-only-world assumption; the `provider: "simulated"` literal becomes a value.

**T2** is the sprint's structural bet. A public HTTPS listener owned by the effect surface, on its
own failure boundary (own tokio task, bounded channel, request timeout), that does exactly three
things: verify the provider's HMAC signature, dedupe by provider event id, enqueue. It does *not*
touch OrgIntel synchronously. It is shaped as a unit — *ingress + effects + receipts* — so that
sprint-02 T6 (the plane split) extracts it whole later, no refactor through living code. Signature
verification is the trust boundary, not IP allowlists. **Deletes:** nothing yet; creates the place
T6 carves out.

**T3** implements the `§3.1` rule the hardest way: the inbound reply is **authored** as an
authoritative effect/event in the Authority layer (the world did this), and **projected** as a message
into OrgIntel so the agent can read it and the scheduler can wake. OrgIntel is never a second writer
of "what the world did." The scheduler's existing new-mail wake (`schedule.rs`) then does the rest.
**Deletes:** the assumption that mail only arrives via `restless message`.

**T4** is a slice of sprint-02 T5, scoped to the email credential. The Resend key and webhook secret
are read from the daemon's environment at the point of use (the pattern the model key already uses),
and a `credential_reference` pointer (`§8.2`) is stored in company config now — resolved via env
today, swappable to Infisical later without touching the adapters. **Deletes:** nothing; adds the
cheap-early indirection that makes the Infisical swap a config change, not a code change.

**T5** is a typed check, not a kernel: a new party on a live capability → owner attention (sprint-02
T9's queue, or a minimal stand-in if T9 is not yet wired). It does not encode policy, thresholds, or
a DSL (`§6.5`). It exists because a real send to a real person is `§6.4`'s "materially irreversible"
case. **Deletes:** nothing. **Its item's shape comes from T8**, not from T5 improvising one.

**T8** is the cheap-early half of the owner surface, and it is here because T5 forces the question:
an approval needs something to be raised *against*, and no owner-facing noun exists on the wire. The
insight is a cost asymmetry — **the transport is disposable, the vocabulary is permanent.** Swapping
a socket for HTTP later is mechanical; renaming a noun three clients have hardcoded is not. So T8
settles nouns, error kinds, idempotency scope, write shape and the event rail's contract, all inside
the *existing* socket protocol. No HTTP, no SSE, no framework choice — those are deferred precisely
because they are replaceable.

The correction that makes it concrete: **there is not zero clients.** `web/` holds 9,841 lines of
SPA (`cf8a028`) rendering `DeskView` — 20 fields including `needsYou: NeedsYouItem[]`, with an
`EmailDraftView` that is exactly what T5 must show, and a `version` on each ref that answers the
double-clicked-Approve problem better than a client-generated key. In two places it already encodes
`§4.7` honest-status discipline the daemon does not (`ConnectionRow` keeps `ok` and `failed` apart so
"never checked" is representable; `HqView.runway` pairs a null with the reason it is null). It also
carries asset-custody fields the rebuild deliberately discarded. So T8's first task is a
reconciliation, not a design: 20 fields, each marked *live source* / *derivable* / *delete*.
**Deletes:** `Blocked::message()`'s string flattening at the wire, the ad-hoc error prose in
dispatch, and `DeskView`'s `library` / `records` custody surface.

**T6** is the evidence. One consenting recipient, named by the owner; one send, one real reply, one
wake, one follow-up. The report records the loop end to end against `§20.2`'s passing evidence, plus
where the real loop broke that the simulator never exposed (deliverability, timing, ambiguous
outcomes). Per `§25` rule 10: if the loop fails, that is the finding and it is written down as such.

---

## What we are trying to learn

- Does the effect surface, built entirely against simulators, carry a real provider with **no
  company-side change** — the `§10.8` claim that "the company should not need different logic for a
  simulated provider"?
- Does an event ingress as the world's front door let the company react to real async signals in real
  time — and is that the shape we want for payments and orders next, or does the first live run expose
  a different need?
- Is daemon-env plus a `credential_reference` enough secret management for one provider, or has the
  workload already crossed the line where Infisical earns its weight? (Expectation: no. Confirm the
  trigger has not fired.)
- Does a minimal typed approval gate suffice for a real external effect, or does the first live run
  demand more governance than `§6.5` assumes?
- Where does the real reply loop break that the simulated one never could? (deliverability, sender
  reputation, ambiguous provider responses, consent.)

---

## Risk register

Every risk named, one disposition each. Default accepted.

| Risk | Disposition | Why |
|---|---|---|
| Public ingress exposes the control plane to attack / DoS | **Guarded** | Signature verification drops unauthorized traffic at the edge; the endpoint verifies and enqueues, nothing more; its own failure boundary (T2/AC6); sit behind a WAF or tunnel where practical |
| Spoofed webhook ("this is from Resend") | **Invariant** | HMAC signature verification is non-negotiable; an unsigned or invalid request is dropped before it reaches OrgIntel |
| Cold-outreach deliverability / sender-reputation damage | **Accepted (finding)** | Resend is a transactional sender; the MVP is one consenting recipient. Cold prospecting at scale is out of scope and a different problem (see below) |
| **Cold email to parents (individuals) is not lawful in the UK** | **Invariant** | UK PECR reg 22 requires prior consent for unsolicited electronic marketing to *individual subscribers*, and soft opt-in does not reach new prospects. Not a deliverability preference — the law, and it does not bend for a sprint goal. |
| **Cold email to incorporated businesses is lawful, with conditions** | **Guarded** | PECR reg 22 does not cover *corporate subscribers* — limited companies, LLPs, schools, academies, MATs. So B2B outreach to tutoring companies is permitted. Conditions that are not optional: UK GDPR still applies where the message identifies a person (legitimate interests + a recorded assessment + honouring objections); **sole traders and ordinary partnerships count as individuals under PECR and are therefore off-limits**; every message must identify the sender and carry a working opt-out (reg 23). The sole-trader carve-out is the trap — much of the tutoring market is one person. |
| Aris misjudges corporate vs sole trader | **Guarded** | The company does not decide this. The owner supplies or approves the target list with the entity type stated; Aris drafts and personalises. Companies House incorporation status is the check, not a guess from a website. |
| The owner is the sender of record | **Accepted** | Sending from an owner-controlled domain means the owner carries reputational and legal responsibility for what an autonomous agent writes. Mitigated by T5's approval on first contact with any new party, and by the sample being genuinely useful rather than a pitch. |
| **Aris misreports a real commercial outcome** | **Guarded** | It has done exactly this once: £45 claimed, £18 confirmable, its one named customer recorded as a loss. Sprint-02 reconciliation now shows receipts back to it each wake, but that has never been tested with real stakes. AC5 counts outcomes from the provider's record, never the company's account. |
| The Resend key or webhook secret leaks via the container | **Pending fix** | T4 holds both host-side; AC7 grep-verifies; closes the sprint-01 credential regression for email |
| Building the Authority Kernel proper prematurely | **Guarded** | Sprint 03 builds the receiver + adapters + ingress only. Capability grants, policy language and the full `§5`/`§6` engine stay deferred; approval is a typed check (T5) |
| Adopting Infisical before its workload | **Guarded** | Deferred with a named trigger (N credentials / a second operator). Daemon-env + `credential_reference` now; the trigger is re-evaluated at sprint end |
| The ingress couples to the OrgIntel failure path (F12 repeat) | **Guarded** | T2 gives it its own listener and bounded channel; AC6 demonstrates a stalled ingress does not block a scheduled wake |
| The control-plane posture change goes undocumented | **Guarded** | AC8 records it (note or ADR). The new public surface is marked, not smuggled |
| **The SPA and the daemon drift into two vocabularies** | **Guarded** | Already half-true: `web/`'s `NeedsYouKind` and the contract's `attention_item` name the same concept differently, and nothing reconciles them. T8's first task is the field-by-field pass; the cost of deferring is that every synonym invented this sprint becomes SPA debt |
| Wiring `DeskView` as-is drags back discarded machinery | **Guarded** | Its `library` / `records` fields are the content-addressed asset-custody surface CLAUDE.md forbids recreating. T8 deletes them from the type rather than finding them a backend source |
| T8 becomes a protocol redesign instead of a naming pass | **Guarded** | Scope is explicitly inside the existing socket protocol. HTTP, SSE, framework choice and version negotiation are named out of scope because they are replaceable; only vocabulary, error kinds, idempotency scope and write shape ossify |

---

## Planning decisions (recorded)

This sprint was planned through a founder discussion. The decisions and their rationale are recorded
here so they are not relitigated or silently lost.

- **Two lawful motions, run together: B2B outbound and consumer inbound.** An earlier version of
  this section said inbound-only. That over-corrected. The unlawful thing is cold-emailing *parents*;
  cold-emailing an incorporated *tutoring business* is a different legal regime and is permitted.

  It is also the better commercial bet, and the sprint-01 Aris run is the evidence: its one durable
  loss was a parent rejecting on price against an incumbent Bond/CGP stack, and another prospect
  wanted a product for an exam board that does not exist. Both are consumer-market frictions. A
  tutoring business buys once and serves many students, is less price-sensitive per unit, and is a
  *distribution channel* rather than a single sale — one tutor can put the papers in front of thirty
  parents Aris is not allowed to email.

  So: **B2B outbound to incorporated tutors and schools** for speed, and **consumer inbound** (free
  sample behind a request page) for the parents, where email is the reply to a request rather than an
  intrusion. Same rail, same effect surface, two audiences.

- **Motion detail: inbound for consumers.** The earlier draft's "one consenting recipient the
  owner names" is a friendly test subject, and quietly avoids the question the sprint now exists to
  answer. But the fix is not to cold-email parents — that is unlawful here, and no amount of
  deliverability engineering makes it lawful. It is to **invert the motion**: Aris publishes a
  genuinely useful free sample behind a page where a parent asks for it, and email becomes the
  *reply* to a request rather than an intrusion.

  This is better on every axis that matters, not merely the legal one. Consent arrives with the
  address. Deliverability stops being the bottleneck (transactional mail to someone expecting it,
  which is exactly what Resend is good at — closing the caveat the provider decision held open).
  The sample is the product, so building it is not overhead. And a request is a far stronger demand
  signal than a reply to a cold send: someone who asked has told you something true.

  It also uses machinery that already works — `web.deploy` runs today (simulated), and making one
  static page real is a smaller step than making cold outreach lawful. Owner-supplied lists remain
  allowed where the owner states a lawful basis, but the company never sources recipients itself.

- **Provider: Resend.** API-first (one POST, one key), idempotency header maps onto our
  `idempotency_key`, free tier covers the MVP, handles DKIM/SPF/bounces, and its inbound is
  webhook-native and GA — so it rides the event ingress in both directions rather than needing a
  second receive mechanism. Rejected: Postmark (marginally better deliverability, pricier, stingier
  free tier, less webhook-first); Mailgun/SendGrid (heavier, no MVP gain); AWS SES (cheap at scale,
  too much setup now). *Caveat held open:* Resend solves transactional/consented send and reply
  receive; it does **not** solve cold-outreach deliverability at scale. If Aris's real bottleneck turns
  out to be cold prospecting, "which API" is the wrong question.
- **Receiver: the effect manager (the authority-plane effect surface), not `restlessd`-as-a-whole,
  and never OrgIntel.** A webhook is the inbound half of an effect — governed external interaction,
  same concern as the outbound send (`§2.2`). OrgIntel owns coordination and must not become a second
  writer of what the world did (`§3.1`): the reply is authored as an authoritative effect/event and
  projected into OrgIntel as a message. The ingress is a module of the effect surface on its own
  failure boundary today, shaped to slide into a separate authority-plane process when sprint-02 T6
  lands.
- **Inbound: event ingress, not IMAP polling.** Payments are webhook-native (`§11.4`); there is no
  IMAP for payments, so the ingress rail is needed regardless of email. Webhooks are also real-time
  and carry the full delivery lifecycle (delivered/bounced/complained), not just "a message arrived."
  IMAP polling was a laptop-behind-NAT concession, not a design endpoint; in the ingress design the
  earlier "IMAP vs webhook" fork disappears — email is one provider on the rail.
- **Secrets: defer Infisical with a named trigger; daemon-env + `credential_reference` now.** Infisical
  is the spec'd credential backend (`§2.6`, `§8`) but self-hosting it (MongoDB + Redis + Node + Go +
  nginx) for one Resend key is the speculative generality the repo exists to avoid, and `§8.1` itself
  says "before the workload requires it." The model key already proves a working daemon-env pattern.
  T4 adds the `credential_reference` indirection so the later Infisical swap is config, not code.
- **Posture: the runtime stays closed; the control plane opens a governed front door.** `company-runtime
  §11.1` ("no public inbound ports by default") is a *runtime* posture and stays true for the runtime.
  The control plane is a different component and is the natural place for an authenticated event
  ingress. The change is recorded (AC8), not smuggled.

---

## What is and is not an effect

Settled in `authority-plane` §2.2 during this sprint's planning, and load-bearing
for T1's shape: the effect service is an **accountability boundary, not an API
gateway**. Research, browsing and reading are ordinary runtime work with no
receipt; publishing a listing or sending an email is an effect. **A receipt does
not require an API** — a consequential action driven through the company's own
browser session earns the same receipt, idempotency key and party as an HTTP
adapter. So T1 builds provider dispatch, not a provider catalogue.

`aris_test` therefore simulates the **failure shapes** — success, denial, timeout
before execution, timeout after, duplicate, ambiguous outcome, delayed reply —
not a catalogue of providers. Seven shapes cover the behaviour; no number of
provider mocks would.

## Strategy is an OrgIntel problem, and the spec already solved it

Sprint 02 found that Aris recorded a price rejection against an incumbent and a
segment mismatch in its own journal, then kept selling to parents. It never asked
whether the segment was wrong. The instinct is to treat that as "the model needs
a better prompt". It is not — it is the absence of the two mechanisms
`orgintel` §1.1 names as what OrgIntel *is*, and §3.2 / §3.4 already specify
them.

**They are cheap.** §3.2 says an exploration record "may begin as a readable file
with lightweight metadata" and — explicitly — **"do not build an experiment state
machine in V0."** §3.4's evolution record is a markdown template. So this is a
file convention plus context assembly, not a subsystem.

Aris's own case maps onto §3.4's template exactly, which is the test of whether
the template is real:

```text
Observed problem:      parents reject on price against an entrenched Bond/CGP
                       stack; one prospect wanted an exam board we do not serve
Proposed change:       shift the primary segment to incorporated tutoring
                       businesses; parents become inbound-only
Why it may help:       a tutor buys once and serves many; less price-sensitive
                       per unit; is a distribution channel to parents we may not
                       lawfully email
Predicted effect:      higher reply rate per send; price objection stops being
                       the dominant loss reason
Scope and budget:      one sprint, the existing ceiling
Baseline:              the parent funnel's own conversion, already recorded
Result / Adopt:        pending
```

Sprint 03 therefore adds one thing to T0, and it is small: **Aris maintains
`/company/org/hypotheses/` and `/company/org/improvements/` as files, and open
records enter its wake context** — the same pattern as the effect ledger and the
shared spine. No state machine, no new tables, no `add_hypothesis` API. If the
company revises its own strategy from its own evidence, that is the first
evidence the value proposition is real rather than asserted.

## Channels: lawful, cheap, ranked by leverage

Aris's constraint is not budget, it is that it may not cold-email parents. Everything below respects
that. Ranked by evidence-per-pound, not by reach.

| Channel | Why it fits Aris | Cost | Lawful basis | Who acts |
|---|---|---|---|---|
| **Resource marketplaces** (TES, Teachers Pay Teachers) | Existing buyer intent — teachers and tutors arrive already shopping for practice papers. Handles payment and delivery, so no Stripe this sprint | free to list, ~rev-share | none needed; it is a shopfront | owner opens the account (identity/payout = last mile); Aris writes listing + samples |
| **B2B email to incorporated tutors & schools** | Buys in volume, less price-sensitive, becomes a distribution channel to parents Aris cannot email | ~free | PECR corporate subscriber + GDPR legitimate interests | Aris drafts and sends; owner supplies/approves the list |
| **Free sample behind a request page** | Consent arrives with the address; turns cold traffic into a lawful list that compounds | hosting only | consent given at request | Aris builds page + sample |
| **Tutor partnership / give-to-get** | Give papers free to tutors who redistribute; buys reach and testimonials at once | free | it is a gift, not marketing | Aris proposes; owner approves terms |
| **11+ forums & parent communities** (elevenplusexams, Mumsnet, subreddits) | Where the demand actually congregates; a genuinely useful free resource is welcome where a pitch is not | free, or cheap ad slots | platform rules, not PECR | **owner posts** — community accounts are personal identity |
| **Organic search on long-tail** ("free CEM 11+ practice paper") | The sample is the SEO asset; compounds while costing nothing | free | n/a | Aris writes; slow to pay off |
| **Paid search on long-tail** | Fastest signal on willingness-to-pay, tiny budget suffices in a niche | £5–20/day | n/a | owner sets billing; Aris writes copy |

**Not this sprint:** paid social (wrong intent, needs creative iteration), buying lists (unlawful for
individuals, poor quality for businesses), and anything requiring sender-reputation warmup.

**The pattern worth noticing:** the top two channels both put Aris in front of people who are already
looking, and neither requires Aris to interrupt a stranger. The consumer inbound page is what turns
that traffic into something it may lawfully email later.

*This is engineering planning, not legal advice — the corporate/sole-trader line in particular is
worth the owner confirming before the first send, because much of the tutoring market is one person.*

## Test companies and live companies

Going live removes the ability to smoke-test on the thing that is live. So the portfolio splits in
two, and the split is **not** persistent-vs-ephemeral — it is **which world the company acts on**:

| | Companies | Providers | Lifecycle |
|---|---|---|---|
| **Live** | `aris`, `thymelake`, `cosmon` | real where configured (`email.send` → Resend for Aris) | persistent; their history is evidence |
| **Test** | `aris_test`, `thymelake_test`, `cosmon_test` | **always simulated**, no exceptions | ephemeral; created, run, destroyed |

This costs almost nothing to build because it is the architecture's own claim (`ARCHITECTURE.md`
§10.8, `authority-plane` §19): the company-side path is identical for a simulated and a real
provider, so the *only* difference between `aris` and `aris_test` is provider dispatch config —
which S03-T1 introduces anyway. If a change works on `aris_test` and fails on `aris`, that gap is
itself the finding.

Two practical notes, both learned the hard way:

- **Underscores, not dashes.** A company name becomes a Postgres schema name (`[a-z_][a-z0-9_]*`);
  `aris-test` is rejected at creation. Sprint 02's comparison harness hit this.
- **Teardown needs a caller.** `OrgIntel::drop_schema` has existed unused since sprint 01 and was
  nearly purged as dead code. Ephemeral companies are its caller: `restless down --destroy` should
  remove container, volume and schema together, and the sprint-02 fix that lets a cached handle
  survive its schema being dropped is what makes reuse of a name safe.

**No credential for a real provider is ever configured for a `_test` company.** Not "should not" —
the dispatch table has no entry, so the failure mode is a simulated send, not a real one.

## Explicitly out of scope

Deferred because no run has demanded them, per `orgintel §11.1` and CLAUDE.md ("observe before
modelling"):

- **The Authority Kernel proper** — capability grants, policy/capability language, the full `§5`/`§6`
  policy engine. Deferred with its live trigger (sprint-02's posture). Sprint-03 approval is a typed
  check.
- **Infisical self-hosting** — deferred with trigger (N provider credentials / a second operator).
- **Cold-outreach-at-scale infrastructure** — warmup, dedicated IP, consent/compliance tooling. A
  different problem from "which transactional API." Revisit only if Aris's real bottleneck is cold
  prospecting.
- **Payments (Stripe), `web.deploy` live, Thymelake `order.receive`** — they ride the same ingress
  rail afterward, cheaper once it exists. Not this sprint.
- **Sprint-02 T5 (Runtime Bridge) full scope** — sprint-03 T4 is a credential slice of it; the
  bridge's process-ownership and identity fixes remain carried.
- **Sprint-02 T6 (plane split)** — sprint-03's ingress is shaped as the extractable unit; the split
  itself remains carried, lands after the live loop is proven.
- **Per-company / per-agent machine identities** — `§8.1`: not before the workload requires it.

---

## Carried from sprint 02

- **T5 (Runtime Bridge)** — still carried; sprint-03 T4 is a scoped slice (credential isolation for
  the first live provider). The bridge's four fixes (identity, process ownership, the `docker exec`
  dependency, the model-key regression) remain a boundary whose cost is set by *when*, and the email
  credential is the first wedge.
- **T6 (split `restlessd` along the plane seam)** — still carried; sprint-03's ingress + effects +
  receipts unit is what T6 will extract. Building it as a unit now is the cheap-early version of T6.
- **T9 (attention queue)** — still carried; sprint-03 T5's approval gate needs it, or a minimal
  stand-in for the first live run. **Its shape is no longer an open question:** `DeskView.needsYou`
  in `web/` already specifies it, down to the email draft the owner signs. T8 reconciles that against
  the contract's `attention_item_id` and hands T5 the result, so the stand-in is a projection of a
  settled type rather than an improvisation that a later SPA has to unpick.
