# Host-owned MCP gateway

The company actor can call a small set of host-owned MCP tools without
receiving the upstream service bearer, browser profile, cookies, or CDP
endpoint. This is for local Docker-bridge deployments. Remote OAuth providers
continue through the existing `mcp-remote` path. An explicitly reviewed,
networkless filesystem stdio MCP can now run in a Core-owned Docker worker;
arbitrary company-local stdio MCPs still run inside the Runtime volume.

## Install and use

1. Run the upstream MCP server on host loopback with a private token file.
   Clapping Hands uses `127.0.0.1:8799/mcp` and a user systemd service. Its
   owner-only token file is outside every company Runtime.
2. Choose a proposed or blocked Work owned by the intended active Staff actor. Install the
   reviewed `clapping-hands` connection at its local MCP endpoint, actor, Work,
   and a reviewed three-, four-, or five-tool sourcing read profile with the owner
   CLI `restless local-mcp --company <company> install-host --name <name> --endpoint
   http://127.0.0.1:8799/mcp --token-file <absolute-private-file> --actor
   <actor> --work <work-uuid> --tool <tool-name>` (repeat `--tool` for each read tool).
3. Install probes the server through the MCP SDK, records its version and the
   digest of the permitted tool definitions, and enables that connection.
   A changed version or tool definition requires owner reinstallation.
4. When that Work starts, Staff receives a signed MCP capability for its exact
   company, actor, connection, Work, and Attempt. The capability expires after
   the normal session TTL. The Runtime relay endpoint accepts only that grant
   while the Attempt remains live and un-interrupted.
5. `restless local-mcp --company <company> disable --name <name>` or the owner Resources
   action disables new calls. An in-flight call checks connection and Attempt
   state every two seconds and stops waiting if revoked. The upstream read may
   already be underway, so disabling is not a rollback of that read.

The MCP route shares the existing Runtime model relay listener on port 7790
(plus `RESTLESS_PORT_OFFSET`), which is already reachable from the company
container through `host.docker.internal`. The route rejects peers outside
the local Docker bridge `172.17.0.0/16` and host loopback before checking its
separate signed MCP grant and live Attempt; it never accepts a model grant.
The upstream server remains bound to host loopback. The proxy uses the MCP
SDK on both sides, advertises only the installed allowlist, and rechecks tool
definitions before every invocation.
It refuses calls after a 90-second bound and responses beyond 1 MiB. It does
not retry an uncertain call automatically. No generic browser tools are
installed for Clapping Hands.

The current host HTTP bridge admits only the reviewed Clapping Hands sourcing
profile: `clapping-hands` at its local port (7799 plus the Restless port offset)
with `clapping_hands_marketplace_search`, `clapping_hands_marketplace_details`,
and `clapping_hands_gumtree_public_listing`; the reviewed four-tool profile adds
`clapping_hands_gumtree_public_listings`, and the five-tool profile adds
`clapping_hands_marketplace_photo`. Older exact Work pins remain valid; adding
either tool needs a fresh install and Attempt. The Gumtree batch accepts 1–8
exact URLs and retains a typed row per page, including blocked and incomplete
outcomes. Gumtree's current edge 403 remains an access failure, not a successful
batch read. The photo tool accepts one exact Facebook item URL and one bounded
photo position, returns a JPEG image block, and marks text-only details as
having uninspected media. Other host HTTP tools fail closed at
installation, actor launch, and request handling. A server's MCP
`readOnlyHint` is not evidence that the code cannot write. This profile trusts
the owner-managed Clapping Hands service and its audited read surface; a
server version or tool schema pin detects declared contract changes but is not
proof of behavior. Future effectful MCP tools need adapters that obtain
Restless effect permits for exact payloads and reconcile uncertain outcomes.
Existing company-local stdio and remote OAuth MCPs can still expose arbitrary
tools; this host HTTP policy does not govern their effects.

## What the status means

`ready` means MCP initialization and permitted tool discovery succeeded. It
does not assert site login. A `last_success_at` is recorded only for a typed
complete read, alongside the site and tool; blocked, auth-required, incomplete,
`search-unverified`, and tool errors remain separate statuses. A narrow query
that cannot produce verified cards is not a confirmed empty result. Asking prices and availability in
Clapping Hands results are observations, not verified sale outcomes.

## Boundaries

- Linux Docker bridge address `172.17.0.1` and the company Runtime's
  `host.docker.internal` mapping are required. A different local network
  needs an explicit peer-gate configuration. Local MCP availability follows
  the Runtime model relay lifecycle; actor execution already requires that
  relay and an admitted model route.
- The upstream token file is read by the host daemon and must be an absolute,
  private regular file. Its bytes are never serialized into company status or
  the actor launch contract.
- Fresh actor sessions are needed after capability expiry. Disabling or
  reassigning a connection does not change the MCP configuration of a running
  actor process, but every gateway request checks current authority.
- Clapping Hands owns its separate browser profile. The company browser's
  owner-control pause/resume mechanism does not govern that profile.

## Broker boundary and next migration

Core is already the MCP client and actor-scoped MCP server for the reviewed
Clapping Hands, DeepWiki, and filesystem-read profiles. It owns the connection
pin, live Attempt grant, request validation, and read receipts. CH remains the
owner of its authenticated browser and extraction code. This is the intended
division for Sydney Resale.

Company-local arbitrary stdio tools and remote OAuth providers still attach
directly inside the Runtime. They need separate migration before Restless can
claim to broker every MCP tool. That migration requires a raw HTTP response
size limit before the rmcp client buffers JSON and error bodies, host-owned
provider credentials, reviewed per-tool authority, and effect permits with
reconciliation for writes. MCP `readOnlyHint` alone cannot establish those
properties. The current same-UID Runtime actor boundary also does not stop one
actor from reading another Attempt's grant file; treat the grant as logical
scope until the runtime process boundary is strengthened.

## Model connection recovery during the pilot

The Staff read also depends on the company model relay. A transient failure
while the host OAuth broker supplies its account identity previously surfaced
as a generic 403 and could set a 24-hour credential cooldown. The relay now
returns 503 for a typed broker failure and gives its snapshot up to five
seconds; a revoked assignment or changed account identity still returns 403.
The 503 follows the short transport cooldown rather than marking the OAuth
connection revoked.

For an older cooldown caused by the relay's ambiguous 403, the owner can run
`restless credential verify-model -c <company> --model <primary-provider/model>`
to compare the current OAuth account identity with the registered owner
connection. Add `--clear-relay-cooldown` only after that read-only check. The
action deletes only an active credential cooldown for that exact company and
model whose recorded failure contains Core's `company model access was removed`
marker; it writes an Authority receipt in the same database transaction. It
cannot clear a provider's own 403 cooldown or bypass a failed identity probe.
Verification proves the current account assignment, not that a subsequent
model inference or marketplace read will succeed.

Roll out daemon and owner CLI from the same source revision after draining
running sessions, then check `/health`, the model relay listener, the startup
recovery barrier, and appliance admission before resuming Staff Work. Retain
the previous daemon and CLI for rollback. If the new relay or owner command
fails, restore the previous release and leave the cooldown and CH version pin
unchanged until the failure is diagnosed.
