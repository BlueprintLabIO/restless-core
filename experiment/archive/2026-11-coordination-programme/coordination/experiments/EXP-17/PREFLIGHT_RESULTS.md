# EXP-17 first-party Codex parity preflight

**Disposition:** passed on 30 August 2026; this satisfies P0 but does not activate counted EXP-17.

The no-count probe used one byte-identical pinned runner in a neutral controller and in a real Restless
Staff Attempt. It admitted exact `gpt-5.6-sol` at `high` effort through a host-held scoped Responses
relay. The neutral actor created and gated a fixed artifact, accepted a duplicate semantic event only
once, resumed the same durable Codex thread after replacing the runner process, then interrupted a live
long-running shell tool and proved its process was gone. The same image and runner then produced a
native unsent customer-response package as Staff. A separate OMP lead inspected it without changing its
bytes and advanced the exact handoff to owner-ready state. No external effect occurred.

## Defects found and closed

The preflight found issues that direct happy-path calls had missed:

1. Codex's Responses base path needed an explicit `/v1/responses` host relay.
2. The prior provider proxy could not relay Codex tool-continuation payloads, so Responses now uses a
   host-only direct upstream route while Runtime receives only a signed exact-model capability.
3. Exact GPT-5.6 tariffs and terminal usage accounting were missing from that route.
4. Test-plane scheduling raced manually controlled probes; it can now be disabled only when every
   configured company is explicitly suffixed `_test`.
5. The runner serialized all input behind the deferred `turn/start` response. That made interrupt and
   steering unavailable during a live tool. Turn admission is now notification-driven and non-blocking;
   a regression test withholds the start response until after interrupt.
6. Duplicate request IDs could execute twice, cancellation cleanup could race Codex's thread lock, and
   short-lived controller objects were cleared before their evidence counters were copied. Each now has
   an explicit guard and cleanup assertion.

## What this establishes

It establishes transport and custody parity for the primary `C` versus `R1` experiment: exact model and
effort admission, the same native session runtime, durable resume, at-most-once semantic delivery,
observable tool/usage events, live cancellation, exact host accounting and clean process/artifact
teardown.

It does not establish that Restless improves outcome quality or efficiency. Counted work remains gated
on EXP-16 closure and a separate immutable task, hidden-fixture, order, rubric and budget freeze. The
machine-readable evidence is [`PARITY_MANIFEST.json`](PARITY_MANIFEST.json).

## Counted-image rerun

After the neutral one-task and longitudinal controllers were added, the exact counted image was rebuilt
and the complete no-count probe reran rather than assuming an additive image change was harmless.

- image: `sha256:36a80bda308854197aaf5c1b68dbfb51c24326bf8e8bd0d6bc986a3d766d7738`
- runner: `1c12128dd4d4fc51ad0e59a1d24bf6df96af68219ff45c9e4ac4d42eeb3662f1`
- requested/observed producer: `litellm/gpt-5.6-sol` / `gpt-5.6-sol`, `high`
- neutral artifact: `d7a51723d521aebe790c2a085aaca405eeab415aaab4d3bdfbbc1447e0acc2fc`
- durable thread: `01a0519f-9054-7100-bda7-6abf60c2c54e`; resume passed
- duplicate semantic deliveries: `1`; cancellation: `interrupted`; long process reaped: `true`
- supervised Staff Attempt: `produced`; lead preserved exact candidate bytes and made the handoff
  owner-ready; external effects: `0`

The final exact-image rerun passed in 123.27 seconds through its isolated current-code coordination listener. An
earlier invocation omitted the plane's port offset and correctly failed readiness before a producer
prompt; a second invocation reached the resident listener and passed but did not prove the intended
isolation. The test coordinator now uses the same override-aware address path as both agent runtimes,
and the third clean company proved the corrected path. This prevents a launch-wrapper mistake or an
older resident daemon from masquerading as current product evidence.

## Task-network isolation rerun

A later no-count R1 rehearsal exposed one remaining ambient source of nondeterminism: Codex startup
briefly spawned a plugin/skill network helper even though the producer task never requested network
access. The counted runner now disables multi-agent, plugin, skill-discovery, app, browser, computer-use
and image-generation features and gives standard child network clients a fail-closed proxy while
exempting only the host model relay. The final pinned image is
`sha256:36a80bda308854197aaf5c1b68dbfb51c24326bf8e8bd0d6bc986a3d766d7738`; the runner is
`1c12128dd4d4fc51ad0e59a1d24bf6df96af68219ff45c9e4ac4d42eeb3662f1`. A fresh company repeated the
entire neutral transport, durable-resume, duplicate, cancellation, process-reap, supervised production
and immutable lead-review proof. It reported `host-model-relay-only-v1`, no ambient helper appeared,
all evidence cleaned up, and accounted USD 0.4760 against a USD 10 ceiling.

The same final image also passed a fresh no-count rehearsal of the complete R1 controller in 220.24
seconds. Exec delegated exactly once, the non-producing lead commissioned exactly one Codex producer,
the producer passed the frozen local gate, and a second lead wake admitted the byte-identical ReviewTarget.
The lead changed no artifact bytes, no external effect occurred, no Codex/OMP process remained, and the
isolated company accounted USD 0.6258 against a USD 5 ceiling. This proves that the benchmark topology
executes; it remains deliberately excluded from comparative outcome results.

## Controller and longitudinal rehearsals

The generic neutral task controller initially failed before a model turn because it generated, but did
not create, its isolated Codex home. Both neutral controllers now create that scope explicitly. A fresh
single-turn fixture passed on the final image in 20.71 seconds for USD 0.0941, with exact local output,
native gate success, reported network policy and no retained Codex process.

The first neutral longitudinal rehearsal then killed the runner but exposed an independently sessioned
foreground tool that survived as an orphan. The controller now records that tool's Linux process group,
kills that exact group on process replacement and proves absence before resuming. The replay preserved
the same durable Codex thread, ignored a duplicate causal signal byte-for-byte, applied the scheduled
signal after replacement, passed its final gate and left no orphan. It completed in 149.21 seconds for
USD 0.4995.

Finally, the complete longitudinal R1 topology passed the same synthetic event contract: one Exec
delegation, one lead commission, three supervisory wakes, ten worker usage snapshots, one exact process
replacement, immutable lead review, final gate success and zero external effects. It completed in
482.97 seconds for USD 1.7432. Its checkpoint monitor is now event-driven without a 30-second success
deadline; the ordinary model/runtime liveness envelope remains the safety boundary. These rehearsals
prove executable controller semantics, not a quality advantage, and are excluded from counted pairs.
