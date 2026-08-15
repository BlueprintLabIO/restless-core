# S04-T10 · A principal on the wire

**Layer:** Authority (the boundary), expressed at the daemon's listeners
**Serves:** The approval boundary S03-T5 built, which is currently reachable from inside the container
**Depends on:** nothing
**Makes deletable:** the accepted-risk comment at `main.rs:215`, whose stated expiry has passed

---

## The hole

`serve()` (`main.rs:278`) is generic over the stream type. The unix listener (owner, on the host) and
the TCP listener on `:7791` (agents, inside the company container) both hand into it, and both land in
the same `dispatch()`. There is no principal, no origin check, and no owner-only command set.

So `restless approve --party <addr>` — the human authority act that lets a company make first contact
with a real stranger through a real provider — is callable from inside the container by any process
the Exec spawns. So are `up`, `down`, `wake`, and `clear-poison`, which clears the spend fuse.

Two things soften it and neither is a control: the Exec's prompt advertises only `spawn` and `effect`
(`context.rs:137,158`), so it is not *told*; and `restless --help` inside the container lists all
seventeen commands, so it is not *prevented*.

The daemon already says this out loud at `main.rs:215`:

> company identity on a request is trusted as-sent — accepted risk this sprint (single-operator host),
> **expiry: before any real external effect.**

Sprint 03 sent real email through Resend. This sprint opens real pull requests. The expiry has passed,
and the disposition was *accepted with an expiry*, not *accepted*.

## Why a principal and not a listener check

The cheap fix is "TCP refuses the owner commands." It works, it is a day, and it is a dead end —
`ARCHITECTURE.md:690` names the non-dead-ends explicitly: *"use actor/principal identifiers, isolate
company state, keep layer interfaces explicit… do not design the full future platform."*

A principal costs the same day and is the field the contract already reserves.
`cross-layer` §2.2 lists `principal_id` and distinguishes it from `actor_id` (*"May differ"*);
§2.3 states *"only the Exec principal calls consequential Authority Plane APIs by default"*;
`authority-plane` §4.1 already names the V0 set — `owner principal` and `company/exec principal`;
`owner-cockpit` §14.6 lists `principal_id` among the identifiers every view must preserve.

Every one of those exists on paper and none exists in code. This ticket is not new vocabulary; it is
the first implementation of vocabulary the specs settled three documents ago.

It is also the first brick of multiplayer, should that ever happen: one principal to N principals is
adding rows, not changing shape. That is a consequence, not a justification — nothing here builds for
it (`owner-cockpit` §14.5 forbids invitations, presence and granular human roles now).

## Scope

1. **`principal` on `Request`.** The CLI sets it: `company/exec` when `RESTLESS_COORDINATOR` is set
   (the container case, already how the CLI knows where it is), `owner` otherwise. Absent means
   rejected, not defaulted — a missing principal is the case this ticket exists to catch.
2. **`dispatch()` gates on it, not on the socket.** Owner-only: `approve`, `up`, `down`,
   `clear-poison`. Everything else is open to both, unchanged. This is a list, not a policy engine
   (`authority-plane` §6.5 warns off the DSL, and this sprint puts the Kernel proper out of scope).
3. **Refusal is typed, not prose.** `error.kind = "authority"`, distinct from `transport` and from
   `no-op` — the `BlockKind` enum in `health.rs:44` already has the shape, and S03-T8 item 2 is about
   stopping it being flattened at the boundary. This ticket does not fix that flattening generally; it
   declines to add a new instance of it.
4. **The principal is recorded on the act.** `approval_granted` currently emits `Some("owner")` as a
   literal (`main.rs:~583`). It emits the authenticated principal instead.

**Not in scope:** authenticating the principal cryptographically. On a single-operator host the
container is still trusted to say what it is; what changes is that it now says something, the daemon
acts on it, and the audit record carries it. Hardening that claim is the Authority Kernel's job and is
deferred with a live trigger — the same disposition, but now written down against a concrete field
instead of a comment.

## Acceptance

Two headless calls, both observed.

1. From inside a running company container: `restless approve -c <co> --party x@example.com` is
   **refused** with `error.kind = "authority"`, and the party does **not** appear in company config.
2. From the host: the same command succeeds, and the emitted `approval_granted` event carries
   `principal = "owner"`.
3. A request with no principal field is refused — verified by writing the raw JSON to the socket, not
   through the CLI, since the CLI always sets one.
4. An agent's ordinary coordination still works unchanged over TCP: `commitments`, `message`,
   `commitment`, `spawn`, `effect`. A gate that breaks the agents' channel is a worse bug than the one
   it fixes.

## What this makes deletable

The `main.rs:215` comment, and the ambiguity it stands for. After this, "who may do this" has one
answer in one place rather than an accepted risk in a code comment with a passed expiry.

---
Sprint spec: [`../sprint-04.md`](../sprint-04.md)
