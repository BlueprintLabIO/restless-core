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
Local owners prepare the host's sharing configuration from **Company → Members →
Enable sharing**. The account plane includes all existing companies with their
original immutable identities; accounts select only companies they may access.
The installed appliance's `enable-sharing` command applies entry settings with
preflight, drain and restoration on failure. A raw local-owner port forward is
not team access.

## Sharing over Tailscale (recommended)

Tailscale is the supported way to reach a self-hosted Core from teammates'
devices. It only carries HTTPS on the host's tailnet name; every person still
signs in with their own verified account, and nothing is published to the
internet. Both addresses use the one `*.ts.net` name: the company on `443` and
the account service on `8443`.

Requirements: Tailscale installed and signed in on the Core host (inside WSL
when Core runs there), MagicDNS and HTTPS Certificates turned on for the tailnet,
and permission for your user to configure it (`sudo tailscale set
--operator=$USER` once on Linux). Each teammate joins the tailnet, or receives
this machine through Tailscale sharing.

1. Print the exact addresses:

   ```sh
   restless appliance tailscale-addresses
   ```

2. In **Company → Members → Enable sharing**, choose **Private network** and
   enter those two addresses, then run the account installer from the
   [account service guide](../services/identity/README.md) on the download.
3. Start the prepared account service, then activate:

   ```sh
   restless appliance enable-sharing --tailscale \
     --environment /srv/restless/accounts/core-entry.env
   ```

   This refuses a setup prepared for other addresses, and refuses to replace a
   different `tailscale serve` route or one published with Funnel. It publishes
   the account route, runs the ordinary activation (preflight, drain, restore on
   failure, and removes that route again if activation fails), publishes the
   company route, and then checks both addresses over the tailnet name with a
   trusted certificate. Tailscale's HTTPS replaces the prepared
   `Caddyfile`s.
4. `restless appliance tailscale-doctor` repeats the reachability check later.

Tailscale gives both origins one hostname, so the account session cookie is
also sent to the company address (cookies ignore ports). Core is trusted code
on the same host, so this is accepted; separate hostnames remain possible with
your own DNS and HTTPS proxy.

**Fallback: SSH SOCKS.** Without Tailscale, an OpenSSH SOCKS tunnel
(`ssh -N -D 127.0.0.1:1080 user@host`) can carry a teammate's browser, set to
that proxy with proxy DNS, to the configured HTTPS addresses. You operate the
HTTPS routing and certificates yourself in that case.

The multiplayer journey — three verified accounts, invitation, concurrent
document editing, room chat, a member's mention and removal — runs nightly as
`scripts/multiplayer-smoke` (see [launch readiness](launch-readiness.md)).

See [the network entry boundary](adr/0007-network-owner-entry-by-verified-assertion.md)
and [the provider-neutral identity contract](adr/0009-provider-neutral-company-access-context.md).
