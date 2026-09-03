# S37-T1 — Own endpoint identity and one access descriptor

**Layer:** Authority contract + Cloud provider projection

## Observed friction served

Raw node IPs, provider application IDs and `sslip.io` were useful diagnostics but are not a stable,
trustworthy product handoff. Web and game clients also received different piles of infrastructure
details rather than one portable native target.

## Outcome

Extend the released publication receipt compatibly with one derived access descriptor containing only
the exact information a browser or native client needs: publication and candidate identity, released
profile, owned hostname, protocol endpoint, expiry, admission-exchange location and launch metadata.

Cloud provisions wildcard DNS, valid certificates, deterministic route identity and UDP allocation.
Core validates and records the descriptor but does not learn provider topology or credentials.

## Acceptance

- A ready web publication resolves through an owned certificate-valid HTTPS/WSS hostname.
- A ready game publication exposes an HTTPS join document plus an allocated UDP endpoint.
- The descriptor is bound to one exact `publication_id` and candidate digest and is stable for that
  publication lifetime.
- No descriptor names a Runtime address, provider root, internal container address or reusable bearer.
- `sslip.io`, node IP and provider application identifiers cannot satisfy released-ready validation.
- Older `published-service.v1` consumers either ignore the additive projection or fail with an honest
  version incompatibility; they never misroute.

## Makes deletable

Manual hostname/port copy instructions, diagnostic-domain release paths and protocol-specific Cockpit
configuration.
