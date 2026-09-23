# Authenticated entry with a local company computer

Signing in and placing the company computer are separate choices. A self-hosted
account plane can verify independent human accounts while managing its computers
and Documents services through local Docker.

| Entry mode | Runtime mode | Deployment |
| --- | --- | --- |
| `local` | `local` (default) | Loopback access as the local owner |
| `network` | `hosted` (default) | Verified accounts and an externally managed Runtime Bridge |
| `network` | `local` | Verified accounts and local Docker computers |

Set `RESTLESS_RUNTIME_MODE=local` alongside the existing network-entry
configuration to select the third row. An unknown value fails startup. Hosted
Runtime mode requires network entry. Omitting the variable preserves existing
deployment behavior.

Network entry still requires the configured identity issuer, its JWKS endpoint,
owner and plane UUIDs, exact account-plane hostname, and a company image pinned
by OCI digest. It uses the same signed, single-use handoff, durable human Actor
mapping, membership checks and revocation path in every deployment. Selecting
local computers does not change anyone's permissions.

The local Documents service verifies tokens bearing the public account-plane
issuer. It fetches Core's public verification key over the private loopback
listener, so it does not need public DNS or a route through a TLS proxy to reach
its own host. Document traffic still passes through Core's authenticated proxy.
Local company processes use the existing capability-protected local model relay.

The [account service](../services/identity/README.md) provides verified sign-in,
versioned membership, and durable suspension and removal using Better Auth. It is
the same issuer Restless Cloud mounts (ADR 0012). People are invited and managed
from the company's **Company → Members** page; the account service keeps only
sign-in, verification, invitation acceptance and **Open company**. Its setup guide
connects the service to this deployment mode, including for a company that began
in local mode. Another issuer must publish the same metadata, entry and
membership-control contracts.
The full self-hosted invitation and cockpit journey is still being qualified in
[launch readiness](launch-readiness.md).

See [the network entry boundary](adr/0007-network-owner-entry-by-verified-assertion.md)
and [the provider-neutral identity contract](adr/0009-provider-neutral-company-access-context.md).
