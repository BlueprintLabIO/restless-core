# Sprint 73 — Someone is typing, and where we stand

**Status:** landed
**Programme:** owner cockpit, collaboration
**Depends on:** Sprint 72.

## Outcome

In a group room or a direct conversation, the others see "Alice is typing…" as she types. The
line goes once she stops or sends. The cockpit's maturity against Notion is written down, area by
area, in `docs/maturity-vs-notion.md`, with what remains in order.

## Observed friction

| Friction | Evidence |
|---|---|
| Chat has no sign that someone is answering | rooms are Slack-like; nothing said a colleague was writing |
| The multiplayer smoke stopped running on this host | a glibc update made `plane.localhost` resolve to `::1` for every `*.localhost`. The script accepted `::1` and skipped its private hosts file. The services listen on IPv4, so the account service never answered. |
| Typing right after sending said nothing | a 3-second throttle survived the send; the first words of the next message fell inside it and were never announced (found by the smoke) |

## Design

Typing is ephemeral process memory, like presence.
- `POST /companies/{c}/rooms/{room}/typing` checks the person is in the room (`room_open_to`,
  the same rule event replay uses) and publishes to an in-process broadcast.
- Each open room stream subscribes and, between replays, forwards another person's signal as a
  `typing` event. The signal never touches the database or the company change hints, so typing
  makes nobody refetch anything.
- The client shows a signal for 6 s and repeats its own at most every 3 s while typing. The throttle
  resets when the composer empties, and a message from the typist ends their line at once.

## Tickets

- [x] **S73-T1 Typing in rooms:**
  - server hub and route
  - room stream forwarding
  - client stream handling
  - one quiet live line above the composer, its height reserved so nothing jumps
- [x] **S73-T2 The smoke on hosts with modern glibc.** The script requires the first address for
  an ordinary lookup to be `127.0.0.1`, and otherwise runs in its private namespace as before.
- [x] **S73-T3 Maturity against Notion:** `docs/maturity-vs-notion.md`.
- [x] **S73-T4 Verify.** The multiplayer smoke adds a step: Alice types in a group room, the owner
  sees "Alice Member is typing…" within 5 s, Alice is not told about herself, and the line goes
  once she stops. All 21 steps pass. The journey reports the typing POSTs it saw when the step
  fails.

## Results
