# ADR 0010 — Native Docs are a Core collaboration subsystem

**Status:** Accepted

**Date:** 8 September 2026

**Parent:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §2.4 and §4.4

## Context

Realtime co-authoring needs specialised CRDT and WebSocket mechanics, but making those mechanics a
new architectural plane or Cloud-owned product would duplicate company identity, access, review and
recovery truth. Treating Markdown and Yjs as simultaneous live sources would create an unrecoverable
split brain.

## Decision

Native Docs are a narrow Core company notebook. Rust Core owns document identity, access, metadata,
comments, mentions, reviews, named versions, agent proposals, exports, search projections and
organisational links. A replaceable, narrowly credentialed Hocuspocus sidecar owns active Yjs
document-body synchronisation and durable load/store only.

Canonical live content is persisted Yjs state. Tiptap/ProseMirror JSON is the structured projection
used for named versions and reviewable agent revisions. Markdown is an explicit export/checkpoint;
editing an export never silently changes the live Doc. Agents propose attributed revisions against a
named base by default and join live editing only during explicit collaboration.

CRDT is confined to document bodies. Rooms, Work, Authority, membership, comments, review state and
general company records remain normal versioned Core state. Ordinary productive files and Artifacts
remain Runtime-owned.

## Consequences

- The sidecar ships, backs up, restores, relocates, deletes and reports compatibility as part of one
  company-cell release; it is not a fourth plane.
- Core issues short-lived document-scoped collaboration tokens after authoritative access checks.
- A durable-save indication requires persistence acknowledgement; sidecar failure must not report a
  successful save.
- Runtime export and re-import are explicit, attributed operations with version/hash provenance.
- Fleet may operate the sidecar but never stores document bodies or decides document meaning.
