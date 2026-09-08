# Core company collaboration programme

**Status:** Draft for founder alignment; implementation not started
**Date:** 8 September 2026
**First safe Core sprint:** 45
**Cloud companion:** `restless-cloud/docs/sprints/cloud-company-collaboration-programme.md`

## Outcome

One authoritative Restless company can be used by a small group of humans and durable agents from
phone or desktop. They can enter through hosted or self-hosted identity, collaborate in Rooms and
native Docs, route scarce judgement to the right Actor, resume durable agent work without replaying
whole histories, and recover after disconnects or Runtime suspension. The same Core contract powers
local/self-hosted and Restless Cloud deployments.

This programme reconciles seven owner-supplied proposals:

- internal Cloud dogfood and personal usage;
- mobile responsiveness and state synchronisation;
- Core multiplayer and judgement routing;
- Actor context management and continuity;
- Core Chat and Rooms;
- native Docs; and
- the architecture tension register.

The proposals are direction, not a licence to create parallel sources of truth. `ARCHITECTURE.md`
remains the parent architecture; Sprint 45 must update its now-stale statement that multiplayer and
managed deployment remain deferred.

## Adopted seam decisions

1. **Core owns company semantics.** Actors, Rooms, Threads, Messages, Mentions, Docs metadata,
   judgement routing, Attention, Work relationships, Decisions and context selection are Core
   concepts. Cloud owns hosted identity, membership, entry, delivery and fleet operations.
2. **Identity is adapter-neutral.** Core verifies a provider-neutral signed
   `CompanyAccessContext`. Cloud/Better Auth and a local self-hosted adapter can issue the same
   semantic contract. Core never reads Better Auth tables.
3. **Browser handoff and company access are distinct.** Fleet issues a single-use entry assertion and
   redirects to the owner account plane. The plane establishes a short-lived company session. Fleet
   is absent from cockpit, API, SSE and document traffic after that redirect.
4. **Actor attribution is server-derived.** Core maps the verified human subject to one durable
   company Actor. Clients cannot select an acting Actor. Core/OrgIntel, not Runtime Bridge, mints
   session credentials for agent Actors.
5. **Membership, responsibility and Authority stay separate.** Membership owner/admin/member controls
   entry; organisational roles control work and judgement; Authority owner/capabilities control
   consequences. A transfer changes either boundary only through its own explicit operation.
6. **Bootstrap owns `company_id`.** Hosted Fleet bootstrap allocates it; local Core bootstrap allocates
   it; Authority records and protects the result. Retries reuse the immutable identity.
7. **One primary session per durable agent Actor.** Additional work is queued or represented by
   attributable delegated Attempts/temporary workers, not sovereign copies of one Exec or lead.
8. **Existing Work is not duplicated.** The current Core Work record remains the durable
   responsibility/commitment boundary. The owner-facing **Work view** composes Goal, Work, Attempt,
   Artifact and Attention; it gets no second mutable lifecycle or status.
9. **One ordinary state path.** Domain-specific HTTP commands/queries, idempotency, optimistic
   versions, canonical responses and the existing operational-event store extended as a transactional
   outbox/cursor. This is a network envelope, not a universal internal command algebra.
10. **Chat is append-only Core state.** It uses restricted rich-text JSON, revisions and tombstones;
    it does not use CRDTs or become a Work/Decision source by implication.
11. **Native Docs are a Core collaboration subsystem.** Rust owns identity, access, metadata,
    comments, reviews, versions, proposals, export and organisational links. A narrowly credentialed
    Hocuspocus sidecar owns active Yjs synchronisation and durable content load/store only. Cloud
    deploys and operates the released subsystem; it does not own document meaning.
12. **Context is a projection, not a database or authority grant.** OrgIntel supplies a compact shared
    spine and retrieval links; each Actor owns its working set and checkpoints. Explicit references,
    backlinks and full-text search precede embeddings or a universal memory service.
13. **Core chooses notification meaning; Cloud delivers.** Core creates recipient-relative Attention
    and minimal delivery intents. Cloud owns push/email transport and device endpoints, never the
    underlying judgement state.
14. **The Core Svelte cockpit is the product client.** Mobile/PWA work extends it. Cloud does not build
    a second cockpit; the public Astro site remains unrelated to authenticated company state.

## Sequence

| Order | Core sprint | User-visible proof | Paired Cloud work |
| --- | --- | --- | --- |
| 1 | [45 — trusted human entry](sprint-45.md) | Hosted and self-hosted humans reach the same attributed Core API without gaining Authority | Cloud 16 |
| 2 | [46 — shared-state spine](sprint-46.md) | Two clients update one company and recover from an event gap without global polling | Cloud 17 |
| 3 | [47 — human Rooms](sprint-47.md) | Two humans use company, project and direct Rooms with Threads and reconnect-safe sends | Cloud 18 |
| 4 | [48 — judgement-aware collaboration](sprint-48.md) | `@exec`, focused Attention, promotion, handoff and conflict routing resume exact Work | Cloud 18 |
| 5 | [49 — Actor continuity](sprint-49.md) | A durable Actor restarts on a fresh session with focused context and no material loss | Cloud 18 |
| 6 | [50 — native Docs foundation](sprint-50.md) | A self-hosted user authors, versions, restores and exports a Core Doc | Cloud 19 |
| 7 | [51 — realtime Docs](sprint-51.md) | Two humans co-edit and converge through an authenticated, durable Yjs path | Cloud 19 |
| 8 | [52 — Docs as company work](sprint-52.md) | Comments, mentions, review and agent proposals connect a Doc to real Work and Decisions | Cloud 19 |
| 9 | [53 — mobile judgement console](sprint-53.md) | Repeat phone use is cached, responsive, resumable and deep-linkable | Cloud 20 |
| 10 | [54 — released collaboration candidate](sprint-54.md) | One immutable Core release passes the full self-hosted and hosted collaboration contract | Cloud 21 |

Sprints are dependency ordered, not a promise to serialize all work. Contract fixtures and client
types may start early; a later sprint cannot claim completion against mocks or a locally patched
server.

## Cross-repository contract ownership

| Contract | Canonical producer | Cloud responsibility |
| --- | --- | --- |
| `CompanyAccessContext` schema and verifier semantics | Core | Better Auth adapter, issuance, rotation and revocation signals |
| Actor mapping result and acting principal | Core | Supply opaque user/membership claims; retain no editable Actor mirror |
| Domain command/query/event schemas | Core | Consume generated release contracts; do not hand-maintain DTO forks |
| Entry assertion | Cloud/Fleet | Core account-plane verifier consumes it once |
| Rooms, Attention, context and Docs semantics | Core | Route, host, notify and operate |
| PWA/cockpit application | Core release | Deliver at the owner plane and preserve direct data path |
| External notification delivery | Cloud | Consume minimal Core delivery intent and return delivery status |
| Release identity and compatibility corpus | Core | Pin exact image/manifest/digest and promote unchanged |

## Architecture and migration posture

- Sprint 45 updates architecture and ADRs before incompatible migrations land. It must explicitly
  supersede the old “multiplayer deferred” posture using the owner's observed mobile/team need.
- Evolve current `owner | exec | staff | system` actor kinds through an explicit compatibility
  migration toward Actor class `human | agent | service` plus separate organisational roles. Do not
  reinterpret old rows silently or make membership role an Actor kind.
- Replace the network HMAC assertion with the released asymmetric/JWKS contract. Preserve local
  loopback entry through the local identity adapter; do not turn local use into a Cloud dependency.
- Thread the verified request principal through every handler before enabling invited members. Remove
  literal owner attribution only after equivalent positive and negative tests exist.
- Extend the existing operational events table/publisher into the outbox/cursor path. Do not add a
  parallel event bus, event-sourced company model or indefinite replay promise.
- Introduce native Docs inside the Core release and lifecycle manifest. The sidecar may be a separate
  process because its protocol requires it, but it is not a fourth architectural plane or Cloud-only
  service.

## Programme acceptance

The programme closes only when one exact release proves all of the following:

1. Hosted and self-hosted identity adapters produce equivalent Core multiplayer semantics.
2. An owner, admin, member, Exec, lead and worker are durably distinct and correctly attributed.
3. Removing membership blocks HTTP, SSE and document channels within the declared bound while history
   remains attributable.
4. Human chat continues while Runtime is asleep; an agent mention wakes one durable Actor and returns
   to the same Thread.
5. Conflicting human direction reaches the accountable scope owner; recent-message order is never the
   decision rule.
6. A fresh Actor session resumes from identity, current Work, a source-linked checkpoint and focused
   Room/Doc context without full-history injection.
7. Two humans co-edit one Doc, restart/reconnect without loss, and review one attributed agent
   proposal without silent overwrite.
8. A repeat phone visit renders safe cached Attention quickly, preserves drafts/route/scroll through
   backgrounding and reconciles through cursor-based events.
9. The writing client gets read-your-own-write from the canonical command response; another client
   sees the committed change without broad polling or a full-page reload.
10. Runtime suspension leaves Chat, Docs, Work, Attention, People and Authority readable.
11. A company backup/restore includes collaboration state and does not repeat an external effect.
12. The immutable Core candidate is consumed unchanged by a Sydney-near Cloud dogfood cell for real
    multi-week use.

## Stop and purge rules

Stop promotion for cross-company access, client-selected Actor attribution, membership-as-Authority,
assertion replay, unbounded revoked sessions, document loss/corruption, silent last-write-wins,
hidden dual Doc truth, or a Fleet hop in the company data path.

Before closing each sprint, purge duplicated DTOs, blanket polling, owner-only shortcuts, full-company
prompt assembly, parallel event machinery, dead collaboration prototypes and any Cloud cockpit or Doc
semantics that compete with Core. Preserve unrelated Sprint 40–44 work and its owner changes.

## Source documents

The owner-supplied files in `/Users/yao/Downloads` remain proposal inputs until their adopted decisions
are reflected in architecture/ADR/spec canon. This programme records the reconciliation; it does not
make every illustrative field name or topology diagram an invariant.
