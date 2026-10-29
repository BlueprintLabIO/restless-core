# Metered Claude cache retirement — deployed and verified

Sprint 109 A0.4: avoid paying to replay large cold history. Runtime owns this change; no new
scheduler, model wake, usage meter or background cache-warmer is introduced.

## Observed mismatch and change

The existing retirement policy used a one-hour idle window for every route. Captured managed
Claude Messages requests carried `cache_control: {type: "ephemeral"}` with no longer TTL.
Claude's [documented default cache lifetime](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)
is five minutes; one hour requires an explicit longer setting. A 40,000-token session left idle
for six minutes was therefore resumed by the old policy even though its short cache could be cold.

For Restless-managed, metered `anthropic/*` calls only:

- The Core-owned OMP overlay explicitly selects short cache retention. This covers both the
  direct Messages path and an API-key route through the broker; route selection itself is unchanged.
- Large settled history uses a five-minute idle window before the next existing wake rebuilds
  from its durable context. The 32,000-token cold threshold and 48,000-token lean threshold stay.
- The in-memory settled record retains that turn's window. The persisted session/load decision
  derives the same window from the authenticated launch route, so restart does not restore the
  universal one-hour assumption. The existing locator schema and scope checks stay intact.
- Warm history and small history remain reusable. Other harnesses, native/subscription routes
  and providers keep their prior conservative window; this work does not infer their TTL from
  a Claude API-key request.

The timer starts at the observed usage/settlement point. It is a conservative post-turn idle
window, not an exact timestamp of the provider's cache write. Cache hits and dollar savings
are not measured by the protocol test below.

## Verification

- Policy regression: a 40,000-token record remains warm at 299 seconds and becomes cold at
  300 seconds on the metered route; the subscription case remains warm. A 32,000-token record
  stays reusable. The persisted locator round-trip applies the same policy at six minutes.
- ACP tests: 16 passed, zero failed; three live tests remain separately invoked/ignored.
- Model gateway tests: 61 passed, zero failed, including route and credential boundaries.
- The separately invoked live OMP test passed against the existing local scripted provider.
  It supplies the actual Rust-generated overlays to the pinned CLI, with fake credentials and
  no provider access. A long environment hint produces `ttl: "1h"` with the unchanged control
  overlay, and `{type: "ephemeral"}` with the metered overlay. Thus the application setting,
  not a fabricated expected request, reaches the real provider adapter.
- The helper uses ephemeral ports, exclusive disposable profiles and bounded children. Its
  provider, CLI processes, profiles and temporary files were removed. The pre-existing broker
  was untouched.

Commands:

```sh
scripts/restless-cargo test -p restless-engine acp::tests::
scripts/restless-cargo test -p restless-engine model_gateway::tests::
scripts/restless-cargo test -p restless-engine live_metered_cache_overlay_sends_five_minute_cache_controls -- --ignored --nocapture
```

The first policy test caught an implementation mistake: tying the policy predicate to the
boot-time dual-provider registry made a metered key look unknown before that registry was
populated. The final predicate uses the authenticated billing/model contract; the existing
dual-provider condition still controls the actual Messages routing. That failed log is retained
at `/tmp/s109-cache-policy-test.log`; successful results are `/tmp/s109-cache-policy-test2.log`,
`/tmp/s109-cache-acp-tests.log`, `/tmp/s109-cache-model-tests.log` and
`/tmp/s109-cache-wire-final.log`.

## Production release

Core `0e3c44c17bfee3a401d9ad0de7aa395a5800c872` is live through Cloud release
`cloud-3488969efb35b46ac458d54118076ff38d657bf0`, retaining qualified Cloud source
`7602eba31c824a9a0e3801fad2bf2ab50e673a3d`. This also deploys PR 516's explicit hidden
intent-marker instruction and its measured short reply contract. At 21:00:56 UTC on 29 October
(30 October Sydney), desired/applied revisions were `125/125`, healthy, with a fresh heartbeat
and no error. Public site, provider, relay and one-address checks passed.

Full Core qualification passed; Connections passed 23 checks with no failures and the separate
skills smoke skipped. The first registry transfer failed on a network request after signing.
The unchanged official transfer/verification tail resumed from the signed handoff, without a
second build. Public signatures verified, and all four image manifests were inspected before
deployment. Direct read-only server inspection confirmed the running account-plane and cockpit
image digests match this exact signed assembly. Temporary operator SSH access was removed.

The existing 8 GB/four-CPU plan is unchanged. The workload pool remains capped at
5,974,450,176 bytes, three CPU cores and zero workload swap. Its OOM/max counters were zero.
All six active company workloads in the snapshot were protected Documents services with their
duplicate Docker health checks disabled; their individual OOM counters were zero. Host available
memory was 5,340,143,616 bytes (about 5 GiB); a short sample observed zero swap in/out. This is
a point-in-time operational observation, not a matched workload efficiency benchmark.

Authenticated production Reno browser testing remains unavailable; no session was bypassed.
Whole-task provider cost/quality, scheduler-enabled no-change wakes, longer histories/media,
editable compute caps and the remaining A0/Sprint 109 requirements stay open. No production
cache-hit or dollar saving is inferred from the local wire test or this deployment.
