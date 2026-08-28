# Core/Cloud boundary pointer

**Status:** Core-side boundary record; Cloud owns the detailed implementation contracts
**Cutover:** 28 August 2026

Restless Core remains the one-company autonomous-company cell: Authority, OrgIntel, Company Runtime,
the local owner cockpit and the historical experiment evidence. Restless Cloud owns the public landing
page/research source, Fleet operations, company entry and membership, and the future managed-cell
product surface.

Cloud consumes released Core artifacts and versioned contracts. It must not vendor Core source, use a
submodule as an integration seam or maintain a Cloud fork of company semantics. The public site moved
from Core's `site/` path to `restless-cloud/apps/public/` in the provenance-recorded Cloud transfer;
Core retains evidence locators but no editable public-site source.

Read the authoritative Cloud documents in the
[restless-cloud repository](https://github.com/BlueprintLabIO/restless-cloud): its
[architecture](https://github.com/BlueprintLabIO/restless-cloud/blob/main/ARCHITECTURE.md),
[specification index](https://github.com/BlueprintLabIO/restless-cloud/tree/main/docs/specs) and
[roadmap](https://github.com/BlueprintLabIO/restless-cloud/tree/main/docs/sprints).
