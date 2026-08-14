# Sprint 03 run report

**Status:** in progress — T1–T5 built and verified live; T0/T6 executing.
**Question the sprint exists to answer:** can a real provider run through the
effect surface, and can the world push back — with the runtime still closed and
the Authority Kernel still deferred?

Nothing below is described as working unless it was executed with stated inputs
and observed output (CLAUDE.md → "Never report green without running it").

---

## What landed

| Ticket | Commit | How it was verified |
|---|---|---|
| T1 · Provider dispatch + Resend adapter | `b3d00b8` | real email sent; Resend's own record says `last_event: delivered` |
| T4 · Credential indirection | `b3d00b8` | exact-secret grep over container env **and** the whole `/company` volume: 0 |
| T2 · Event ingress | `7f194aa` | forged → 401, genuine → 202, redelivery → still one `inbound_effect` |
| T3 · Inbound → effect → message → wake | `7f194aa` | `scheduler wake reason="event: mail from world"`, exactly 1 wake from 1 reply + 1 redelivery |
| T5 · Approval on first contact | `3d11b5c` | refused a new party naming the unblocking command; `approve` wrote it; config round-tripped |
| Purge · retired comparison | `7f194aa` | 3 companies, volumes, schemas, configs and 5 spend records removed |

## The evidence, in order

### T1 — a real provider behind the effect surface

`ARCHITECTURE.md §10.8` claims the company-side path is identical for a
simulated and a real provider. That claim had never been tested because there
was nothing real to test it against. It now holds: the Exec's call is unchanged,
and only the dispatch table decides which world it reaches.

```
receipt   provider "resend", message 1e5a2dd3-…, party yaillives@gmail.com
Resend    last_event: delivered, from aris@blueprintlab.io
```

Counted from the provider's record, never from the company's account — `§20.2`,
and the reason is on file: Aris once reported £45 of revenue that its receipts
put at £18 confirmable.

### T4 — the credential regression is closed for email

```
container env      : 0 occurrences of the exact key
whole /company vol : 0 occurrences of the exact key
daemon process     : 1 (it must be there; that is the point)
```

The first grep used `re_` as the needle and matched three unrelated files —
recorded because a sloppy check that happens to pass is not evidence, and the
rerun with the exact 36-character secret is what the claim rests on.

### T2 — the world's front door, on its own failure boundary

```
forged signature                  -> HTTP 401, nothing written
genuine signature                 -> HTTP 202
redelivery (same provider event)  -> HTTP 202, still ONE inbound_effect
```

The timestamp window is tested separately from forgery: a captured request
replayed tomorrow carries a *genuine* signature, so those are different attacks
that present identically. Rotation sends two signatures in one header and either
may match, or every delivery breaks during a rotation window.

**No secret means no listener.** The daemon logs that it cannot receive rather
than opening an unauthenticated public port.

### T3 — the loop closes without the owner typing

```
inbound reply projected into the Exec inbox   company=aris from=yaillives@gmail.com
scheduler wake company="aris" reason="event: mail from world"
aris wakes from 1 reply + 1 redelivery: 1
```

The ordering is the ticket. Authority **authors** the inbound effect (the world
did this); OrgIntel **projects** it as a message. Backwards would make the
company's own coordination store the record of an external fact, which
`cross-layer §3.1` rule 3 forbids.

### T5 — first contact needs a human yes

```
new party, real provider -> refused:
  "aris wants to email.send to stranger@example.com for the first time,
   through a REAL provider … approve with `restless approve -c aris --party …`"
restless approve         -> "stranger@example.com approved for real effects from aris"
config round-trip        -> providers and credentials intact
```

Per-party, not per-send: gating every send makes the owner a dispatcher, which
`owner-cockpit §2.3` rejects. The test party was removed from the live config
afterwards.

---

## Blocker: the reply leg needs one DNS record

**Probed, not assumed.** `blueprintlab.io` reports
`capabilities: {sending: enabled, receiving: enabled}` and DKIM/SPF verify, but:

```
Receiving record : pending
MX (8.8.8.8)     : route1/2/3.mx.cloudflare.net
Resend expects   : inbound-smtp.ap-northeast-1.amazonaws.com
```

Mail to `@blueprintlab.io` reaches **Cloudflare Email Routing**, not Resend, so a
real reply never becomes a webhook. Resend's API has no endpoint that creates an
inbound address (`/inbound`, `/addresses` return 405 for every method), so this
cannot be worked around in code.

**Owner action, and the cheapest form is a subdomain:** point
`reply.blueprintlab.io` MX at `inbound-smtp.ap-northeast-1.amazonaws.com`, verify
it in Resend, and set Aris's `from_address` to that subdomain. The apex domain's
existing routing is untouched.

Until then the ingress is verified by replaying a genuinely signed payload —
a real test of signature, dedupe, projection and wake, and **not** a test of DNS.
That distinction is stated rather than blurred.

## Standing infrastructure

```
ngrok tunnel      https://b015-144-6-2-32.ngrok-free.app -> 127.0.0.1:7792
resend webhook    ce91c3e0-c495-43e5-9811-470389f04a46
                  events: email.received, email.delivered, email.bounced
signing secret    stored in .env (gitignored), loaded by the daemon at boot
```

The ngrok URL is ephemeral and dies with the tunnel; the webhook must be updated
when it changes. Recorded so a later run does not mistake a stale URL for a
broken ingress.

## Incidents

**`.env` corrupted by an append.** The file had no trailing newline, so
`echo … >> .env` concatenated the webhook secret onto the `RESEND_API_KEY` line
and broke both. Caught by the daemon logging "RESEND_WEBHOOK_SECRET is not set",
repaired by splitting the line, and confirmed by `HTTP 200` from the Resend API
with the restored key. The daemon's honest startup warning is what surfaced it —
had the ingress silently started with an empty secret, the corruption would have
been found much later and by something worse.

---

## T0/T6 — the live loop, outbound

**Aris produced and sent the sample itself.** `actor: exec`, not the owner.

```
receipt   822729aa-a416-4299-a40f-f4f21ac5ae68
key       s03-w4-cem-sample-owner-v1
provider  resend, message 5406aa8a-f270-4cf2-8273-f5c00a7c0740
to        yaillives@gmail.com   from aris@blueprintlab.io
Resend    last_event: delivered
artifact  /company/outputs/cem-sample-paper1.md — 16,625 bytes
email     17,240 chars, the paper inlined in full, not a teaser
```

A complete CEM-style 11+ practice paper: 40 multiple-choice questions across
comprehension, verbal, numerical and non-verbal reasoning, in four timed
sections, **with full worked solutions and a parent marking guide**. It carries
its own IP disclaimer ("Not affiliated with CEM… no copyrighted exam content is
reproduced") without being asked to.

It also asked the owner the question the ticket wanted asked, in its own words:
*"IS THIS GOOD ENOUGH TO PUT YOUR NAME BEHIND? Reply yes / no / what you would
change. Any reply beats silence."*

### The hypothesis is the real result

`/company/org/hypotheses/cem-board-fit-line.md`, in Appendix A's template, and
it is **sourced**: every observation carries a journal citation, and it opens
*"All from my own journal and kernel receipts, not asserted memory."*

Its predictions are falsifiable with numbers (≥50% intent from CEM-fit
prospects vs 0% observed for the unmatched P8), and — the part worth noticing —
it lists what it does **not** know:

> Which target schools/regions actually use CEM vs GL vs school-written papers
> is **unverified by me** … Board mapping must be verified per school before any
> outreach claims "matches your exam".

And it refuses to record a result it does not have: *"Do not record a result
here until reply/send receipts exist."*

**Citation accuracy, checked against the ledger:** Aris cited receipt
`822729aa…` and key `s03-w4-cem-sample-owner-v1`. Both exact, and it correctly
kept the provider message id (`5406aa8a…`) as a separate thing. Sprint 01's Aris
claimed £45 of revenue that receipts put at £18; this is a different standard of
self-report, and it is the first evidence that showing a company its own
receipts each wake changes what it writes.

### Incident: I contaminated a live company's beliefs

The T3 ingress test injected a **synthetic** signed webhook — a fabricated reply
from `yaillives@gmail.com` saying *"Can you send the CEM version too?"* — into
**`aris`, a live company**, because that is where the real provider was
configured.

Aris then read it as real, and built on it. Its hypothesis now records:

> the owner's live reply (inbox, 2026-08-14 07:59) asked for "the CEM version"
> within 5 minutes of receiving a GL-style sample — **the strongest single
> demand signal so far**

and

> triggered by the owner's unprompted request — the first demand signal for CEM
> from outside the simulated panel.

**That demand signal does not exist. I manufactured it.** The sample is real and
good, the sending rail is real, and the hypothesis's *reasoning* is sound — but
its stated trigger is an artifact of my test, and a company acting on it would be
chasing a customer who never spoke.

This is the same error as the sprint-02 destructive verification against
`cosmon`: **testing against a live company because it was the convenient one.**
S03-T7's `_test` companies exist precisely for this and were not yet built when
the ingress needed exercising. The ordering was wrong — T7 should land before any
live-company ingress test, not after.

Correcting the record with Aris is required, not optional: `orgintel`'s own rule
is that receipts win over belief, and here the *belief* came from us.

### The correction, and what it proves

Aris was told the truth and given no instruction about what to conclude. It
applied the correction, then **reversed its own decision**.

It retracted four claims — one more than it was asked to. The fourth was its
own description of the sample as *"owner-reviewed"*, which nobody had flagged:

> ~~"owner-reviewed"~~ — the sample was *sent* to the owner (receipt 822729aa,
> genuinely delivered per Resend); no verdict has been received. The
> name-behind-it judgement is still pending.

It separated the platform's error from its own reasoning without defensiveness
or over-correction:

> I note for the record that the false belief came from the platform, not from
> my reasoning, and I do not distrust the inbox in general — this is one
> corrected fact.

And then, unprompted, it re-ranked its roadmap on the evidence that survived:

> **Decision: P8 alone is too thin. Do not build the CEM line now.**
> … A strictly stronger observed demand signal exists elsewhere: Maths SET 2 has
> **two** unprompted requests from converted customers (P3, P7) versus CEM's one
> from a non-customer. Demand-ranked, SET 2 is the next product.
> … The deploy repair (HTTP 404 since wake 0003) outranks both — it blocks scale
> on every line.

It kept the *test* rather than the line — the sample already exists at £0
marginal cost and is the only instrument that can turn the P8 anecdote into data
— and queued it behind two things it judged more valuable.

**This is the sprint's most important result, and it is not the email.**
Sprint 02's finding was that "the company had the evidence and did not draw the
conclusion": Aris recorded a price rejection and a segment mismatch, then kept
selling to parents. Here, given a corrected fact, it revised strategy from its
own accumulated evidence, changed its product ordering, and demoted work it had
just built — which is `orgintel §3.4` self-evolution doing the thing the spec
claims it does.

The uncomfortable half: we only got to observe it because we broke something
first. The mechanism was exercised by our contamination, not by the company's
own operation, and a correction loop that has only ever run on a
platform-injected error is not yet evidence that it runs on a real one.

---

## Sprint status

| | |
|---|---|
| Outbound loop | **works, verified end to end** — Aris built and sent a real artifact to a real inbox, `delivered`, for $0.79 |
| Inbound rail | **works, verified by signed replay** — signature, dedupe, projection, wake |
| Inbound DNS | **blocked on one owner MX record** |
| Success contract | **partially met.** One recipient received a real sample. The reply leg is untested against real DNS, and the owner's name-behind-it verdict has not arrived |

**What must land before any further live-company testing: S03-T7.** Its `_test`
companies are no longer a convenience — they are the fix for the one incident
this sprint caused.
