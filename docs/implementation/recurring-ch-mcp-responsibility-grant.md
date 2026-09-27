# Recurring Clapping Hands MCP grant: implementation contract

Status: design only. No recurring MCP authority is implemented by this document.
The current `clapping-hands` connection remains pinned to one Work. Do not
change its row or the active sourcing Attempt while implementing this design.

## Observed gap and ownership

`schedule create-responsibility` binds a schedule to an immutable responsibility
version. A due occurrence admits an Opportunity; Exec or its accountable lead
may commission Staff Work and link that Work to the Opportunity. The scheduler
does not create production Work merely because a time fact fired. Today the
reviewed CH connection stores one `assigned_work_id`; both actor launch and the
MCP gateway compare against that exact UUID. A new daily Work therefore has no
CH tools until the owner re-pins the connection.

The owner should approve a **standing eligibility policy**, while Core still
issues a fresh, short-lived MCP grant only for one real Staff Attempt. The
policy is restricted to one company, existing reviewed CH connection, recurring
schedule UUID, responsibility UUID and version, and Staff actor. It copies the
connection's exact four-tool read allowlist, server version, contract
digest, endpoint identity and current connection policy revision. It grants no
OAuth, local stdio, browser-control or effectful MCP tool.

## Storage and owner API

Add an Authority-owned `mcp_recurring_read_policies` table keyed by company,
connection name and schedule UUID. Keep the existing `local_mcp_servers`
fixed-Work row and `assigned_work_id` unchanged. A policy stores the immutable
responsibility ID/version, exact actor, approved CH pin and tool list, its own
UUID policy revision, enabled state and owner timestamps. This separate row
lets an owner approve or revoke future daily Work without re-pinning an active
fixed-Work Attempt. A connection disable or reinstallation must also make every
dependent recurring policy fail closed through the saved connection revision.

Expose owner-only CLI commands to approve, inspect and revoke one schedule
policy, for example `local-mcp approve-recurring --name clapping-hands
--schedule UUID --responsibility UUID --version N --actor STAFF` and
`local-mcp revoke-recurring --name clapping-hands --schedule UUID`.
Inspection shows both the standing policy and the existing fixed-Work pin.
Approval performs a fresh MCP probe and requires its server version and exact
selected-tool digest to equal the enabled connection's pin. It also verifies
the connection's reviewed `clapping_hands_v1` host HTTP profile and exact
four-tool allowlist. The schedule must be uncancelled and recurring, name the supplied
responsibility version, and belongs to this company; the actor must be active
Staff and match the connection's assigned actor. This command never changes
the connection's Work assignment, token file, endpoint or browser profile.
Changing a schedule's responsibility version requires new owner approval.

## Atomic Work provenance

The present `work add` and `schedule link-work` calls are separate. Staff Work
can be claimed between them; a later link must not retrospectively authorize
an Attempt. Add optional `--opportunity UUID --owner-epoch N --schedule UUID`
arguments to Work commissioning, all required together. Inside
`OrgIntel::add_work_inner`'s **existing transaction**, validate the live
Opportunity lease under a row lock, its exact epoch and commissioning actor,
open state and deadline. Require an existing `admitted` or `coalesced` occurrence for that
Opportunity and schedule, with matching responsibility ID/version. Select its
latest `scheduled_for` under the transaction. Create the Work and insert its
`primary` `opportunity_work` link before commit. The existing
one-primary-per-Opportunity index arbitrates competing commissions. Add
nullable `source_schedule_id` and `source_scheduled_for` columns to
`opportunity_work`, constrained to be both null or both set and referenced to
the occurrence's composite key. Only this atomic commissioning path sets
them; it records the exact occurrence that conferred eligibility. Existing
`schedule link-work` continues to record ordinary outcome provenance with
both source columns null. It cannot grant MCP authority. No Work or link is
visible to the scheduler until the transaction commits.

The OrgIntel read query for recurring eligibility returns the exact
Opportunity UUID only when all of these still hold:

- the Work owner is the approved Staff actor and a running Attempt names that
  Work and actor;
- a `primary`, atomically commissioned link has the approved schedule UUID and
  its stored source occurrence key;
- the linked Opportunity is open and within its deadline, with the approved
  responsibility ID/version;
- that exact stored occurrence is still admitted or coalesced and tied to the
  linked Opportunity; and
- that schedule remains uncancelled, recurring and bound to the same
  responsibility ID/version.

The owner-approved Goal, a matching title, the schedule wake message and a
post-creation link alone are not sufficient. Goal membership can be assigned
to unrelated Work and cannot represent this exact occurrence boundary.

## Actor grant and request checks

At Staff Attempt launch, `connected_tool::session_servers` resolves the
eligible Work lineage. It advertises the existing Core MCP URL and mints a
grant with company, actor, connection, Work and Attempt as today, plus the
recurring policy identity/revision. Existing fixed-Work grants keep their
current format and behavior. If the same Work also matches the fixed pin,
advertise the connection once and prefer the fixed grant.

`mcp_gateway::scoped_connection`, discovery, call admission and the two-second
in-flight revocation watch recheck both the enabled CH connection and the
specific recurring policy, then re-resolve the OrgIntel lineage above. A
recurring grant uses that policy's identity/revision instead of the fixed
`assigned_work_id` comparison; a fixed grant keeps the existing comparison. A
revoked policy, cancelled/rebound schedule, expired/settled Opportunity,
changed Work owner, interrupted Attempt, changed CH connection revision, or
changed upstream tool contract prevents new calls. A read already sent to CH
may have completed; stop waiting and record an uncertain outcome rather than
claiming rollback. Keep the existing CH argument validators, call/result
bounds, server-version and tool-definition checks, and no automatic retry.

Append the current started and terminal Core receipts for every accepted
call. Preserve their exact Work and Attempt fields and add nullable schedule,
Opportunity, responsibility ID/version and recurring policy revision fields
for direct owner attribution; fixed-Work receipts leave these null. The
receipt records the lineage observed at admission, even if a later revoke
suppresses the result. No upstream token, cookie, request body or response body
enters Authority.

## Failure boundaries and acceptance

The bearer in an actor launch contract remains stealable by a same-UID peer in
the current company Runtime. The existing gateway still enforces its signed
Work/Attempt and live policy scope, but this change does not provide OS-level
actor isolation. Do not claim that boundary until separate identities or a
process-authenticated channel replaces the grant file.

Before release, complete one isolated `_test` company smoke using the real CH
MCP service: approve the policy without mutating the fixed pin; admit an
Opportunity; atomically commission and link a Staff Work; observe a native CH
call and its matching started/terminal receipts with Work, Attempt, schedule
and Opportunity. Confirm a same-Goal Work, a Work sourced from another
schedule that coalesced into this Opportunity, and a Work linked only by the
old `schedule link-work` path receive no recurring CH tool. Revoke the policy and
confirm the old grant fails; confirm the prior fixed-Work connection remains
usable. Repeat the lineage check after schedule cancellation or version
change, and fail closed on an upstream contract change. No live company
service, browser profile, or existing connection pin needs to be changed for
the implementation build.

Code boundaries: `crates/restless-orgintel/migrations/` and
`src/goals_work.rs`/`src/schedules.rs` own the atomic link and lineage query;
`crates/restlessd/src/connected_tool.rs` owns policy installation and actor
attachment; `capability.rs` carries the recurring policy identity;
`mcp_gateway.rs` owns request-time rechecks and receipt attribution;
`main.rs`/`wire.rs` and `crates/restless/src/main.rs` expose the owner commands
and optional atomic Work arguments. The owner Resources view can follow after
the CLI path passes the live smoke.
