# Sprint 86 — Exec knows what happened, and says what is true

**Status:** landed (see the commits naming each item on main)
**Programme:** owner feedback; Exec quality
**Depends on:** Sprint 82 (Exec works on the source of truth), Sprint 85.

## Outcome

On 14 October:
- The owner raised the spend ceiling and renamed the company, and Exec carried on as if neither
  had happened.
- Exec said the old sites were archived while the Library still listed them. It then retired 69
  records without looking at the Library afterwards.
- An agent's model connection stopped working with "no longer granted to the company". Nobody could
  say who removed it or when, and Exec could not either.
- Exec may not see images the owner sends.

When this sprint is done, Exec sees what people changed, checks its claims against what the owner sees,
can read images, and every change to a model connection is recorded and visible.

## Changes

**S86-01. Exec sees what people changed.**
- Exec's context gets a "Changed by people since your last turn" block, with the 20 most recent
  owner and member changes. Each line says who changed what, from what, to what, and when.
- The source is the existing activity log (`company/activity`): settings, the charter, Brand & voice,
  limits, members, connections, team ownership and model choices.
- One line each, with no payloads. For example: "Yail raised the spend ceiling from $100 to $200
  (2h ago)" and "Yail renamed the company to Reno (2h ago)".

**S86-02. Exec checks its claims against the cockpit.**
- Doctrine: before telling a person that something is done, Exec reads it back the way the cockpit
  shows it. Examples: `restless library list` for the Library, `restless charter show` for the
  charter, `restless work show` for Work.
- If the read-back differs from the claim, Exec says what it sees instead of the claim.
- A "Cockpit map" block in Exec's context gives one line per surface: what lists there and which
  command reads it. The Library lists Work outputs, not files on disk, and moving a file does not
  remove it.

**S86-03. Every model connection change is recorded and visible.**
- Granting or revoking a connection's company access is recorded with who and when. So are removing
  a key, an expiry, and the provider refusing it.
- The record shows on Company › Intelligence and in Activity.
- When a turn fails with "no longer granted":
  - the failure names who revoked it and when, or says the record shows no revocation;
  - the failure is raised as one Inbox item for the owner with "Re-grant" and "Use another model";
  - agents see the same sentence.

**S86-04. Exec sees images.**
- Images attached to a message reach the model as image input wherever the connection's model
  accepts images.
- When it doesn't, Exec is told an image was attached and that it cannot see it, and says so once.
- Images sent earlier stay readable: they are kept with the message, not in a temporary upload store.

**S86-05. Nothing tells a person to run a command.**
- A guard on Exec's and agents' replies to people catches a shell command, a `/company/…` path
  offered as the thing to open, or a raw id.
- It asks the agent to say it in cockpit terms instead. Paths stay allowed inside code blocks the
  person asked for.

## Acceptance

- After the owner changes the spend ceiling, the next Exec context shows the line. A unit test covers
  the block, and the smoke step checks it.
- The rename shows in Exec's context the same way.
- Revoking a connection's grant records an entry. The next failed turn's text names the revoker, and
  the owner's Inbox has exactly one item for it. Unit and API tests cover this.
- A turn with an image on a model that accepts images sends image input (unit test on the request
  builder). On one that doesn't, Exec is told so.
- Seen live on the dev stack: change the ceiling, open the Exec context (`restless context show`),
  and see the line.

## As built

- **S86-02:** there is no new `restless library list`. The doctrine names the reads that already exist
  for each cockpit page. Retiring an output is `restless work retire-artifact`, then read back
  with `restless work graph`.
- **S86-03:** granting a second connection for a provider replaces the first: one binding per
  provider. That was the owner's "silently revoked" case. It is now recorded with the agents it
  strands. A grant also clears that provider's credential cooldowns.
- **S86-05:** besides the command guard, a provider error body streamed as reply text is dropped
  from the live reply.
