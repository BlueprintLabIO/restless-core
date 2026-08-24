# S12 mid-work direct-message probe v9 — frozen plan

**Company:** `sprint12_midwork_v9_test` (disposable `_test` company)
**Purpose:** prove both truthful live observation and the immutable late-input fallback with the
existing Work/message/inbox substrate. This is not a real-company outcome.

## Changed mechanism

The v8 counterexample showed two facts that had to be preserved separately: a Work-linked response
needs its exact Work id in the lead's coordination context, and a live Staff process that reads its
own inbox needs that observation recorded against its already-running Attempt. The changed code:

1. renders a Work-linked inbound message as its literal Work id plus one exact direct-reply command;
2. treats a company actor's self-inbox read as delivery, marks that mail read, and links matching
   Work feedback to its live Attempt; and
3. retains the v8 terminal guard: a linked response that was not actually read supersedes the stale
   Attempt and becomes initial input to one sequential successor.

No message kind, queue, handoff, timer, second cognitive process, or mutable initial-input snapshot
is added.

## Fixed setup and intervention

Use the v8 actor/team/no-effect/model fixture. The Staff Work permits two Attempts only for the
unread-late-feedback fallback. The Staff may inspect its own addressed inbox exactly once at its final
decision point via `restless inbox --as world-builder`; it must not poll, sleep, or use a timer. The
one injected fact is unchanged:

> The landing seam’s frozen interface changed: `terrain_collider_id` is no longer available; use
> `landing_zone_id` instead. Decide the integration contract before I continue. Send only the smallest
> direct decision back to me; do not involve Exec or the owner.

Inject only after Attempt 1 is observed running. The lead and Staff turns remain real model turns.

## Passing observations

1. `product-direction` receives one Work-linked message context naming the exact Work id and sends
   one direct Work-linked `landing_zone_id` decision to `world-builder`, with no owner/Exec message
   or wake, unlinked status, command fragment, file/Git/artifact, receipt or effect.
2. If Staff reads that reply while Attempt 1 is live, the response is marked read and bound to Attempt
   1's feedback evidence; Attempt 1 can complete with that observed fact and no Attempt 2 starts.
3. If its one read happens before the reply, Attempt 1 is superseded—not falsely completed—and one
   successor starts after it exits with the exact linked response as initial feedback.
4. In either timing, the final terminal summary applies `landing_zone_id`, no stale blocker reaches
   the lead, and no concurrent/duplicate actor process exists.

Any unlinked reply, extra internal status/command mail, missing feedback evidence, owner/Exec
interaction, source change, third Attempt, or timer-driven coordination is counterevidence. Destroy
only this named `_test` company after capture.
