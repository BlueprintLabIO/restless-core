# Local MCP browser owner-control lifecycle

Status: historical analysis of an in-company browser broker design. The
2026-09-27 Clapping Hands integration instead keeps its authenticated browser
on the host and exposes three restricted, read-only MCP tools through the
Attempt-scoped gateway described in [host-mcp-gateway.md](host-mcp-gateway.md).
The owner-visible company browser pause/resume contract discussed below is
still unresolved, but it is not used by that host-owned integration.

## Current boundary

- `crates/restlessd/src/connected_tool.rs::session_servers` selects MCP servers
  for an actor and receives optional `work_id` and `attempt_id`. The local
  descriptor carries `assigned_actor` and an owner-set `broker_aware` flag.
  For broker-aware Codex sessions, the runner forwards ambient
  `RESTLESS_COMPANY`, `RESTLESS_ACTOR`, `RESTLESS_COORDINATOR`, and
  `RESTLESS_SESSION_CAPABILITY` by name. The signed capability is already
  scoped to the actor session and, for Staff, its Work and Attempt. No token
  value is stored in the descriptor or launch contract.
- `crates/restlessd/src/staff/execution.rs::run_staff_with_failover` and
  `crates/restlessd/src/exec.rs` put the returned servers into
  `AgentControls`. `crates/restlessd/src/acp.rs` transmits them in ACP
  `session/new`. ACP's server list provides process launch configuration, not
  a per-tool-call lifecycle callback to Restless.
- `crates/restless-runtime-bridge-protocol/src/lib.rs::Message` has
  `LaunchAgent`, `AcpStdin`, `AcpStdout`, and `Cancel`. `LaunchAgent` carries
  actor and responsibility, but no Work or Attempt identity. `Cancel` ends the
  whole ACP operation; it cannot pause one MCP call while preserving unrelated
  actor work.
- `crates/restlessd/src/runtime.rs::watch_browser_control` is subscribed to by
  the desktop websocket path in `owner.rs`; ACP and local MCP calls do not
  subscribe to it. The in-company broker enforces owner control with HTTP 423
  and revokes automation WebSockets, but that only blocks browser protocol
  traffic. It does not deliver a typed pause to the owning MCP call.
- `owner.rs::return_control` records idempotent Work feedback against the
  requesting actor and exact running Attempt, then clears the browser lease.
  That feedback is durable, but the bridge has no call identifier or resume
  channel with which to wake a suspended MCP invocation after hand-back.

Therefore cancelling `LaunchAgent` on owner takeover is too broad, while
letting the stdio MCP call discover 423 and return an ordinary tool error is not
a pause/resume contract. The latter can let the actor continue while the owner
is using the browser. Neither behavior meets ADR 0014.

## Code map for the required change

1. **Descriptor and session binding** — the owner-only `--broker-aware` opt-in
   forwards the signed actor coordination environment only to that local
   Codex stdio child. Ordinary local and OAuth MCPs do not receive it. A
   future browser-call lifecycle still needs Runtime generation and exact
   call identity. Do not put secrets or a CDP URL in descriptor storage.
2. **Call lifecycle protocol** — extend
   `crates/restless-runtime-bridge-protocol/src/lib.rs` with a correlated
   brokered-call lifecycle (start, owner-pause, hand-back/resume, terminal
   outcome). It must be fenced by Runtime identity/generation and exact
   actor/Work/Attempt, and must not be implemented as `Cancel` of the whole
   `LaunchAgent` operation. Update protocol validation and the host/runtime
   dispatch in `crates/restlessd/src/runtime_bridge.rs` together.
3. **Owner transition wiring** — in `owner.rs`, publish pause for the exact
   `requesting_actor`, `work_id`, `attempt_id`, and owner lease when control is
   acquired. On `return_control`, resume only after the existing idempotent
   Work feedback has been persisted and the lease is unclaimed. Lease expiry,
   Runtime-generation change, bridge loss, or missing hand-back must terminate
   the suspended browser call as a non-success outcome; never resume on lease
   expiry alone.
4. **Company-side enforcement** — the Clapping Hands browser provider must run
   inside the company Runtime, use only broker port 9223, and keep endpoint
   details inside that process. It must consume the correlated lifecycle,
   stop browser work on 423/revocation, and resume only after the matching
   hand-back signal. On resume it must reconnect through 9223 and inspect
   current page state before continuing a read. It must not launch another
   Chromium, use 9222, return a CDP URL, or perform writes.
5. **Acceptance** — in an isolated company, start one harmless read, take
   owner control during it, verify the exact Attempt's read pauses while an
   unrelated actor/Work remains unaffected, return control, and verify that
   only the same generation and Attempt resumes after the durable hand-back
   feedback. Also verify 423, lost bridge, generation change, and lease expiry
   are explicit non-success outcomes and do not create a replacement browser.

## Public-only headless read carve-out

The broker lifecycle above applies to Clapping Hands sessions that use the
owner-visible company browser. It does not prohibit a separate ordinary
read-only MCP that opens one public page in a fresh, unauthenticated headless
profile, closes that browser when the read finishes, and exposes no endpoint,
authentication, recording, or write tools. This fits
`docs/COMPANY_OPERATING_RULES.md`'s stateless headless-capture exception.

The current Clapping Hands source includes `src/cash-converters-server.ts` and
the `clapping-hands-cash-converters` package bin. It implements a one-tool Cash
Converters public listing reader. Its URL is constructed from one 12-digit item
ID under the fixed `www.cashconverters.com.au` electric-guitar listing path.
It uses a fresh temporary profile per call and does not call the company
browser broker. This public-read path does not need the owner-control pause
protocol above. Build a reproducible archive from the current committed source
before staging it. This tool is an integration probe, not a film-camera search.

The public wrapper sets `/usr/bin/chromium` when present and passes
`--no-sandbox` only for that fixed company-image binary. The company image's
existing visible Chromium launcher also uses `--no-sandbox`; this tradeoff was
used only in the disposable compatibility probe, not deployed by this task.
In a company container, a compromised page renderer would share uid 2000 and
could read that company's mounted files; an empty temporary profile does not
provide a renderer security boundary. Keep any initial dogfood company free of
credentials and valuable files, or resolve the sandbox limitation before
installing this reader into a company volume.

The intended install and actor-scoped descriptor shape, once a current archive
has a recorded SHA-256 and an isolated `_test` company exists, is:

```sh
npm install /company/inbox/clapping-hands-cash-converters.tgz \
  --prefix /company/clapping-hands --omit=dev --ignore-scripts
```

```text
company: <isolated _test company ID>
name: cash-converters-public-read
command: /company/clapping-hands/node_modules/.bin/clapping-hands-cash-converters
assigned_actor: exec
args: []
broker_aware: false
```

Confirm that npm created this executable at the shown path; the bin name is
`clapping-hands-cash-converters`. The local MCP patch supports this absolute
`/company` command shape. Do not treat these placeholders as an installation:
no company or shared service was
changed by this source port. Deploying the patched daemon and installing a
verified package into the intended company are separate steps.

The public reader must be installed without `--broker-aware`, so its MCP child
receives no signed actor capability. The flag is reserved for a separately
approved, broker-aware package after the per-call owner-control lifecycle is
implemented and verified; setting it alone does not make Facebook browsing
safe or functional.

The exact archive was installed into a disposable container from the inspected
company image (Node v24.21.0, uid/gid 2000, read-only root, temporary storage
only, no credentials and no company volume). The MCP handshake advertised
exactly one read-only tool and one public listing call succeeded for item
`039200502009` (“Electric Guitar Black”, A$299.00, Capalaba). The scratch
container was removed after the call. This verifies package compatibility in
that image; it does not verify a Restless descriptor, company-volume install,
or brokered-browser lifecycle.

## Brokered-browser gate

ADR 0014's Clapping Hands provider seam is a prerequisite, not proof of this
Restless lifecycle. The current protocol cannot represent the required
per-call pause/resume, and the company MCP process has no safe hand-back signal
to await. Implementing only a watch, cancellation, environment variable, or
generic MCP error would leave a race or stop unrelated work. Keep broker-backed
local MCP descriptors disabled until the protocol and company-side provider
are integrated and the acceptance scenario passes.
