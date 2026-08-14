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
