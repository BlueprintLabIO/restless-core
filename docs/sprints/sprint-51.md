# Sprint 51 — Authenticated realtime Docs

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 19
**Depends on:** Sprints 45–46 and 50

## Outcome

Two authorized humans concurrently edit one native Doc, disconnect, restart the collaboration process
and reconnect to the exact persisted content without loss, duplication or cross-company disclosure.

## Scope

- Add Yjs to the open editor only and package a replaceable Hocuspocus sidecar inside the Core release.
- Make persisted Yjs state the canonical live body; named ProseMirror JSON remains a projection of a
  checkpoint, never a second concurrent writer.
- Issue short-lived Core collaboration tokens bound to company, document, human/user principal,
  durable Actor, read/write access, expiry and session/nonce.
- Authenticate before loading content and reject writes from read-only clients. Observe revocation and
  terminate/expire active sessions within the declared bound.
- Give the sidecar a narrow persistence adapter/credential for document content only. It cannot read
  membership, OrgIntel, Authority or another company's rows.
- Persist merged state with bounded debounce, retry failed stores visibly and expose durable-save
  status. Never report saved while the durable write is unknown.
- Keep document-body WebSocket traffic separate from Core SSE metadata/comments/review events.
- Add two-client convergence, restart, corrupt-state recovery and tenant-negative fixtures.

## Acceptance

1. Two humans make concurrent edits, disconnect and converge to one exact durable document after
   reconnect.
2. Sidecar restart reloads the last durable Yjs state rather than an empty/reconstructed substitute.
3. Wrong company/document/audience/signature/expiry/access tokens fail before content load; read-only
   clients cannot submit updates.
4. Membership removal closes or expires the Doc session inside the stated bound while preserving
   historical attribution.
5. A failed persistence write is retried and shown as not durably saved; no success is fabricated.
6. Recovery chooses durable Yjs state, then valid named version, then backup and records a new visible
   recovery version.
7. Normal Work/Chat/Attention mutations remain HTTP/outbox/SSE and no CRDT dependency leaks into them.

## Ticket outline

- [ ] C51-T0 — released sidecar boundary and protocol contract
- [ ] C51-T1 — collaboration token issuance/verification
- [ ] C51-T2 — Yjs persistence adapter and durable-save status
- [ ] C51-T3 — Svelte collaboration provider and awareness
- [ ] C51-T4 — revocation, restart and corruption recovery
- [ ] C51-T5 — two-user convergence and tenant-isolation proof

## Deletion and exclusions

Delete any unauthenticated socket, in-memory-only canonical body or duplicate direct database path.
Do not add Redis, horizontal Hocuspocus clustering, CRDT Chat/Work, indefinite low-level update history,
offline-first guaranteed editing or a shared Fleet document service.
