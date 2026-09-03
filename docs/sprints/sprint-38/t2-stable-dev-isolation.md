# S38-T2 — Isolate stable daily use from development

**Layer:** Machine host, Runtime and developer tooling

**Serves:** Improving Restless must not destabilise the Restless instance the founder is using.

## Work

- Give stable, dev and `_test` profiles explicit non-overlapping roots, database scopes, sockets, port
  ranges, container/volume labels, browser state, logs and launch caches.
- Make every destructive or mutating developer command resolve and display its profile before acting.
- Refuse implicit attachment to a discovered stable daemon from a dev checkout.
- Add exact profile ownership to cleanup and migration operations.

## Acceptance

Run stable and dev concurrently, then adversarially invoke dev stop, reset, migrate, attach and cleanup.
Stable health, company state, Runtime, schedules, browser and launched artifacts remain unchanged. The
same test proves stable operations cannot consume dev resources.

## Makes deletable

Port-offset folklore, broad container cleanup, shared browser homes and environment-dependent profile
inference.
