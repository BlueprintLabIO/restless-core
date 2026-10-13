# Sprint 80 — Direction and permission (Core)

**Status:** landed (Core half; web and Fleet are the cloud session's half)
**Programme:** multiplayer; owner cockpit
**Spec:** `docs/specs/direction-and-permission.md`
**Depends on:** Sprint 79b (roles that hold at once), the S80 contracts (`f8025632`)

## Outcome

A member asked Exec to keep the website copy in first person. Exec answered in four paragraphs
that it had changed nothing, and nothing was recorded or routed. Now each change a person directs
takes one of three paths, and every step of it is recorded and visible:

- **Direct:** the person owns that team, or is the owner. Exec applies it.
- **Act, then tell:** the change is reversible, and a grant or the default allows it. Exec applies
  it, records it with its undo handle, and whoever owns the direction sees it in For you with Undo.
- **Ask first:** anything else. Exec files a request. It reaches the owner's Inbox (or the team
  person's) with Approve, Approve and always allow, and Decline. Exec carries it out on approval.

## Changes

- **Authority records** (`direction_authority.rs`): applied, undone, request, decision, outcome,
  grant and revoked records go in the append-only governance store. Partial unique indexes make a
  request answered once and an undo happen once. The second writer is told who answered first.
- **Routing:** `classify` decides the path from the team's person, standing grants, and the
  default (a member's content and Work changes in a team nobody owns apply, then tell).
- **Exec CLI:**
  - `restless direction path|applied|done|grants`;
  - `restless request file|show`.

  The `direction-operation` daemon command admits only Exec (or the owner's own socket). A runtime
  cannot forge its actor.
- **API:** the routes below. Members see only their own requests and changes. The person a request
  is routed to may decide it. The owner and admins decide and undo anything, under their own names.
  Standing permissions belong to the owner alone.
  - `/direction/requests`;
  - `…/{id}/decision`;
  - `/direction/applied`;
  - `…/{id}/undo`;
  - `/direction/grants`;
  - `…/{id}/revoke`;
  - `/direction/digest`.
- **Projection:** open requests become Inbox items, with a title, created_at and brief strings like
  any other item. Applied changes roll up into one For you row a day per person (migration 0100
  admits the `direction-day:` read key; schema 100).
- **Undo:**
  - **Content:** team briefs, and document versions through S77 restore.
  - **Staffing:** an agent's model.
  - **Schedule:** cancels a schedule the change created.
  - **Work:** restores its owner or its goal.

  Undo refuses when the target changed again since ("changed since"). It tells the requester in
  their Exec room: "Mara undid: …".
- **Doctrine** (`context.rs`, "People own teams"): the proposal sentences become the three paths.
  Never refuse in paragraphs; never apply an Ask-first change before approval.
- **Plane projection v3** (`company_projection.rs`): adds `requests_waiting` and
  `changes_applied_last_day`. These are counts only, so the record still cannot hold content.
  Fleet writes the digest line from them. The fixture is `contracts/company-projection.v3.fixture.json`.
  The v1 and v2 fixtures stay for Cloud.

## Acceptance

Multiplayer smoke step 9i, run with Bob as an admin:
1. Alice's content rule on an unowned team applies at once. The owner's For you shows it, and Undo
   restores the brief.
2. A spend change asks first and reaches the owner's Inbox. Approve, then Exec's `done`, turns it
   Done for Alice.
3. Approve, and always allow: Alice's next staffing change applies. After Revoke, it asks again.
4. Make Alice owner: her next direction for that team is Direct.
5. The owner and admin Bob approve one request at once. One gets 200; the other gets 409 "… already
   approved this."
6. With a change applied and a request open, the digest counts both.
7. Alice never sees Bob's requests, and gets 403 from grants and Undo.

Unit: four DB tests in `direction_authority`, the CLI serialization test, the actor-pin test, and
the v3 projection record and fixture.

## Deploy order

Fleet must accept projection v3 before a plane running this sends it. A v2-only Fleet would refuse a
v3 record. Migration 0100 is additive (it widens a CHECK).
