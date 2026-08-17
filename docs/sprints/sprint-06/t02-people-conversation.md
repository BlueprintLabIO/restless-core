# S06-T2 · People is a conversation surface

**Layer:** Owner surface. Reads OrgIntel actors, Work and messages; writes only through the existing
`POST /api/companies/{c}/actors/{actor}/conversation` path. No new owned concept.
**Serves:** `ARCHITECTURE.md` §4.4 and `CLAUDE.md` — *let intelligence do the work*; the owner's
primary act with a person is to talk to them.
**Depends on:** S06-T1 (the layout it rearranges).
**Makes deletable:** the permanent Exec rail on the People route.

**Status: landed.**

---

## The friction

Selecting a person on People produced a four-quadrant metrics panel: current focus, four counts,
a mandate table, recent outcomes. It answers *what has this actor done*. It offers no way to say
anything to them.

The only conversation surface was the Exec rail, permanently mounted on every route — which made the
People page render the same conversation twice whenever the Exec was the selected person: once in the
400px rail, once as the selected row's profile. Two representations of one actor, one of which could
be talked to and one of which could not.

The transport was never the obstacle. `getActorConversation(company, actor)` and
`sendActorMessage(company, actor, …)` are already actor-generic, and `owner.rs` resolves the actor
from `list_actors()` — the Exec has no privileged path. The surface simply never used it for anyone
but `exec`.

## Scope

1. **The centre column becomes the conversation with the selected person.** Same transcript treatment
   as the Exec rail — Markdown for agent messages, attachments, intent receipts, day separators, the
   waiting indicator — reading from `getActorConversation(companyId, selectedId)` and writing through
   `sendActorMessage`. Switching the selection switches the transcript.
2. **The profile becomes a right-hand evidence column**, ~340px, single-column: presence, role,
   current focus, the four counts, mandate, recent outcomes. It keeps every value it had; it stops
   being the primary surface.
3. **No Exec rail on `/[companyId]/people`.** `AppShell`'s `rail` becomes optional and the layout
   omits it for this route. On every other route the rail is unchanged.
4. **Composer state is per-actor.** Switching selection does not carry a half-typed message to a
   different person.
5. **The composer opens only where a message is actually delivered** — see the finding below.

## What building this found: owner mail reaches only the Exec

The ticket was drafted assuming the composer should open for anyone and glass-lock on runtime
presence, the way the Exec rail does. Reading the delivery path before wiring it showed that is wrong,
and running it confirmed why.

`restlessd/src/schedule.rs:139` is the **only** handler for the message notification, and it is gated:

```rust
Some("message") if value["body"]["to"] == "exec" => { … fire_exec(…) }
```

A message addressed to any other actor is inserted, fires its NOTIFY, and matches nothing. Staff read
owner input through Work feedback (`staff.rs:173`, "Owner/operator feedback through message …"), which
is keyed to a Work revision — not through `owner_conversation`, which only the owner HTTP endpoint
reads.

Probed on the `aris_feedback2_test` company (never against a live company — `evaluation-dogfood`
§9.6.1):

```
$ restless message --company aris_feedback2_test --to staff-email-writer "…delivery probe…"
{ "message_id": 19 }

$ restless events --company aris_feedback2_test | head
171 wake_end     exec        ← newest event, from an earlier wake
170 turn_usage   exec
169 model_attempt exec

$ restless inbox --company aris_feedback2_test --as staff-email-writer
[ { "id": 19, "to_actor": "staff-email-writer", "read_at": null, … } ]
```

Recorded, unread, and **no event produced**. Nothing woke.

So an open composer on a staff member would be a simulated capability — a message the owner believes
was sent, indistinguishable from a real one, entering the company record and staying there. Instead:

- **Exec** — composer open; this is the real path.
- **Everyone else** — no composer. The surface states what actually happens and points to the two
  paths that work: talk to the Exec, or leave feedback on the person's Work.

The transcript is still read for every actor, because reading is real.

This is friction for **S06-T5**, where an owner ↔ lead conversation is a success criterion. T5 must
open this path for at least the lead, or it cannot be met. It is recorded in T5's scope.

**Not in scope:** work-scoped conversation (`work_id` on the conversation endpoints) — it exists in
the API and stays unused here; group or team-wide conversation, which is S06-T5's question; any
change to how messages wake an actor.

## The layout

```
┌────────────┬───────────────────────────────┬──────────────┐
│  People    │  Conversation with            │  Evidence    │
│  index     │  the selected person          │              │
│  240px     │  1fr — primary                │  340px       │
│            │                               │  presence    │
│  · exec    │  transcript                   │  focus       │
│  · …       │                               │  counts      │
│            │  ─────────────────────────    │  mandate     │
│            │  composer                     │  outcomes    │
└────────────┴───────────────────────────────┴──────────────┘
```

Below 1180px the evidence column moves under the conversation rather than shrinking below legibility.

## Why the Exec rail stays everywhere else

The rail is not being deprecated. `owner-cockpit` §5's right-hand executive chat is the surface for
the *company*, and Attention, Work and Authority are company surfaces. People is the one route where
the centre is already about a specific actor, so a permanent second conversation with a different
actor competes with it rather than supporting it.

## Verification

Run, with results:

- `cd web && npm run build` — built in 1.39s, no errors.
- `cd web && npm run check` — `306 FILES 0 ERRORS 0 WARNINGS`.
- `grep -rn "situation" web/src` — no matches (T1's deletion holds).
- Delivery probe on `aris_feedback2_test` — transcript above; message recorded, no wake. This is what
  closed the composer for non-Exec actors.

Visual supplement: the People route shows three columns and no Exec rail; every other route keeps it.

## What this does not claim

It does not claim the owner *should* talk to individual staff — S06-T4/T5 argue the opposite, that
the owner should mostly talk to a lead. This ticket makes the surface capable of a conversation with
whoever is selected; T6 makes the lead the obvious selection.
