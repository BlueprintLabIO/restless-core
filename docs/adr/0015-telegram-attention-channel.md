# ADR 0015 — Telegram as the owner's first Attention channel

**Status:** Accepted; the run against a real BotFather bot remains open

**Date:** 5 October 2026

## Decision

The owner can receive Attention items in Telegram and answer approvals there. They bring their own
bot from BotFather and store only a credential reference (`infisical:` or `env:`). One company has at
most one link in `restless_authority.telegram_links`, which lives host-side in the account plane:
token reference, bot username, the cockpit origin for deep links, the Authority owner who started
pairing, a one-time code, and the paired chat id. Nothing about the channel enters a company
container.

- **Pairing.** Only the company's Authority owner can start it. Starting probes the bot live
  (`getMe`) and issues a ten-character code for 30 minutes. The owner sends that code to the bot
  from a private chat, through a `t.me/<bot>?start=<code>` link or by typing it. Long polling with
  `getUpdates` means a self-hosted Core needs no public webhook. Pairing again replaces the link and
  its deliveries; unpairing deletes both.
- **Notify.** Attention stays the only definition of owner-worthy work. Each delivery pass sends at
  most ten items, each with its title, what happened and a deep link. An item is identified by its
  id, source kind and source reference, so a newer Authority record or handoff is a new message and
  the same one never is. Items still being briefed wait.
- **Approve.** A first-contact party, a reserved tool call or an email mandate proposal gets Approve
  and Decline buttons. A button carries only a delivery id. The delivery row binds it to the exact
  question: the record id and party, the call key and command digest, or the proposal id. A press is
  accepted only from the paired private chat, and only while the person who paired still holds
  Authority. It then runs `approval::grant`/`decline`, `effect::decide_tool_call` or
  `decide_email_mandate_proposal` as that owner, exactly as the cockpit does.

## Risks and dispositions

- **A crash between claiming and sending loses the Telegram copy.** Accepted. A delivery is claimed
  (`sending`) before `sendMessage`, and Telegram has no idempotency key. A refusal or connect failure
  releases the claim for the next pass. Any other unknown outcome is never resent. The item stays in
  the cockpit, which is the record.
- **A replayed or concurrent press.** Guarded. Each delivery is claimed once (`decided_at IS NULL`).
  The question must still be open, so an item already answered in the cockpit or replaced by a new
  command digest answers "Already decided".
- **A stolen or forwarded button.** Guarded. The callback must come from the paired chat and its
  sender must be that chat (private only).
- **Authority moves to another person.** Guarded. A press from the old owner's chat is refused until
  the new owner pairs.
- **One bot shared by several companies.** Accepted. One poller per distinct token reference routes
  codes and presses by delivery row.

## Consequences

- The Cloud email projection (`owner_notifications.rs`) and this channel both read `attention::project`.
  The email projection is hosted-only (it needs a membership identity), and the two channels do not
  share delivery state.
- A second channel would reuse the binding and claim rules. It would not reuse the Telegram transport.
