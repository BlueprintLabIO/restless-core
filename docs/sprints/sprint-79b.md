# Sprint 79b — A role change holds at once

**Status:** landed
**Programme:** bug fixing (admins, access)
**Depends on:** Sprint 79. (Numbered 79b: Sprint 80 is the direction-and-permission spec.)

## Outcome

Making someone an admin, or taking it back, holds at once. Their session from before the change
ends, and they come back with the new role. Before, the role waited for the person's next entry:
a promoted member could not act as an admin, and a demoted admin kept admin rights until their
session renewed. Cloud confirmed production had the same gap through the same issuer code.

A member refused something an admin may do is told "Only the company's owner or an admin can do
that." Before, it named the owner alone, which was not true.

The multiplayer smoke covers an admin for the first time.

## Findings and causes

| Finding | Cause |
|---|---|
| The owner made Bob an admin; 30 s later Core still called him a member | the issuer's role change bumped the version but sent Core nothing ("the next handoff carries both") |
| Once the issuer sent it, Core refused it twice | the entry verifier accepted only `suspended` and `removed` controls, the apply step required a terminal status, and the receipts table checked for terminal statuses |
| Changing a suspended member's role would have reinstated them | the first fix enqueued the role change as `active`; a role change now keeps the member's status |
| A member refused an admin action was told only the owner could | one refusal sentence served every refused route |

## Changes

- **Issuer** (`services/identity/src/membership-sql.mjs`, published as `@restless/issuer` 1.1.1):
  - a role change enqueues a membership control with the new role, the next version and the
    member's current status, through the outbox suspension and removal use;
  - the controls table admits `active`, with an `ALTER` for installs made before;
  - only `suspended` and `removed` mark a member as ending access or hold entry.
- **Core:**
  - the control verifier accepts `active`, and the apply step accepts a non-terminal status;
  - for someone who never entered, an `active` control records nothing, since a denial would keep
    them out;
  - migration 0099 widens the receipts check; schema 99.
  - ADR 0012 records the change and its rollout order: Core before the issuer.
- **The member boundary** names "the owner or an admin" for the routes an admin may use.

## Regression checks

- Smoke step 9h:
  - the owner makes Bob an admin from Members;
  - Bob's earlier session ends within 30 s, and he re-enters as an admin;
  - his screens, Company settings included, make no refused request;
  - Alice, a member, deleting the logo is refused with "owner or an admin";
  - the owner takes the role back, his admin session ends, and he re-enters as a member.
- `services/identity/test/membership-role.test.mjs`:
  - a role change enqueues an active control with the new role and the next version;
  - taking it back is the same;
  - an unauthorised change enqueues nothing;
  - a suspended member's role change stays suspended.
  - Three of these failed before the fix.
- `restless-engine` entry: an active control verifies with its role and version; an unknown status
  is refused.
- `restless-orgintel` access:
  - an active control applies;
  - the old session is no longer current, and the new role and version are;
  - for an unseen member it records nothing, and their first entry succeeds.
- `restless-owner`: a member is told "owner or an admin" for the logo.

## Rollout

Core (schema 99) ships before Fleet re-vendors issuer 1.1.1. An older Core refuses active controls
and the outbox retries them, so the order is safe either way, but the fix only holds with both.

## Results

- Multiplayer smoke: 24/24, exit 0.
- Every `restless-orgintel` test; `restless-owner` 122/122; `restless-engine` entry 21/21; identity service 11/11.
- Schema version 99.
