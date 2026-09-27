# Host-owned MCP gateway

The company actor can call a small set of host-owned MCP tools without
receiving the upstream service bearer, browser profile, cookies, or CDP
endpoint. This is for local Docker-bridge deployments. Remote OAuth providers
continue through the existing `mcp-remote` path; a company-local stdio MCP
still runs inside its Runtime volume.

## Install and use

1. Run the upstream MCP server on host loopback with a private token file.
   Clapping Hands uses `127.0.0.1:8799/mcp` and a user systemd service. Its
   owner-only token file is outside every company Runtime.
2. Choose a proposed or blocked Work owned by the intended active Staff actor. Install an
   exact endpoint, actor, Work, and one or more exact tool names with the owner
   CLI `restless local-mcp --company <company> install-host --name <name> --endpoint
   http://127.0.0.1:8799/mcp --token-file <absolute-private-file> --actor
   <actor> --work <work-uuid> --tool <tool-name>`.
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

## What the status means

`ready` means MCP initialization and permitted tool discovery succeeded. It
does not assert site login. A `last_success_at` is recorded only for a typed
complete read, alongside the site and tool; blocked, auth-required, incomplete,
and tool errors remain separate statuses. Asking prices and availability in
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
