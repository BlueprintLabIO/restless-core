# S27-T1 · Verify an assertion at the plane edge

**Layer:** Authority Plane.

**Observed outcome or friction:** There is no supported way to reach a Restless plane that is not the
machine you are sitting at. `ensure_loopback` (`crates/restlessd/src/owner.rs:717`) refuses a
non-loopback bind outright, and `local_owner_boundary_violation` (`owner.rs:850`) refuses any request
carrying a forwarding header. ADR 0001 chose both deliberately and named the condition that ends them:
a supported network entry point with a real account and session boundary.

**Work:** Add one additional entry mode. The plane runs in **local mode** (today's behaviour, the
default, unchanged) or **network mode**, in which access is decided by verifying a signed assertion.

Network mode requires explicit configuration naming issuer, audience and verification key. Absent
complete configuration the plane refuses to start in network mode, naming the missing field — it never
silently downgrades to trusting the network. `ensure_loopback` becomes conditional on entry mode; the
forwarding-header refusal stays in force for local mode.

A verified assertion is consumed once and exchanged for the plane's own revocable session. Per ADR
0001's standing invariant and ADR 0007, both modes resolve to the same stable owner principal and run
the same application and Authority operations. Authentication proves who may assume the principal; it
grants no Authority capability and does not become a second authorisation system.

The Sprint 05 owner bearer token is not reintroduced in any form, including as a development shortcut.

**Evidence:** A plane started in network mode with a valid assertion serves the cockpit, and the audit
attribution records the same stable owner principal that local mode produces — compared against a
local-mode run, not asserted. A plane started in network mode with an incomplete configuration fails
at startup naming the missing field. A plane started with no network configuration binds loopback and
behaves exactly as before, with its existing tests unchanged and passing.

**Deletion target:** The unconditional loopback bail as Restless's only network posture.
