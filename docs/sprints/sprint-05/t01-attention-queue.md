# S05-T1 · Unified attention projection and SPA review loop

**Layer:** Owner surface, projecting source-owned OrgIntel, Authority and Runtime references. No new
authoritative attention store.
**Serves:** `owner-cockpit` §5 — the Attention Inbox as the owner's primary work queue
**Depends on:** S04-T10 (owner principal), S05-T4 (generated OrgIntel wire rows)
**Makes deletable:** owner-notification-as-prose at `effect.rs:145`, the SPA's fixture-only company
desk, and `NeedsYouContext` as four hard-coded product workflows

---

## The run proved a queue; it also disproved the email-only shape

`approval.rs:96` already constructs an exact owner ask for first contact. `effect.rs:134` emits the
source event, then `effect.rs:145` copies it into ordinary mail and `effect.rs:146` returns prose to the
blocked agent. That loses outstanding/resolved state and asks the agent to relay the owner's work.

Sprint 04 then produced two other owner moments that cannot fit
`approval_required - approval_granted`:

- a real Git compare link that needs owner review/merge;
- a production gate that must remain open until the centre page is observed live.

The current SPA is further ahead in appearance than semantics. It renders a left queue and selected
detail pane from `$lib/fixtures/cosmon`, but `view.ts` recognises only `decision`, `email-approval`,
`promotion-approval` and `escalation`; its buttons are inert. Adding `browser-handover`, then
`github-review`, then one kind per website would preserve the wrong abstraction.

## Project the common envelope, not a universal command

The queue is a read projection over source-owned requests. The first live envelope contains:

```text
id derived from source reference
source plane + source object/reference
category
title
what happened
why it matters
recommendation
specific owner action requested
what happens if the owner does nothing
deadline/review date where present
evidence/artifact references
optional runtime attach reference
cost, consequence and reversibility where present
whether work can continue while waiting
available actions supplied by the source
created/resolved timestamps
```

This is not a new mutation algebra. Each action routes to its existing owner:

| Source | Examples in this sprint | Resolution writer |
|---|---|---|
| Authority | four exact `customer-contact.email` first-contact grants | existing approval/grant operation |
| OrgIntel | merge recommendation, outcome review or explicit owner handoff | decision/directive/Work operation |
| Runtime reference | open the prepared browser/desktop | no organisational resolution; attach is inspection |

The SPA never marks a source request resolved because a row was read, dismissed locally, a desktop
closed, or a lease ended. A refetch reconstructs the queue from the authoritative source state.

## Only proven categories

Sprint 05 implements the smallest categories its real run produced:

- **approval** — a source-owned Authority request;
- **outcome review / decision** — a prepared Git change and recommendation;
- **failure/recovery** — the production gate or Runtime attach failure.

The category is presentation, not the source object's identity. Opportunity, contradiction,
information, attention-learning, visible priority formulae and arbitrary user-created categories stay
deferred until a run produces them.

## The owner gateway seam

The static SPA cannot safely reach a Unix socket, Postgres, Docker or a VNC endpoint. Add one narrow,
authenticated owner gateway to the existing modular daemon:

- serves the built SPA and the owner-ready `DeskView` projection;
- subscribes to meaningful updates only: added/resolved attention, source health and Runtime attach
  state;
- invokes the same typed application operations as the CLI for owner actions;
- exchanges opaque Runtime attach references through S05-T5;
- attributes every write to the single V0 owner principal;
- exposes no generic shell, file, Git, browser-action or raw-database endpoint.

The exact HTTP route names are not the contract. The contract is one authenticated owner boundary,
typed inputs and outcomes, and no second writer. SSE is sufficient for queue changes; raw ACP tokens,
shell output and browser commands do not enter this stream.

V0 authentication is deliberately one-owner:

- a generated owner credential is stored host-side as a hash;
- login exchanges it for a Secure, HttpOnly, SameSite session cookie;
- state-changing requests require same-origin protection;
- the listener defaults to loopback and remote exposure requires TLS;
- there are no invitations, human role editor or multiplayer presence.

## SPA interaction

Retain the existing queue/detail composition at `web/src/routes/[companyId]/+page.svelte`:

```text
queue item
→ inspect recommendation and evidence in the detail pane
→ open direct artifact or enter browser focus mode
→ accept, reject, direct, ask, defer, or invoke the source-specific authority action
→ refetch/reconcile source state
→ move to next item
```

For Sprint 05, only actions backed by a real source operation are enabled. An unsupported action is
absent, not a button that posts into a void. The four email drafts are shown in full, with exact party,
sender, subject and body, before the grant action.

An item with a Runtime attach reference offers **Open live browser**. T5 owns the desktop and control
protocol; T1 only supplies the context, attach reference and focus-mode frame. The queue collapses so
the desktop gets the main canvas; the executive rail may remain available on demand.

## Scope

1. Replace the hard-coded `NeedsYouContext` workflow union with the common attention envelope and
   typed source/action/reference fields. Keep the mapping from generated wire rows to `DeskView` pure
   and explicit.
2. Project the four exact email approvals, the prepared merge review and the production gate from
   their source state. Do not mint a second attention lifecycle.
3. Remove the untyped owner-mail copy for requests represented by the queue. The blocked agent keeps
   its typed refusal.
4. Serve the real Aris desk through the authenticated owner gateway and replace the company page's
   fixture import with the read client.
5. Wire real grant/decline and organisational response callbacks to their existing operations.
6. Add meaningful live refresh and explicit source health/staleness. A dead source never renders as a
   generic green state.
7. Provide the browser focus-mode host and attach-state presentation for T5.
8. Keep `restless attention` as a headless rendering of the same projection for diagnosis and
   fallback. Reading ordinary `inbox` must not resolve it.

**Not in scope:** a new attention table, universal command endpoint, all eight hypothetical categories,
attention-learning, full resolved-history analytics, raw logs, an artifact database, or website-specific
browser controls.

## Acceptance

Run first against a `_test` company; no live party is contacted.

1. A first-contact attempt to an ungranted `_test` party produces one Authority attention item. The
   same source reference appears in `restless attention` and the live SPA, and ordinary inbox reading
   cannot remove it.
2. Refreshing the SPA and restarting the daemon reconstruct the same outstanding queue. No client-side
   cache or replay log is required for correctness.
3. Granting or declining through the SPA invokes the existing owner-authorised operation and the item
   leaves or changes state because its source changed, not because the UI deleted it.
4. A real OrgIntel review item with a Git compare URL and a real production blocker render through the
   common envelope without adding two new page components or command variants.
5. The four Aris drafts render exactly as persisted and remain unsent. Before T3's explicit live gate,
   there are zero new `customer-contact.email` receipts.
6. Killing OrgIntel or the Runtime produces distinct stale/unavailable presentations while available
   Authority controls remain usable.
7. An unauthenticated owner projection request, a cross-company reference and a forged principal are
   denied. No VNC/CDP credential or provider secret appears in the desk JSON, page source or logs.
8. The selected item can host T5's live desktop in focus mode and return to the same queue position.
9. `npm run check`, the generated binding drift guard and focused owner-gateway tests pass; the run
   report also includes one intentionally failing auth/reference probe so green is meaningful.

## Observed completion — 16 August 2026

All nine checks passed. In addition to auth, source-reference, restart and live Aris projection
probes, an isolated Chrome client signed into the built SPA, selected an actual Authority item and
clicked **Decline**. The item disappeared only after `approval_declined` landed in Authority; the
client did not delete local queue state.

The run exposed and removed a source-ownership violation: approvals, effects and receipts still lived
in OrgIntel's compactable `events` table. They now live in the daemon's narrow private Authority
schema. A versioned, idempotent import preserved Aris's 154 governance rows and purged legacy config
grants after transfer. With the `_test` OrgIntel actor/message/Work tables unavailable, the SPA
projection reported `orgintel: unavailable` and `authority: available`, retained its Authority item,
accepted a grant and recorded then replayed one generic self-reported effect. The same-key replay
returned the original receipt ID and created no second receipt.

Both temporary companies and their Authority/OrgIntel state were destroyed after the proof. No Aris
grant or effect was exercised.

## Risks

| Risk | Disposition | Why |
|---|---|---|
| Projection becomes a second source of truth | **Invariant** | IDs derive from source refs; actions write to source owner; refetch reconstructs state |
| Generic envelope becomes a universal command enum | **Guarded** | Actions are typed source references/callbacks; the envelope is read composition only |
| Owner gateway becomes a shell/filesystem API | **Invariant** | It serves owner projections, source-owned actions and attach transport only |
| Existing fixture UI dictates backend ontology | **Guarded** | Replace the per-kind context union; map proven source rows into the owner-facing contract |
| Authentication work expands into a user platform | **Accepted** | One owner credential and session in V0; multi-human identity is deferred |

## What this makes deletable

- `effect.rs:145`'s owner-notification-as-untyped-mail path for projected requests;
- `$lib/fixtures/cosmon` as the company inbox's runtime data source;
- four hard-coded workflow variants as the only vocabulary for owner attention;
- the instruction that asks a blocked agent to relay the exact owner command.

Mail returns to correspondence. Attention becomes the high-signal owner queue, and its browser target
is just one piece of evidence/action context rather than a new workflow system.

---
Sprint spec: [`../sprint-05.md`](../sprint-05.md)
