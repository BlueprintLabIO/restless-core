# Instant decisions with durable Undo

Observed failure: `InboxDetail.markDone` holds the current card for the whole Undo timer; the
`inbox-actions.afterUndo` timer and pending set live only in one page process. Moving `onresolved`
before that promise would make the screen faster while losing the decision on a tab close. It
would also leave approvals without any recoverable Undo boundary. C1 is not complete.

The visible result must be immediate; the consequential decision must wait. Record the exact
pending decision durably, with its actor, company, target, displayed revision and Undo deadline.
Use the existing Authority/OrgIntel decision paths; do not build a generic HTTP replay service or
another workflow engine. Durable storage must not carry secret values or stale session cookies.

Required end-to-end cases:

- Mark done, skip, approve and accept remove the card immediately and show the next prefetched
  item. A failed initial save restores the card once, with the draft/decision intact.
- Undo before the deadline cancels the decision. No Work resume, permission grant, acceptance,
  provider send or other downstream effect occurs before that deadline.
- Navigate, reload, close the tab and restart the daemon during the window: the original deadline
  and decision survive. The next session reads the result and offers Undo while still possible.
- Another person, new revision, revoked access or removed membership is rechecked before commit.
  A stale decision never accepts new work or re-grants revoked authority.
- A commit failure restores the item with its reason; an unknown outcome is reconciled before
  replay. Requests and commits have stable identifiers so a repeated request is not another action.
- Include owner, delegated admin and assigned member cases, and two clients deciding concurrently.

Delete the page-owned timeout/pending set after the durable path replaces every affected caller.
Keep existing direct authority checks as the authority boundary for every commit.
