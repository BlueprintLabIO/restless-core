# Core/Cloud boundary pointer

**Status:** Core-side boundary record; Cloud owns the detailed implementation contracts
**Cutover:** 28 August 2026

Restless Core remains the one-company autonomous-company cell: Authority, OrgIntel, Company Runtime,
the local owner cockpit and the historical experiment evidence. Restless Cloud owns the public landing
page/research source, Fleet operations, company entry and membership, and the future managed-cell
product surface.

Cloud consumes released Core artifacts and versioned contracts. It must not vendor Core source, use a
submodule as an integration seam or maintain a Cloud fork of company semantics. The public site moved
from Core's `site/` path to `restless-cloud/site/` in the provenance-recorded Cloud transfer;
Core retains evidence locators but no editable public-site source.

## What Core owes Cloud

Cloud consumes three things from Core, and none of them exist yet:

1. **An account plane that verifies a network identity assertion.** Core's owner gateway is
   loopback-only by construction (`crates/restlessd/src/owner.rs:717`, `owner.rs:850`). Until this
   ships, a Cloud sign-in gates nothing. See [ADR 0007](../adr/0007-network-owner-entry-by-verified-assertion.md)
   and [Sprint 27](../sprints/sprint-27.md).
2. **A pinnable release manifest** naming account-plane and Runtime image digests, schema versions, and
   the API and assertion contract versions.
3. **A health/version probe** on plane and cell reporting the release identity actually running.

Cloud owns the issuing half of identity (Better Auth), mints a short-lived assertion and **redirects**
the browser to the owner's account plane. It does not reverse-proxy company traffic; see Cloud's
[ADR 0001](https://github.com/BlueprintLabIO/restless-cloud/blob/main/docs/adr/0001-owner-plane-entry-and-routing.md).

## The tier Cloud was missing

Reviewing the Cloud specification set on 30 August 2026 found it described **two** tiers — Fleet and
cells — with no account plane; the phrase appeared zero times in that repository. Two consequences had
followed silently: the cockpit had been attributed to the cell (Core serves it from the plane,
`owner.rs:740`), and credential custody had nowhere to live but a shared multi-owner service,
contradicting the structural property [`CELL_ARCHITECTURE.md`](../CELL_ARCHITECTURE.md) §3 claims.

Cloud's architecture, specs, ADR 0001 and roadmap are now corrected to three tiers. **Core's tier model
was already right and is unchanged.** Recorded here because the same class of divergence — a document
set drifting from the tier model without either side noticing — is what Sprint 25 fixed in the code,
and it will recur.

Read the authoritative Cloud documents in the
[restless-cloud repository](https://github.com/BlueprintLabIO/restless-cloud): its
[architecture](https://github.com/BlueprintLabIO/restless-cloud/blob/main/ARCHITECTURE.md),
[specification index](https://github.com/BlueprintLabIO/restless-cloud/tree/main/docs/specs) and
[roadmap](https://github.com/BlueprintLabIO/restless-cloud/tree/main/docs/sprints).
