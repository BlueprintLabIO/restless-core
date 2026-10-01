# Attio OAuth MCP through Core: next bounded migration

Status: design and live prerequisites, 28 September 2026. No code or live connection was changed by this document.

## Why Attio is the first OAuth pilot

The existing Restless connected-tool path and Aris dogfood reference Attio at `https://mcp.attio.com/mcp`. [Attio's current MCP documentation](https://docs.attio.com/mcp/overview) lists `whoami` as a read-tier tool. An unauthenticated MCP initialize request returned 401 with OAuth protected-resource metadata; the authorization-server metadata at `https://app.attio.com/.well-known/oauth-authorization-server` advertises dynamic registration, authorization code with PKCE, and refresh tokens. This establishes a plausible live provider, not an authenticated Restless connection or a successful tool call. GitHub's remote MCP was also reachable, but its [official host-integration documentation](https://github.com/github/github-mcp-server/blob/main/docs/host-integration.md) says dynamic client registration is unsupported, so the older GitHub `get_file_contents` prototype cannot assume that `mcp-remote` will obtain a client registration.

## Exact first capability

One owner-selected company connection to Attio, attached to one actor and one Work, exposes only `whoami` with an empty argument object. The owner must select the Attio workspace during sign-in and verify the returned workspace identity before resuming Work. Core pins the authenticated server version and exact `whoami` tool definition. MCP read-only annotations are advisory; the allowlist and empty-argument validator are Core policy. All other Attio tools, including searches and writes, remain unavailable through this connection.

## Ownership and transition

1. Add an owner action to create a **host-only** Attio connection. The existing `connected-tool install` path copies an `mcp-remote` cache into `/company/home/.restless/connected-tools/<name>`; this new action must never take that path. Authentication opens Attio's authorization URL in the owner's browser. A pinned helper receives the callback, writes its cache in a connection-specific host-owned directory, and probes `whoami`. Only after the probe and owner workspace check does Authority enable the connection.
2. For an existing legacy Attio connection, the owner starts migration while its Work is proposed or blocked and no Attempt is running. In one durable state change, mark the connection `migrating` so `session_servers` cannot launch another legacy `mcp-remote` child. Prefer a fresh owner authorization into the new host-only cache. Remove the old Runtime cache for that exact connection and verify absence before enabling broker mode. If the Runtime is unavailable, cleanup fails, or the connection changes concurrently, keep it paused; do not fall back to the legacy path. This transition does not establish that an old agent never copied a token before migration.
3. Issue an existing `issue_mcp_session` grant only for the enabled broker mode, exact company, actor, Work, Attempt, connection, and policy revision. Return Core's MCP endpoint to the actor, with no upstream URL, OAuth cache path, access token, or refresh token. Recheck the live Attempt and current policy on discovery and every call. Reobserve the pinned upstream contract before invocation.
4. Write started and terminal Core receipts with one call ID, pin and policy revision, bounded request/result digests, and an outcome such as `response_observed_unverified`, `auth_required`, or `outcome_unknown`. Do not log token material or full Attio response. A timeout or disconnect after invoking `whoami` has an uncertain outcome and is not retried automatically.
5. Disable immediately revokes subsequent calls, including calls using an already issued Attempt grant. Refresh is owned by the host helper under a per-connection lock: refresh tokens stay in its private cache, replacement is atomic, a failed refresh becomes `auth_required`, and reauthorization is an owner action. New policy or tool schema requires a fresh Attempt and owner review.

## Helper boundary

Do not run third-party `mcp-remote` JavaScript inside privileged `restlessd`. Pin its package and Node versions and run it under a separate unprivileged worker identity with only its own credential directory mounted read/write; no company volume, browser profile, Docker socket, Core database, or other providers' caches. Bound process lifetime, memory, concurrency, stdout/stderr, request and response bytes. Constrain network access to Attio's MCP and OAuth hosts through an enforced egress policy, then have Core communicate over a private local channel. Helper stderr and provider results are untrusted data. Keep the browser callback under the owner's control, outside the company Runtime.

The existing HTTP broker's SSE event cap and decoded result cap do not bound a JSON response before the MCP SDK buffers it. Add a raw upstream response-byte limit before accepting this authenticated provider; a timeout alone does not bound memory.

## Code touchpoints

- `crates/restlessd/src/connected_tool.rs`: host-only install/migration states; prevent legacy cache sync and legacy ACP attachment after opt-in; exact Attio profile; owner-selected workspace observation; reconnect/refresh state.
- `crates/restlessd/src/mcp_gateway.rs`: resolve brokered OAuth connections, validate `whoami` with no arguments, pin/recheck contract, call the private helper, enforce bounds, record start/terminal receipts, and map auth failure without leaking it.
- `crates/restlessd/src/capability.rs`: reuse the Attempt-scoped MCP grant and policy revision; no new long-lived agent credential.
- `crates/restlessd/src/main.rs`, `crates/restlessd/src/wire.rs`, `crates/restless/src/main.rs`: owner commands for host-only connect, migrate, inspect, disable, and receipts.
- `crates/restlessd/src/owner.rs` and `web/src/routes/[companyId]/company/resources/+page.svelte`: owner-only connect/reconnect, workspace confirmation, current scope/pin/status, disable, and receipt view. The account's model-provider sign-in screen is a separate feature.
- A version-pinned helper package and isolated worker launcher: OAuth callback, cache, refresh, and private MCP channel. It must be released as one unit with the daemon.

## Acceptance on a disposable company

Use the owner's Attio account only after explicit owner sign-in. Verify a live authenticated `whoami` result and the selected workspace. Start a fresh Staff Attempt and make the native MCP call; compare its Work/Attempt identity with both Core receipt phases. Confirm no OAuth material in the company Runtime or agent launch contract. Force token refresh by the provider's normal expiry path and repeat the read. Disable the connection and confirm the old Attempt grant fails. Restart the daemon and repeat with a fresh Attempt. If any gate fails, keep the brokered connection paused and leave the legacy path explicitly labelled as such.

No code release should claim general OAuth support, arbitrary Attio tools, write safety, or OS-enforced actor isolation from this one read pilot.
