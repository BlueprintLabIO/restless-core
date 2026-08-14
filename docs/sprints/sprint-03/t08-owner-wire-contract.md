# S03-T8 · The owner wire contract

**Layer:** Cross-layer (the owner-facing surface of all three)
**Depends on:** nothing
**Blocks:** T5 (its approval needs an attention item with a name and a shape)

---

## Why this ticket exists

The attention queue is about to become the first owner-facing noun on the wire.
T5 needs one to raise an approval against; sprint-02 T9 is carried and unbuilt;
and the notification path has nowhere to send anything. Whatever shape it takes,
every later client inherits it.

The cost asymmetry is the whole argument. **The transport is disposable; the
vocabulary is permanent.** Swapping a Unix socket for HTTP later is mechanical.
Renaming a noun that three clients have hardcoded is not. So this ticket spends a
little now on the things that ossify, and explicitly defers the things that do not.

It is also the cheapest this will ever be *except* in one respect, which the
planning discussion got wrong: it is not true that there are zero clients.

## There is already a client, and it is ahead of us

[`web/`](../../../web) is 9,841 lines of SPA, lifted in `cf8a028`, rendering the
target posture from fixtures. Its read model is
[`DeskView`](../../../web/src/lib/model/view.ts) — 20 fields, including
`needsYou: NeedsYouItem[]`. So the wire vocabulary is not being chosen from
nothing; it is being **reconciled between the contract and a working UI**, and
the UI is more specific:

```ts
type NeedsYouKind = 'decision' | 'email-approval' | 'promotion-approval' | 'escalation';
type NeedsYouRef  = { approvalRequestId: string; version: number } | ...
type NeedsYouContext = { kind: 'email-approval'; draft: EmailDraftView | null } | ...
```

`EmailDraftView` is the exact draft awaiting a signature — precisely what T5 must
render. And `NeedsYouRef` carries a **`version`**, which answers the
double-clicked-Approve problem by rejecting the stale second click rather than
deduplicating it.

In two places the SPA already encodes discipline the daemon does not:
`ConnectionRow` keeps `ok` and `failed` as separate booleans so "never checked" is
representable, with `status` *"verbatim from the live check, never paraphrased"*;
`HqView.runway` pairs `months: number | null` with `reason` for why it could not
be estimated. That is `cross-layer-contract` §4.7 and `owner-cockpit` §2.6 already
built. The daemon should adopt it, not relitigate it.

`DeskView` also carries concepts the rebuild deliberately discarded — `library`,
`records` are the content-addressed asset-custody surface CLAUDE.md says not to
recreate. Those get deleted from the type, not implemented.

## Scope

1. **Reconcile `DeskView` against the contract, field by field.** For each of the
   20 fields: *live source* / *derivable today* / *delete as legacy*. The output is
   a written list, and it settles the noun set. Keep the contract's opaque
   identifiers (`attention_item_id`, §2.1); keep the SPA's richer kinds and
   contexts where they carry real distinctions.
2. **Typed error kinds on the wire.** `{ok, data, error: {kind, message}}`.
   `BlockKind` already *is* this enum — `credential`, `quota`, `budget`, `model`,
   `no-op`, `transport` — and is being flattened into a string by
   `Blocked::message()` at the boundary. Stop flattening. The UI must distinguish
   "authority denied" from "daemon unreachable" from "already resolved"; it cannot
   switch on prose.
3. **Idempotency on §4.5's five classes, and no others.** `operation_id` on
   company creation, runtime create/restore/destroy, consequential external
   effects, resource provisioning with external cost, and **owner approval
   decisions**. Not on `tell`, not on every mutation — §4.5 is titled
   *"Idempotency is selective"* and says in terms: *"Do not require idempotency
   keys for every message, file edit, or internal planning update."*
4. **Writes return a receipt, not resulting state.** `{accepted: id, status}` over
   §4.7's category set. §4.4 is *asynchronous by default*; approvals, effects and
   runtime ops are not synchronous, and a write that returns state teaches every
   call site otherwise. This is the hardest item to retrofit because it is baked
   into control flow, not into a schema.
5. **Events are invalidation hints, not payloads.** `watch` already has the
   skeleton — `events_after(watermark)`, resumable, 2s poll ([main.rs](../../../crates/restlessd/src/main.rs#L277)).
   The rule to fix now: the UI renders from projections and the stream only says
   *something changed, refetch*. Never fold the event stream into UI state — that
   is what keeps OrgIntel's *"may be compacted, repaired, or regenerated"*
   property (§3.2) from silently becoming a lie. One rail, filtered client-side;
   no new push channel per feature.
6. **Address concepts by owner, not by daemon internals.** `attention_item`,
   `effect`, `receipt` — not "socket command X". Sprint-02 T6 will extract the
   ingress/effects/receipts unit into its own process; if the wire names concepts,
   that extraction is a deployment change invisible to every client. This costs
   nothing today — it is naming discipline, not an abstraction layer.

## Not in scope, deliberately

SSE vs WebSocket (irrelevant until the SPA is wired; §14.4 says either is
sufficient). REST-vs-RPC shape and the web framework — replaceable; vocabulary is
not. Auth beyond one attributed principal (§14.5 forbids more for V0). Protocol
version negotiation — two founders, one deployment. Token streaming for the exec
chat — §14.4 forbids it; replies arrive as message events like everything else.

The HTTP listener itself is also out of scope. The owner API stays local-only, and
[T2](../sprint-03.md)'s ingress remains the only public port — when a browser
needs in, it comes through a thin localhost bridge, not by making the daemon a
network server. AC8 records this alongside the ingress posture.

## Acceptance

1. The `DeskView` reconciliation exists as a written list, all 20 fields
   dispositioned, and the legacy fields are deleted from the type.
2. An attention item crosses the wire under one name with a stable
   `attention_item_id`, and T5's approval is raised against it.
3. A provider-credential failure and a daemon-unreachable failure arrive as
   different `error.kind` values, verified by two headless calls.
4. The same approval submitted twice with one `operation_id` resolves once —
   observed, not asserted — and a `tell` sent twice without one delivers twice.
5. A write returns `{accepted, status}` and the caller observes the outcome via a
   subsequent read, not from the write's response.
6. A client that misses events while disconnected reconstructs correct state by
   refetching the projection, with no event replay.

## What this makes deletable

`Blocked::message()`'s string flattening at the boundary, and the ad-hoc error
prose in [main.rs](../../../crates/restlessd/src/main.rs)'s dispatch arms.
The `library` / `records` / asset-custody fields of `DeskView` and the fixture
data behind them. If the reconciliation finds `add_goal` / `add_decision` /
`add_artifact_ref` still have no reader after the view is settled — flagged during
the sprint-02 purge as storage for concepts with no write path — they become
deletable too, or the view is what finally gives them a caller.
