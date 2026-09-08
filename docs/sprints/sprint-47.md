# Sprint 47 — Human Rooms and Threads

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 18
**Depends on:** Sprints 45–46

## Outcome

Two humans use one company Room, one project Room and one direct/group Room from the Core cockpit. They
send idempotent Messages, reply in Threads, see unread state and reconnect without duplicate or lost
conversation while the Company Runtime is stopped.

## Scope

- Add company-scoped Rooms, participants, append-only Messages, Thread state, read cursors, references,
  message revisions and tombstones to the existing OrgIntel database.
- Use one restricted, validated rich-text JSON subset with derived plain text; never store arbitrary
  HTML or use Yjs for Chat.
- Apply simple Room visibility: company, project scope, or explicit participants. Avoid per-Message ACLs.
- Create the default company Room idempotently and support project, group and direct Room creation.
- Add cursor-paged Room/Thread queries, send/edit/delete/read commands and outbox events.
- Build the Svelte Room list, compact message/thread views, draft-safe composer, unread projection,
  reconnect/degraded states and mobile-ready layouts.
- Store typed references to existing Work, Decision, Attention, Doc and Artifact IDs without copying
  those objects or inventing attachment custody.
- Start Postgres full-text/trigram search over authorized message projections only after the core
  conversation journey is working.

## Acceptance

1. Two human Actors see a new Message without refresh; a timeout retry with the same command ID creates
   exactly one Message.
2. A guessed Room ID is denied across companies and to non-participants in a private Room.
3. A member cannot add an unauthorized participant; owner/admin inspection follows the documented
   visibility policy rather than implicit universal access.
4. Message edit appends an attributed revision and delete leaves a tombstone without orphaning replies
   or references.
5. Cursor paging and read state remain correct under concurrent sends and reconnect.
6. Chat stays usable while Runtime is suspended and clearly retains unsent local drafts during Core
   unavailability.
7. Search returns authorized messages and never reveals inaccessible Room existence or content.

## Ticket outline

- [ ] C47-T0 — Room/message schema and access policy
- [ ] C47-T1 — commands, queries, idempotency and paging
- [ ] C47-T2 — outbox events, unread and reconnect
- [ ] C47-T3 — desktop/mobile Room and Thread experience
- [ ] C47-T4 — revisions, tombstones, references and search
- [ ] C47-T5 — two-human Runtime-asleep dogfood

## Deletion and exclusions

Delete the nullable-recipient-means-owner convention once migrations and API consumers are complete.
Do not build Slack-scale channel administration, presence as correctness, typing indicators, social
feeds, voice/video, guest links, CRDT Chat or automatic conversion of conversation into Work.
