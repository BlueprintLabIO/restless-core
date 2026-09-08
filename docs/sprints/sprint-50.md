# Sprint 50 — Native Docs foundation

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 19
**Depends on:** Sprints 45–46; Room/Work reference contracts from Sprint 47 may be consumed later

## Outcome

A self-hosted Core user creates and edits a focused company Doc, creates and restores meaningful named
versions, and exports an attributed Markdown/JSON checkpoint to the Company Runtime without creating a
second live write path.

## Scope

- Establish Native Docs as a Core collaboration subsystem, not a fourth plane and not a Cloud-owned
  office service.
- Add Rust-owned document identity, metadata, kind, status, visibility, participants, owner Actor,
  organisational links and optimistic versioning.
- Add the restricted Tiptap/ProseMirror schema, stable top-level block IDs and deterministic JSON/plain
  text/Markdown projections. Reject arbitrary HTML.
- Implement named versions carrying durable content snapshot, JSON, plain text, hash, Actor, reason and
  time. Restore creates a new version and preserves later history.
- Support starter templates for briefs, plans, decision notes, reports, reviews, handbooks and operating
  notes without creating a template workflow engine.
- Export an exact named version to `/company/docs/<slug>.md` and JSON with source Doc/version/hash and
  attribution. Runtime changes return only through an explicit import-as-proposal path.
- Build a readable Svelte authoring/history/compare experience, lazy-loaded away from the initial app
  shell and responsive enough for phone review.
- Include document metadata and named versions in company backup/export/release contracts.

## Acceptance

1. A human creates, edits, reads and archives a Doc under company/document access checks.
2. A named version preserves exact content state, JSON, plain text, hash, author and reason; restore
   creates a new version without erasing history.
3. Unsupported/raw HTML is rejected and stable block IDs survive unrelated edits.
4. A hidden/participant Doc cannot be opened through a guessed ID or search projection.
5. Markdown/JSON export names the source Doc/version/hash; editing the exported file cannot silently
   mutate live content.
6. A backup and restore recovers metadata, versions and export provenance in a disposable company.
7. Opening normal Attention/Chat does not load the editor bundle.

## Ticket outline

- [ ] C50-T0 — subsystem ownership ADR and document schema
- [ ] C50-T1 — restricted editor schema and stable block IDs
- [ ] C50-T2 — metadata, access and organisational links
- [ ] C50-T3 — named versions, comparison and restore
- [ ] C50-T4 — explicit Runtime export/import boundary
- [ ] C50-T5 — self-hosted authoring and recovery proof

## Deletion and exclusions

Delete any competing “Markdown is live truth” path and duplicate document metadata outside Core. Do
not build an office suite, page layout, slides, spreadsheets, plugins, range comments, public links,
Office fidelity, arbitrary embeds or live agent editing in this sprint.
