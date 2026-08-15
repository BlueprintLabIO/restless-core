# S05-T1 · The attention queue, as a projection

**Layer:** Owner surface, projecting OrgIntel events. No new owned concept.
**Serves:** `owner-cockpit` §5 — the Attention Inbox, Core contract, the owner's primary work queue
**Depends on:** S04-T10 (landed in sprint 04 — an attention item is resolved by an authority act, which needs a principal)
**Makes deletable:** the owner-notification-as-prose path at `effect.rs:145`

---

## The item already exists. It is addressed to the wrong reader.

`approval.rs:96` constructs the owner's ask — party, capability, provider, and the sentence explaining
why it is irreversible. `effect.rs:134` emits it as an `approval_required` event carrying
`{capability, party, provider, reason}`.

Then `effect.rs:146` `bail!`s the sentence back to **the agent that was blocked**, and the owner gets
an untyped copy as ordinary mail (`effect.rs:145`, `to: None`).

Three consequences, all observable today:

- The owner's ask arrives as prose in a message body. There is no way to list what is *outstanding*
  versus what has been dealt with.
- `restless inbox` **marks read on read** (`orgintel::mark_read` behind the `inbox` arm). The one
  notification the owner gets disappears the first time they look at their mail.
- The instruction to act — *"approve with `restless approve -c aris --party …`"* — is handed to the
  agent, which is then expected to relay it. CLAUDE.md's prepared-last-mile rule names this exactly:
  *never hand the surrounding workflow back as instructions.*

So this is not a missing feature. It is a working mechanism pointed at the wrong consumer.

## Project, do not invent

`ARCHITECTURE.md` §16.1 and §16.6: introduce a first-class entity only after repeated real scenarios
reveal the same need. There is one category of attention item in existence. Building
`owner-cockpit` §5.3's eight categories, §5.4's priority model and §5.6's learning loop now would be
the exact failure CLAUDE.md warns about — *building a Product hypothesis as though it were a Core
contract*.

What is derivable **today, with no new table**:

```
outstanding attention =
    events of kind `approval_required`
    minus those whose party has a later `approval_granted`
```

Both event kinds already exist and are already emitted. `events_of_kind` already exists on `OrgIntel`.

Two properties fall out for free, and they are the reason this shape is worth preferring:

- **No new identity authority.** The `attention_item_id` is *derived from the source event id*, not
  minted. `owner-cockpit` §14.6's `attention_source_ref` is the event; the item is a projection over
  it. Nothing becomes a second writer of anything (`cross-layer` §3.1).
- **S03-T8 item 6 holds for free.** A client that missed events while disconnected reconstructs
  correct state by refetching, with no event replay — because there is nothing to replay. The
  projection *is* the state.

## Reconciling with the SPA, which is ahead of us

`web/src/lib/model/view.ts:74` already has `NeedsYouItem`, and S03-T8 says reconcile with it rather
than invent alongside it. Adopt now:

- `id`, `kind`, `title`, `detail`, `createdAt` — the envelope's spine, and `owner-cockpit` §5.2's
  minimum.
- `kind` as an enum. Only `'email-approval'` has a producer today; the type carries the others
  (`'decision'`, `'promotion-approval'`, `'escalation'`) as declared-but-unproduced rather than
  deleted, because T3's `repo.push` is a `promotion-approval` in waiting.

Defer, explicitly:

- `NeedsYouContext`'s rich per-kind payloads. `EmailDraftView` is the tempting one — it is the exact
  draft awaiting signature — but the daemon does not persist outbound bodies in a form the projection
  can reach without a second store, and Aris only started persisting them at all last sprint. One
  category does not justify a context union.
- `NeedsYouRef.version`. It answers the double-clicked-Approve problem, which is real and which
  §4.5 lists among the five idempotency classes. It is not reachable in a terminal, where the failure
  mode is not a double click. It becomes required the moment the SPA is wired, and should be built
  then, against a UI that can demonstrate the bug.

## Scope

1. **`restless attention [-c]`** — outstanding items, oldest first (`view.ts:80`: *"null sorts oldest
   — undated items have by definition been waiting at least as long"*). Per item: id, kind, title,
   detail, age, and the exact command that resolves it.
2. **Resolution stays where it is.** `restless approve` is the writer. This ticket adds no second way
   to grant authority — one act, one writer, and T10 gates it.
3. **The owner's ask stops being mail.** `effect.rs:145`'s untyped `send_message` to the owner is
   removed; the event remains and the projection reads it. The agent still gets its typed refusal —
   that one is correct and stays.
4. **A count, for the status line.** `owner-cockpit` §12.6 and `view.ts:160`'s `needsYou: number`.

**Not in scope:** the other seven categories, the priority model (§5.4), attention-spam learning
(§5.6), defer/dismiss/delegate actions (§5.5). One category, one action, no lifecycle.

## Acceptance

Run against a `_test` company (S04-T1), so no live party is contacted.

1. An `email.send` to a never-contacted party through a **real** provider raises an item that appears
   in `restless attention` with its party and capability — and the same run's transcript shows the
   agent received a typed refusal, not a request to relay instructions.
2. `restless approve --party <that party>` is followed by the item **leaving** `restless attention`,
   observed, with no second command run to clear it.
3. Reading `restless inbox` does **not** cause the item to disappear from `restless attention` — the
   regression this ticket exists to fix, asserted directly.
4. Killing and restarting the daemon between steps 1 and 2 leaves the queue identical, demonstrating
   the projection reconstructs rather than remembers.
5. Zero `psql` in the transcript.

## What this makes deletable

`effect.rs:145` — the owner-notification-as-untyped-mail path. And the assumption behind it: that a
message in an inbox is how the owner learns something needs them. After this, mail is correspondence
and attention is a queue, which is `owner-cockpit` §5.1 as written.

---
Sprint spec: [`../sprint-05.md`](../sprint-05.md)
