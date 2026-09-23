# Custom harness integration

The owner requested installed custom harnesses to be selectable for Exec and Staff, with independent
credentials. The daemon registry/endpoints, Intelligence provider controls, independent Vault keys,
and generic ACP agent routing are deployed locally. Live reads expose the Hermes and OpenClaw
presets; browser fixtures verify installation/assignment controls without changing owner state.
Hermes and OpenClaw have passed full host-driven native turns with exact assignment, coordination
readiness, scoped shell execution and streaming against isolated HTTP model fixtures. Real account
authentication remains unverified and requires the owner to complete each harness's native setup.
Historical checkpoints below describe earlier verification boundaries.

`manage.mjs` installs into a private per-harness profile and probes real ACP. Linux `flock` serializes
installation attempts; status checks distinguish interrupted processes without overwriting a live job.
Discovery denies filesystem/tool requests, bounds framing and time, and retains native setup methods
when session creation fails. `compatible` means the protocol advertised models; it does **not** prove
that credentials are accepted or a model can produce a response. `authentication` remains `unverified`.
An optional requested model is selected through the actual protocol before discovery returns it.
No arbitrary harness stderr is returned to the owner API.

`legacy-models.mjs` adapts the legacy ACP model menu and `session/set_model` acknowledgement into
config options. It preserves exact model IDs (including colons and slashes), session scope, errors and
modern config options. Its response metadata identifies the evidence as `legacy-acknowledged`.
The productive proxy now uses this adapter and preserves actor identity and passes the Core instruction file through native system-context handling.
The current generic route explicitly reconstructs each turn from Core context and reports thinking effort
as harness default; it does not pretend to set an unsupported effort or silently reuse changed native configuration.

`presets.json` records official install/setup commands. Hermes' installer owns uv at
`$HERMES_HOME/bin/uv`, not `$HOME/.local/bin/uv`. Its local profile disables AWS instance-metadata
credential discovery: on this local Linux runtime that lookup stalled initialization. Credentials must
come from the harness's own setup or explicitly supplied references. Global MCP discovery is disabled;
Restless can supply session MCP explicitly. Independent per-harness home/config/cache paths are forced.

`openclaw.mjs` starts a loopback Gateway for each Restless launch with a private token file. It
shares the native account state/refresh locks within that harness profile, keeps separate actor
databases, and uses unique Gateway sessions and working context. It reads the actual configured
model catalogue through `models.list` and acknowledges a selection only after `sessions.patch`
confirms the exact session and provider/model. Model IDs preserve nested slashes and colons.
Core instructions enter the native system context through the launch workspace's AGENTS.md;
ordinary requests remain user turns. The native tool allowlist excludes OpenClaw's independent
coordination and secrets tools. The scoped host identity reaches native shell tools. Probe launches
receive no actor grant. The adapter reconstructs each turn; it does not claim native hot-session reuse.

The tested 2026.9.5 CLI accepts `OPENCLAW_GATEWAY_TOKEN` for local Gateway calls, but its ACP bridge
requires `--token-file`. Passing `--url` to `gateway call` rejects environment-only authentication;
the adapter uses its isolated configured local target. No credential values appear in process argv.

## Verification

```sh
node --test tools/custom-harness/*.test.mjs
```

18 Node regressions cover process/concurrency behavior, credential isolation/redaction, exact model
selection, legacy session scope, malformed/fragmented transport, bounded failures and denied probe tools.

Real runtime checks on 2026-09-22 used a separate `restless-custom-harness_test` container with no company
volume or user credentials. Hermes 0.21.4 installed, passed `hermes acp --check`, initialized and exposed
its native terminal setup method; session creation correctly remained unavailable without provider setup.
OpenClaw 2026.9.5 installed; its native Gateway/ACP process selected the fixture model, made two
requests to an isolated deterministic HTTP provider, executed a harmless real shell command with the
correct test actor/capability, and streamed two text chunks. The captured request also proved that
Core context was a system message and private coordination/secret tools were absent. This verifies
the adapter and native execution, not live provider authentication or a full Restless-host turn. Hermes browser/computer-use setup
was skipped in the protocol-only smoke check; the ordinary preset retains those installer steps.

The subsequent full host contracts installed each real preset in a fresh disposable Runtime, assigned
only `delivery-build`, probed and selected the exact native model, reached a current-code isolated
coordination listener with a signed session capability, executed a shell digest proof, and streamed the
fixture reply through `acp::with_agent`; Exec's existing route remained unchanged. OpenClaw's contract
also exposed an immediate probe-to-turn Gateway relaunch failure: force-killing discovery left the next
Gateway unable to acquire/bind before readiness. The adapter now requests graceful termination, waits
three seconds for native lock cleanup, and uses SIGKILL only as a fallback. Both contracts passed after
that repair. Their deterministic HTTP providers do not verify live OAuth or native account refresh.

Sources:
- https://hermes-agent.nousresearch.com/docs/user-guide/features/acp
- https://hermes-agent.nousresearch.com/docs/getting-started/installation
- https://docs.openclaw.ai/cli/acp

## Owner integration checkpoint

The host stores ordinary harness configuration in `custom-harnesses/<company>.json` and keeps keys as
Infisical references. Owner-only endpoints save configuration, install, launch native setup, store keys
and probe models. Agent assignments use `harness:custom:<id>` and preserve the full native model ID.
A fresh selection probe is required before saving an assignment. Native setup verifies an actual
visible xterm window before returning the Computer route, instead of returning a generic desktop.
The existing generic native billing classifier was corrected so custom routes are not mislabeled
as subscriptions. Settings revisions invalidate cached discovery after configuration changes.

Verified: three Rust registry/routing tests, the owner/member boundary regression, 16 Node tests,
Svelte check (zero errors/warnings), type ramp and production frontend build. The browser-only verifier
`web/scripts/verify-custom-harnesses.mjs` covers preset/custom forms, independent key entry, models,
retry and 1440/390/320px layouts without real company writes. A separate real Hermes native setup
opened a visible terminal in the isolated test computer. Source-first visual reference pass used
Beautiful UI status rows, Cult UI disclosure restraint and shadcn-svelte Field labels; no library code
was copied. The preview server was stopped after verification.

This historical checkpoint preceded the host-turn verification below. Hermes and OpenClaw host turns now pass, and the daemon containing the repaired adapter is deployed locally. Real account consent remains an owner setup step.

## Reproducible OpenClaw transport smoke

In a disposable `_test` company runtime with OpenClaw installed and no existing native configuration:

```sh
RESTLESS_COMPANY_ID=custom_harness_test node tools/custom-harness/openclaw-smoke.mjs
```

The smoke refuses to replace existing native settings, creates only fixture provider configuration,
checks discovery/exact selection/system context/scoped shell execution/streaming, and removes its
configuration and processes. It must not run against owner company state. The isolated installation
container used during development was removed after verification; no native sign-in was performed.

OpenClaw setup uses the official onboarding flags to skip installing a second Gateway service,
bootstrap persona, channels, hooks, skills and its separate UI. Required provider plugins from native
onboarding are retained in the launch configuration and shared extension directory; native account
setup and those plugins still need real-account verification. Flags/reference:
https://docs.openclaw.ai/cli/onboard . The isolated streaming test used no provider plugin.

## Automatic catalogue refresh

Ordinary intelligence reads and the provider page schedule background discovery after ten minutes
for successful catalogues, or thirty seconds for incomplete setup. One job per company/harness and a
shared two-process limit avoid duplicate launches. Cached results remain visible during the check;
arriving models refresh the picker without changing its current selection. Credentials/configuration
changes invalidate the cache, and native setup triggers a fresh check. A source fingerprint avoids
rewriting runtime helpers on every read.

The host passes explicit secret environment names to discovery. Redacting every environment value
corrupted legitimate model IDs; the real runtime refresh regression caught and verified this fix.
18 Node tests, four Rust cache/registry/routing tests, a real Docker background-refresh regression,
Svelte checks/build and 1440/390/320 browser fixtures passed. This checkpoint preceded local
deployment and the native system-context correction described below; both are now complete.

## Native system context and Hermes execution

Generic ACP commands must reference `${SYSTEM_PROMPT_FILE}` through a native system-instruction
argument. The runner forwards user messages unchanged; it no longer hides standing instructions
inside the first user message. The owner form and registry validate this requirement.

Hermes uses `hermes.py`, a process-local native ACP entry point. Its SessionManager keeps provider
resolution, independent login and model switching. The agent factory supplies Core's system prompt,
registers an explicit file/terminal/web/browser toolset, and disables private memory, identity,
background review and transcript storage. The latter also avoids an unrelated model call to generate
an internal session title. Installed Hermes source is unchanged. Native version controls are checked
before constructing an agent.

The ordinary full installer completed for Hermes 0.21.4 in an isolated runtime. The real native
`hermes-smoke.mjs` check then selected `custom:fixture-model`, made two calls to a deterministic local
HTTP provider, executed a harmless shell command with the test actor/capability, and streamed two
text chunks. Captured requests contained Core system instructions and excluded native vault,
memory and delegation tools. This verifies the native adapter, not real account authentication
or a complete host-driven Staff/Exec turn. The smoke only accepts `_test` companies and restores
the native config after use.
