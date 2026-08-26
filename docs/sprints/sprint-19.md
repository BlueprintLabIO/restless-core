# Sprint 19 — Owed work reaches the person who can act on it

**Status:** All five tickets implemented on `dev` and covered by focused tests. The live end-to-end
probe — a running daemon carrying these changes, against a `_test` company — is an open gate: the only
daemon on this machine is serving a live company mid-run.

**Date:** 26 August 2026

**Depends on:** Sprint 16's outcome-review handoff and Sprint 17's supervised operation. This sprint is a
repair sprint that runs **before** Sprint 18's Dogfood 3, because three of Sprint 18's success-contract
items (8, 10, 11) assume the owner actually receives what the company prepared.

**Evidence:** live `restyle` company state on 26 August 2026 (four owner handoffs, two of which routed
through `assigned_to='exec'`); the scheduler's own unit test
`restart_recovery_distinguishes_missed_interrupted_and_completed_wakes`, which currently *asserts* the
lossy behaviour; `crates/restlessd/src/schedule.rs` owed-work conditions; `crates/restlessd/src/approval.rs`
best-effort announcement.

**Spec refs:** `ARCHITECTURE.md` §3.2, §4.4, §4.5 and §16; `orgintel` §6–§8 and §10;
`authority-plane` §6.4–§6.5; `owner-cockpit` §2–§4; `cross-layer-contract` §3.1.

**Salvage:** No legacy control-plane machinery. Reuse the existing OrgIntel messages/`read_at`,
`owner_handoffs`, LISTEN/NOTIFY wakes, Authority records and the Svelte owner design system. Do not lift a
delivery lifecycle, notification service, outbox protocol or second visual identity.

---

## Observed product gap

Three owner-reported failures from real use of a live company. They are not three unrelated bugs; two of
them are the same misclassification.

1. **Prepared work never reaches the Exec chat.** A Work item finished, or reached the point where it
   needed judgement, and nothing surfaced. The owner's own hypothesis — "once-delivery: a pending item
   that missed the first delivery is never checked again" — is correct, and the mechanism is exact.

   The only way an ordinary judgement becomes owner Attention is `lead → exec → owner`: the attention
   projection shows a handoff only when `assigned_to IS NULL`, and only Exec may set that. Exec's owed-work
   detection is therefore the single point of failure for owner attention. Both of its durable conditions
   are **edge-triggered on wake timestamps** rather than on the owed fact itself:

   - `schedule.rs` fires Exec for an assigned judgement only when
     `max(escalated_at, created_at) > latest_event_at("wake")`. `latest_event_at("wake")` is the last Exec
     wake *for any reason at all*. One unrelated wake — an owner chat message, a schedule — moves that
     watermark past the handoff, and the handoff is never a trigger again.
   - `recover_exec_conversation` only considers messages `from_actor == "owner"`, so a lead's message
     ("the outcome is prepared for review") has **no durable recovery path at all**; it is delivered only
     by the live `NOTIFY` and an in-memory `WakeClaims::pending` entry, both of which a daemon restart or a
     dropped `PgListener` destroys.
   - `exec_conversation_is_owed` then decides delivery by comparing the message time with the last wake
     window. A message that arrives *during* a wake that started before it is treated as observed, although
     the wake's context was assembled before the message existed and `run_exec_turn` never marked it read.
     A health-gated `blocked_wake` — which never assembles context and never runs a model — records
     `wake_end` and silences the message permanently by the same comparison.

   All three read a durable per-fact question ("has this exact thing been delivered?") off an unrelated
   global watermark. `read_at` already answers it for messages; nothing answers it for handoffs.

2. **Approval decisions do not consistently become company work.** `approval::grant` writes the durable
   Authority record and then sends the Exec message on a best-effort path that only logs a warning on
   failure; `approval::decline` and `approval::revoke` tell the company **nothing at all**. Any lost
   announcement leaves the Work blocked on an authority question the owner has already answered, and the
   announcement itself is an ordinary Exec message — so it inherits every delivery defect in (1).

3. **A finished outcome reaches the owner as a dead end.** On the same company the accountable actor
   prepared `/company/outputs/redesign/2026-08-larder-sample/index.html` — a complete, self-contained
   23,114-byte web page — and linked it as the ReviewTarget. The cockpit answered "**The website is not
   ready yet.** This outcome does not have a directly reviewable website" while looking straight at the
   website. `attention::project` understood exactly two target kinds, a loopback HTTP service and a
   Markdown/plain-text file; everything else fell through to `None`. `ARCHITECTURE.md`'s outcome-native
   review names live sites, playable builds, rendered documents, PDFs, images, audio and video as the
   native experiences — only two of those could ever reach the owner, and the copy for the rest asserted
   something false rather than saying what could not be opened.

4. **The Work detail page presents a machine contract as if it were prose.** `work.outcome` on the live
   company runs 1,500–4,250 characters of imperative instructions to a model, in a single unbroken
   paragraph, rendered verbatim as the first thing the owner reads. The layout compounds it: the reading
   column caps at `76ch` inside a `minmax(0, 1fr) 210px` grid, so on a wide screen a long wall of text sits
   on the left with several hundred pixels of dead space between it and a thin rail of three short facts.

   The general form of (3) is the important one: **anything the owner sees is currently the machine's
   instruction, displayed to a human.** `OwnerBrief` already exists because handoffs hit exactly this
   problem; Work records hit it too, and nothing tells an actor that a field it writes will be read by a
   person.

## Founders' decision

> **Owed work is a durable fact about the thing owed, not a timestamp compared with an unrelated wake.
> Record delivery where the fact lives, re-derive what is owed on every scan, and stop when the fact is
> delivered — not when the clock moves.**

and

> **Every field the owner reads is a piece of writing addressed to a person. The exact machine contract
> stays exact; the actor that authors it also opens it with something a non-technical owner can read.**

and

> **Whatever the company presents to the owner must actually be viewable. If the cockpit cannot open a
> prepared outcome, it says what it is and shows the evidence — it never claims the outcome is not
> ready.**

## Problem classification

**Deterministic and enumerable** — therefore a durable query, never a heuristic: whether a message is
unread; whether a handoff is pending; who it is assigned to; whether an assignee has been given it;
whether an Authority decision has been announced to the company. These were all being inferred from wake
timestamps, which is the misclassification this sprint removes.

**Judgement and open-ended** — therefore left to the model: whether a pending judgement should be
escalated, resolved or left; what a prepared outcome means for the business; how to word the owner-readable
opening of a Work outcome. The repair adds no router, threshold or template to any of these.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| Level-triggered owed work becomes a 5-second spend loop | **Invariant** | Delivery is durable per fact (`messages.read_at`, `owner_handoffs.delivered_at`) and is written only by a turn that actually completed; a wake that never ran a model never marks anything delivered |
| A health-gated or provider-failed wake spins the scan | **Guarded** | Bounded in-memory failure backoff per company, cleared by the next successful turn; a restart clears it and retries once |
| Marking a handoff delivered hides it from the owner forever | **Accepted** | Delivery gates the *trigger*, not the context: every pending assigned handoff still appears in the assignee's context and in `blocked-on-a-person` org signals on every later wake |
| Reconciling Authority decisions duplicates announcements | **Invariant** | The announcement is idempotent on the exact Authority record id, recorded as an OrgIntel event before the scan can repeat |
| Reconciliation turns OrgIntel into a second writer of authority | **Invariant** | Authority remains the only writer of the decision; OrgIntel receives a projection and the reconciler only repairs a missing projection |
| An actor ignores the owner-readable instruction and still writes a wall | **Accepted** | The cockpit bounds the damage: the exact contract is present but no longer dominates the page. Compliance is measured in the next dogfood run, not asserted here |
| The layout change imports a second visual identity | **Invariant** | Existing tokens, type scale and Svelte design system only; no new component library, no React |

## Layer slices

| Concern | Authoritative owner | Sprint 19 responsibility |
| --- | --- | --- |
| Owed messages, owed judgement, delivery record | OrgIntel | Own `delivered_at` on `owner_handoffs`; expose undelivered owed facts as a query, not a timestamp |
| Waking the accountable actor, failure backoff | Runtime scheduler | Re-derive owed work every scan from durable facts; back off a failing substrate instead of a silent watermark |
| Approval decisions | Authority Plane | Stay the only writer; hand OrgIntel one idempotent, reconcilable projection of each decision |
| Produced files and their bounded observation | Company Runtime | Observe and serve one prepared file and its own directory, read-only, through the existing isolated review origin |
| What the owner reads | Owner cockpit + prompts | Present the outcome without a wall, open every format it can, and instruct every accountable actor that owner-facing fields are writing |

## Success contract

1. A judgement assigned to Exec wakes Exec exactly once per undelivered generation, regardless of how many
   unrelated wakes happen in between, and stops waking it once delivered.
2. A message from a lead to Exec is recovered after a daemon restart with no `NOTIFY` and no in-memory
   queue.
3. A `blocked_wake` never marks an owed fact delivered, and never spins the scan.
4. Escalating, refreshing or reassigning a handoff makes it owed to its new assignee again.
5. Every Authority approval decision — granted, declined, revoked — reaches the company exactly once, and a
   decision whose announcement was lost is repaired by the next scan without duplicating it.
6. The Work detail page has no dead horizontal zone at desktop width, and the exact outcome contract is
   present but does not dominate the first screen.
7. Exec and lead context both carry the rule that owner-facing fields are written for a person, and the
   `work add` instruction states it at the point of authoring.
8. A produced page, document, image or recording opens natively in the review frame; a target that
   cannot be opened says exactly that and shows the recorded evidence, and no request escapes the
   prepared outcome's own directory.

## Non-goals

- a delivery/notification lifecycle, outbox table, retry policy engine or read-receipt protocol;
- a second writer of approvals, or an approval state machine;
- a model call inside the owner BFF to rewrite or summarise machine text;
- a new Work field, entity or renderer for owner-readable copy — the prompt change is tested first;
- a general file-serving API, an export path, or a universal renderer — a file review is scoped to one
  ticket, one Runtime generation and one prepared outcome's own directory; and
- redesigning the cockpit beyond the Work detail and review surfaces named in the report.

## Ticket decomposition

Status lives only in this checklist.

| Status | Ticket | Slice | Observed friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [x] | [**S19-T1 · Make owed work a durable fact, not a wake watermark**](sprint-19/S19-T1.md) | OrgIntel + scheduler | Prepared work never reaches the Exec chat; a lead's message has no durable recovery path | `exec_conversation_is_owed`, the owner-only recovery filter, the `> latest_event_at("wake")` handoff comparison and their tests |
| [x] | [**S19-T2 · Reconcile every authority decision into the company**](sprint-19/S19-T2.md) | Authority + OrgIntel | A granted approval can be durably recorded and never announced; decline and revoke announce nothing | Best-effort announcement inside `grant`, and the silent decline/revoke paths |
| [x] | [**S19-T3 · Present the Work outcome to a reader**](sprint-19/S19-T3.md) | Owner cockpit | A 3,000-character machine contract is the first thing on the page, with dead space beside it | The `76ch`-inside-`1fr` dead zone and the unbounded raw contract block |
| [x] | [**S19-T4 · Say that owner-facing fields are writing**](sprint-19/S19-T4.md) | Prompts + context assembly | Owner-visible records are authored as instructions to a model | Nothing yet; this is the cheapest test of whether a Work-level brief field is needed at all |
| [x] | [**S19-T5 · A prepared outcome the cockpit can actually open**](sprint-19/S19-T5.md) | Company Runtime + owner cockpit | A finished `index.html` reached the owner as "this outcome does not have a directly reviewable website" | The `_ => None` review dead end and its false "not ready" copy |

## Verification

Headless first, per repo convention.

- `cargo test -p restless-orgintel -p restlessd` for the durable owed-fact invariants and the approval
  reconciler, including the adversarial cases: unrelated wake in between, blocked wake, restart with no
  `NOTIFY`, repeated reconciliation.
- A live `_test` company probe for the end-to-end path; never the live `restyle` company. The exact
  `stat`/`head` reads a file review issues are proved read-only against the real `restyle` ReviewTarget,
  because that is the target that failed.
- `pnpm check` and a real desktop + mobile render of the Work detail page against live company data for
  T3, compared with the references named in `docs/FRONTEND_DESIGN_REFERENCES.md`.
- T4's evidence is the assembled context, not model compliance. Compliance is a Dogfood 3 observation.
