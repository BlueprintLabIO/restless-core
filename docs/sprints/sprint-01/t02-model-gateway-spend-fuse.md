# T2 · Model gateway + spend fuse

**Layer:** Kernel — model credential isolation and budget are kernel concerns (§3.2).
**Serves:** The one authority boundary this sprint actually crosses (secret + budget). Everything else governance-shaped is deliberately out.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T1 (for injecting the base URL into the container).

## Build

- Lift the `company-model-gateway` crate. It is already standalone: HMAC-signed short-lived purpose tokens, server-side provider-key injection, crash-durable file usage spool, fail-closed request limits.
- **Add the missing dollar dimension.** Today it bounds request *counts* only. Per-request cost from a model rate table in config; appended fsync'd to the existing spool; in-memory counter rebuilt from the spool on boot.
- Pre-flight check before each call; reject with a typed error at the ceiling. Fail closed, never fail open.
- The container receives a base-URL override plus a short-lived purpose token. **It never receives a provider key.**

## Acceptance

- With a deliberately tiny ceiling, the Nth call is rejected and the company **pauses inspectably** rather than crashing.
- Grep of container environment and filesystem finds no provider key material.
- Model traffic is observable at the gateway.

## Salvage

Model gateway. **Re-validation:** confirm fail-closed at ceiling with a live tiny-ceiling run, not a unit test.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
