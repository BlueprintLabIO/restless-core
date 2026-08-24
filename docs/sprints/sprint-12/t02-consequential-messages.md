# S12-T2 — Deliver consequential team messages

**Layer:** OrgIntel + Runtime bridge.

**Observed friction served:** questions, blockers, interface changes and recovery evidence may wake the
wrong altitude, wait for a scan or route through Exec as unlinked narration.

## Outcome

An addressed Work-relevant message wakes the actor able to decide. Lead and Staff communicate
directly; Work feedback reaches the next Attempt; Exec receives only portfolio or cross-team
escalations.

## Acceptance

- Staff can send a Work-linked question, blocker, changed-interface fact, artifact/result or review
  request directly to its accountable lead.
- A message to the current Work owner remains deterministic Attempt input and cannot race a separate
  conversation process.
- A recovery observation may be linked to the affected Work while addressing its accountable lead.
- Delivery tolerates a duplicate notification without launching a second cognitive process.
- Narration does not create a required message kind, cadence or wake.
- A `_test` scenario changes one interface mid-Attempt and observes one useful lead wake, no Exec wake
  and no timer.

## Non-goals

- automatic semantic scoring of every message;
- meetings, presence, broadcast feeds or a common room;
- exactly-once internal delivery.

## Deletion target

Exec relay, fixed rendezvous wakes and duplicated message paths that do not change Work input or actor
judgement.
