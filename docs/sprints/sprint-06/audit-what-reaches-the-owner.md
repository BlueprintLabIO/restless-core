# Audit · What actually reaches the owner, and why it does not scale

**Date:** 17 August 2026 · **Scope:** every path by which a company consumes owner attention
**Method:** read the projection and its writers, then query the live companies
**Verdict:** expansion is necessary, and its shape is narrow — one of six categories needs a second
destination. The other five must never get one.

---

## 1. Exactly two things reach the owner

`restlessd/src/attention.rs::project` is the sole composer of the owner's queue. It has two sources
and no others.

| # | Source | Written by | Can it be delegated? |
|---|---|---|---|
| 1 | Authority `approval_required` records | An effect attempt blocked on an unapproved party | **Never.** A real authority boundary. |
| 2 | OrgIntel pending `owner_handoffs` | Any actor calling `restless work handoff` | Depends on category — see §2. |

Everything else a company does — messages, events, work transitions, attempts, artifacts, gates —
reaches the owner only if they go looking. That part is healthy and is not what this audit is about.

## 2. Of six handoff categories, one is doing all the work

```rust
pub enum OwnerHandoffCategory {
    Identity, Captcha, Mfa, LegalAttestation, PaymentConfirmation,  // irreducibly human
    OwnerJudgement,                                                  // ← the whole problem
}
```

The first five are the prepared last mile: a human must present identity, pass a CAPTCHA, hold the
second factor, sign the attestation, confirm the payment. **No amount of org structure removes them,
and nothing in this audit proposes to.**

Live query, live Aris:

```
$ restless work graph --company aris   → handoffs by (category, state)
  ('owner_judgement', 'declined'): 1
  ('owner_judgement', 'pending'):  1
```

**Two of two. The five irreducible categories have never been raised by a live company.** Every
demand Aris has ever made on its owner through OrgIntel was a judgement call — the category that, by
definition, someone else with enough context could make.

## 3. A pending handoff stops the work

`request_owner_handoff` does three things in one transaction: insert the handoff, close the running
Attempt as `blocked`, and set the Work to `blocked`. Observed on Aris:

```
BLOCKED: exec | Review the Aris tutoring-centre offer | awaiting owner handoff a41eff58-…
```

So a judgement request is not a notification the owner can get to later. It is a **stop** on a Work
node that stays stopped until one person answers.

## 4. Why that does not scale — the arithmetic

Judgement requests scale with the number of actors producing reviewable outcomes. Owner attention
does not scale at all.

| Staff | Judgement stops in flight | Who can clear them |
|---|---|---|
| 3 | a few per day | 1 person |
| 30 | dozens per day | 1 person |
| 300 | — | 1 person |

The failure mode is already in the record one level down. S05-T7: *Kimi exhausted its allowance after
54 tool calls and stopped the singleton Exec.* That is the same shape — all coordination on one
actor — and it stopped the company. The owner is the next singleton up, and unlike the Exec they
cannot be failed over.

S05-T8 records the consequence already arriving: *escalated machine work to the owner.*

## 5. The schema could not express a second destination

```sql
CREATE TABLE owner_handoffs (
    ...
    requested_by TEXT NOT NULL REFERENCES actors(id),   -- who asked
    category     owner_handoff_category NOT NULL,       -- what kind
    state        owner_handoff_state NOT NULL           -- pending/resolved/declined/withdrawn
);
```

There is no column for *who owes the answer*, because there was only ever one answer: the owner. The
name `owner_handoffs` encodes the assumption. This is not an oversight — it was correct while the
company had one coordinator — but it is the thing that has to change.

## 6. What was changed, and what deliberately was not

**Changed** (migration `0007_teams_and_escalation.sql`):

- `teams` with an accountable `lead_actor_id`; `actors.team_id`.
- `owner_handoffs.assigned_to` — `NULL` means the owner, preserving the meaning of every existing row.
- `owner_handoffs.escalated_from` / `escalated_at` — so a lead cannot become a silent filter.
- `request_owner_handoff` assigns `owner_judgement` from a team member to that member's lead.
- The owner queue filters to `assigned_to IS NULL`.

**Deliberately not changed:**

- **Authority.** No kernel record gains a team. A lead grants no effect permission, no budget, no
  credential scope, no approval right. `approval_required` still goes to the owner and always will.
- **The five irreducible categories.** They bypass team routing entirely, asserted for all five by
  enumerating the enum, so a future sixth category cannot be added without deciding its routing.
- **The owner's ability to see everything.** The change filters *whose queue an item sits in*, never
  what the owner may read. `restless judgement --as <lead>` shows any lead's queue.

## 7. What this does not yet fix

Judgement now stops *arriving* at the owner. It does not yet *get answered*, because *nothing wakes a
lead when its queue grows*. `schedule.rs:139` wakes only `exec`; staff start only through
`dispatch_claimed_work`.

So the honest description of the current state is: **the load is redirected and attributed, not yet
absorbed.** The missing piece is a single capability — *start a turn for a non-Exec actor because
something is addressed to it* — which is the same gap S06-T2 found for owner mail. It is the next
thing to build, and until it exists a lead is a correct queue with nobody reading it.

## 8. Risk disposition

| Risk | Disposition |
|---|---|
| A lead silently swallows judgement the owner needed | **Guarded** — `escalated_from`/`escalated_at`; disband returns a fall-through count; unreasoned escalation is refused |
| "Lead" drifts into an authority tier | **Invariant** — no kernel record gains a team field; not relaxed under schedule pressure |
| The owner loses sight of delegated decisions | **Accepted, mitigated** — a filter on queue ownership, not on visibility |
| Teams drift from the real Work graph | **Accepted** — coordination state is repairable and regenerable by design (§4.4) |
| A saturated lead becomes the new bottleneck | **Pending fix** — this is S05-T7 one level down; the recorded fall-through makes it visible, and a second level is a decision for evidence, not now |
