# Sprint 115: approve by reply

**Status: planned, not started (10 Nov 2026 AEDT). Needs a new ADR, part A.**

The owner has a day job and wants one simple morning routine: the day's emails arrive in their own
inbox, they reply "approve", and the emails go. Today every inbound email is untrusted, including
the owner's, and approval happens only in the Inbox.

That blanket rule is too blunt for the owner's own address. A reply can be proven to come from the
owner, provided Restless checks it and the model does not. A known client's email stays evidence.

## What the owner decided

- The owner can release a prepared batch by replying from their own email.
- Known clients and everyone else do not get this. Their email wakes the company and informs it, as
  it does today, and it never grants authority or changes the rules.

## What exists to build on

- Weekday schedules at a local time and timezone (`restless schedule create-responsibility`).
- Signed Resend webhooks, `email.received`, deduplication and the Exec wake (`inbound.rs`,
  `incoming_email.rs`).
- Resend's retrieve-received-email response has `authentication` with `spf`, `dkim` and `dmarc`,
  computed by the receiving server, and `raw` for the original message. Restless reads this response
  today but does not use `authentication`.
- Exact email approval: `email_approval.rs` binds sender, recipient, digest and effect key, and a
  changed draft asks again. Decisions run through Authority's Undo window (`approval_decisions.rs`).
- Telegram's approval model (ADR 0015): the button carries only an id, and a delivery row binds it
  to the exact question. This sprint reuses that binding idea, not the transport.

There is no batch today. Each email is its own approval, which is why a morning routine needs part
C.

## A. ADR and spike (one day at most)

1. Write ADR 0018: a verified owner reply can release one exact prepared batch. Everything else in
   an inbound email stays untrusted. Restless decides, never the model.
2. On a real Gmail reply to Reno's receiving domain, record what `authentication` actually contains,
   and how DKIM and DMARC alignment to the sender's domain are expressed.
3. Confirm that a one-time address on the receiving domain, such as `approve+<token>@…`, reaches the
   webhook and shows in `to` or `received_for`.
4. Confirm how the host can send the owner's morning email outside the agents' sending permission.

**If `authentication` is missing or cannot show alignment, stop and bring the options to the owner.**
The fallback is the one-time address plus the From check, which is weaker.

## B. The owner's approving address

1. One address per company, registered by the signed-in owner in Company › Integrations › Email.
   Adding or changing it needs an Inbox press, and can never be done by email or by an agent.
2. Off by default, and one switch turns it off.
3. It is separate from the mandate's `own_addresses`, which say who is exempt from outreach limits.

## C. A batch is one exact thing to approve

1. Exec prepares a batch from existing email requests. Each email keeps its own digest and effect
   key, and the batch has a digest over all of them.
2. The batch shows as one Inbox card listing every email. Approving there still works, and is the
   fallback for everything below.
3. Any change to any email makes it a new batch, and the old one can no longer be approved.

## D. The morning email

1. A weekday schedule has Exec prepare the batch, and the host then emails it to the approving
   address: each recipient, subject and full text, plus a link to the Inbox card.
2. Its Reply-To is a one-time address holding a random token of at least 128 bits. Only the token's
   hash is stored, bound to the batch digest. It expires at the end of that day in the company's
   timezone.
3. The email contains **no approving link**. Mail scanners open links on their own. The link only
   opens the card.

## E. Restless checks the reply, before any model sees it

A reply releases the batch only if every check passes:

| Check | Fails when |
| --- | --- |
| Signed webhook, body fetched from the Receiving API | Either is missing |
| Addressed to the one-time address | Token unknown, used, expired or for another batch |
| Exactly one From address, equal to the registered one | Different address, several addresses, or a look-alike in the display name |
| DKIM and DMARC pass, aligned to that address's domain | Anything other than a clear pass, including `null` |
| First line of the new text is "approve" | Anything else, including "approve" appearing only in quoted text, or "don't approve" |
| Batch digest unchanged, and every email within current limits | The batch changed or a limit would be passed |

1. Pass: the token is consumed atomically and the batch goes through the same grant path as an Inbox
   approval, as the owner. Exec receives only the Authority result, never the text of the email.
2. Only "approve" is recognised. Any other reply goes to Exec as ordinary untrusted email, and the
   batch stays waiting.
3. Fail: nothing is sent, and the card stays. The Inbox shows which check failed, at most once per
   batch, so forged mail cannot flood the owner. No failure notice is emailed to the sender.

## F. What a release can do

1. It releases that batch only. It can never raise a limit, add a kind of recipient, change a
   setting or approve anything else.
2. The sends wait out an Undo window, starting at 5 minutes rather than the Inbox's 5 seconds, since
   the owner is on email.
3. The owner gets a short confirmation with a link to the Inbox, where Undo lives. Undo needs a
   signed-in session.
4. Exact emails go once, as today, and an uncertain send is never retried.

## Accepted risk

Anyone who controls the owner's Gmail can release that day's batch, within its limits. They cannot
do anything else from there. Two-step verification on that account is part of turning this on, and
the setup screen says so.

## Done when

- **The real path:** the owner replies "approve" from real Gmail to a batch addressed only to the
  owner's own addresses. It goes out once, after the Undo window, and the Inbox card clears. Real
  outreach comes only after this passes.
- **Every refusal, tested against the engine:** forged From with failed DKIM, right From with a
  wrong token, a reused token, an expired token, a changed batch, a display-name look-alike, two
  From addresses, DMARC `null`, "approve" only in quoted text, "don't approve", a replayed webhook,
  a release that would pass a limit, and a forwarded email replied to from another address.
- **Not agent-reachable:** Exec cannot release a batch through its session, its mail or a crafted
  message. An injection in an email body changes nothing.
- **Live per role:** owner sets up, turns off, approves and undoes on desktop and phone. A member
  cannot register an address.
- **Quality gates:** no new jank, and the morning routine adds no tokens beyond Exec's scheduled
  preparation turn.

## Out of scope

- Approving by reply for anything other than an email batch.
- Clients or members approving by email.
- One-click approve links.
- Telegram batch approval, which could reuse part C later.
- Replying to edit a batch. The owner changes it in chat or in the card.
