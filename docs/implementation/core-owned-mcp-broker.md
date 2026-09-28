# Core-owned MCP broker: target and migration

## Decision

Restless Core is the authority and call path for company MCP tools. An actor
receives a narrow MCP namespace bound to its company, actor, Work, and Attempt.
Core acts as an MCP client to the upstream service and as a scoped MCP server
to the actor. It is therefore the broker, including for tools whose browser
or provider process is outside Restless.
Core owns connection setup, upstream credentials, tool discovery, per-call
policy, revocation, outcome recording, and owner controls. The upstream service
owns its domain state: for Clapping Hands, that includes its browser profile and
Facebook login. Neither cookies nor the upstream bearer enter the company
Runtime.

The first live slice is the [host MCP gateway](host-mcp-gateway.md) for three
reviewed Clapping Hands read tools. A separate
[public HTTP pilot](public-http-mcp-pilot.md) uses the same Attempt-scoped
Core broker for one reviewed DeepWiki tool and one owner-selected public
repository. An isolated [filesystem stdio pilot](brokered-stdio-filesystem-pilot.md)
uses the same route for one reviewed local read tool and a sandboxed host worker.
These are brokered profiles, not a general broker for every MCP transport.
Existing arbitrary company-local stdio servers and `mcp-remote` OAuth
connections still execute inside the company Runtime. They need migration
before Core can make a general security or audit claim.

## Call path

1. The owner installs a connection and chooses the exact actor, Work, and
   allowed tools. Core discovers the upstream contract and pins the version
   and tool schema digest. Discovery metadata alone is not proof that a tool is
   read-only.
2. At each Attempt launch, Core issues a short-lived grant for the exact
   company, actor, connection, Work, and Attempt. The actor sees the broker
   endpoint and permitted tool descriptions; it does not see the upstream
   endpoint, browser debugger, credentials, or refresh tokens.
3. Core checks the live Attempt and current owner policy on every discovery
   and call. It checks the pinned upstream tool contract before invocation,
   bounds runtime and result size, and appends started and terminal call
   receipts binding the actor, Work, Attempt, connection, tool, pin, policy
   revision, status, duration, and request/result hashes without copying
   upstream secrets or arbitrary private response bodies. Revocation
   prevents new calls and stops waiting on in-flight reads, while marking
   their external outcome uncertain if necessary.
4. Effectful tools require a separate Restless effect permit for an exact
   action and payload, an idempotency or reconciliation key where possible,
   and an owner decision when the company mandate requires one. Tool names or
   MCP `readOnlyHint` values do not grant this authority.

### Optional fixed-Work read-call limit

An owner may set `--max-calls-per-work N` (1–10000) when installing a reviewed
`install-host`, `install-public-read`, or `install-stdio-read` connection. The
default is `null`, meaning unlimited as before. Core exposes the configured
limit in `local-mcp list` and the owner Resources connection card. For a fixed
Work grant, Core locks the connection and reserves a `started` receipt before
the upstream call. It counts all started receipts for that company, connection
and Work across resumed Attempts and later pins, including timed-out or
uncertain calls. Concurrent Attempts cannot both consume the last slot. Once
the limit is reached, Core returns a typed `work_call_budget_exhausted` /
`not_invoked` result and writes a terminal denial receipt without an upstream
call. `tools/list` and invalid arguments do not consume a slot. The limit
counts broker tool calls, not listing URLs or returned pages.

If Core cannot reserve a call because its pin or receipt store changed, it
fails closed before contacting the provider and returns typed
`broker_reservation_failed` / `not_invoked`. It writes a terminal receipt when
the receipt store is available; a lost commit acknowledgement may leave a
started reservation that still counts toward the limit.

This fixed-Work limit does not apply to recurring Opportunity Work grants,
which require a separate recurring policy if they need a call budget. It also
does not cover legacy direct company-local stdio or `mcp-remote` OAuth calls.
On an existing connection, omitting `--max-calls-per-work` preserves its prior
limit, including on re-pins; a new connection with no limit starts unlimited.
To remove an existing finite limit, the owner must explicitly reinstall with
`--unlimited-read-calls` (mutually exclusive with `--max-calls-per-work`). The
owner Resources re-probe also preserves the current setting.

```
Staff actor -> scoped MCP namespace -> Core broker -> reviewed adapter -> upstream MCP
                                          |                         |
                                   policy, audit, status       CH browser or provider
```

## Transport migration

| Transport | Current state | Target |
| --- | --- | --- |
| Host Streamable HTTP, CH | Core relays three reviewed read tools; host owns bearer and browser. | Keep this path as the first end-to-end acceptance case. Add owner-visible pause, login recovery, and version switch with fresh sessions. |
| Public Streamable HTTP, DeepWiki | Core has a reviewed no-auth profile for `read_wiki_structure` on one selected public repository. | Prove a fresh actor call and append-only receipt in an isolated company before release. Add other providers only with reviewed endpoints, exact tools, and argument validators. |
| Remote HTTP with OAuth | A company-side `mcp-remote` child reads credentials from its Runtime directory. | Core-owned OAuth and refresh, with a broker adapter exposing only approved tools. Migrate an existing grant by a reviewed owner flow; never silently copy a shell-readable token into a new trust boundary. |
| Local stdio | Legacy children run in the company Runtime with the actor. The filesystem pilot launches one published provider in a local-image-ID-pinned Docker worker with no network and one read-only data mount. | Add reviewed profiles one at a time, with explicit filesystem/network envelopes. Route calls through Core policy and audit; do not execute arbitrary third-party stdio code in the privileged Core process. |

The actor's tool list must be tested in its *actual* Codex mode. A Responses
namespace such as `mcp__clapping_hands` contains child functions; counting
only top-level functions can falsely report that the tools are absent. A
successful `tools/list` or model request containing a namespace still does not
prove a real tool call. The acceptance check is a fresh Staff Attempt reading
an exact listing through Core, followed by a fresh Attempt repeating it.

The September 2026 company model route uses OMP 18.3.2. Its prebuilt
`pi-coding-agent/dist/cli.js` skipped `type: "namespace"` tools when
translating a Codex request to the model request, even though Codex exposed
the three CH children correctly. Patching `pi-ai/src` alone did not affect
that prebuilt CLI. The host launcher now applies a versioned OMP patch and
runs its source CLI; the patch translates namespace children into callable
model tools and exposes the three reviewed CH reads without relying on
`tool_search` or deferred loading. The source patch and pinned package must
be upgraded together, with a real actor call after each change.

On 28 September 2026 AEST, Staff Attempt
`bcac3e2a-978f-471b-9c05-20fe1c97e702` made a native
`clapping_hands_marketplace_details` model tool call for the exact RX 6800
listing `965653713159638`. Core recorded `host MCP read completed`, and the
tool returned `complete`, displayed ask `AU$450`, `document-replay`, and an
observation at `2026-09-27T16:05:21.305Z`; provider wall time was 6,038 ms.
This verifies the native model-to-Core-to-CH read path for that Attempt. It
does not verify listing availability, GPU condition, cooler noise, or a
general MCP transport migration. The Attempt later blocked while trying to
write its report because its advertised `apply_patch` tool was unavailable;
the model tool call and Core log remain independently observable. That
historical Attempt predates Core's durable call receipts, so it does not
validate the new receipt path. The
compatibility `ch-read.mjs` command
remains available for recovery, through the *same Core broker* and Attempt
grant, but its use must be reported separately from native MCP calls.

### Pilot isolation limit

The compatibility command reads an Attempt grant from a `0600` file in the
company Runtime. That mode protects the file from other Unix users, but the
current company agents share the `company` Unix identity and Runtime volume.
A same-UID peer that finds another live Attempt's file could copy its grant
and impersonate that scoped actor until the grant expires or the Attempt
ends. Core still enforces the signed company, actor, Work, Attempt, connection,
tool allowlist, and live-Attempt checks; those checks do not authenticate the
process holding a stolen bearer. The pilot must not be described as providing
OS-enforced actor isolation. Move each actor into a distinct UID/container or
replace the file with a credential-free, process-authenticated broker channel
before treating this as a general multi-actor security boundary.

## Release gates

- The CH pilot reads a current listing from its saved authenticated profile in
  a real company Staff Attempt. The owner status page shows the typed result,
  exact pin, actor scope, and disable control. Owner pause and hand-back are
  verified without a second process opening CH's browser profile.
- One OAuth provider and one local stdio provider are migrated through the
  broker in isolated smoke runs. The stdio filesystem worker has passed a
  direct provider/sandbox smoke; it still needs a fresh real Staff Attempt
  through Core and a revocation check. Revoking a grant blocks a subsequent
  call; the company Runtime has no reusable provider credential. The old
  direct paths remain labelled as legacy until those migrations pass.
- Effectful tool calls obtain a payload-bound permit and reconcile uncertain
  outcomes before retry. No generic write-capable MCP server becomes enabled
  merely because its metadata claims it is safe.
- The status page distinguishes connection, site authentication, completed
  read, partial/blocked read, and uncertain result. Build success and tool
  discovery are not reported as a successful marketplace read.
