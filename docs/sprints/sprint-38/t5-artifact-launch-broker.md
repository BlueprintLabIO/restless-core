# S38-T5 — Build a bounded artifact launch broker

**Layer:** Owner plane, Authority and machine host

**Serves:** A completed artifact is not useful if the owner must reconstruct ports, tokens, client
builds or launch commands.

## Work

- Project `embedded_web`, `native_client` and `company_computer` launch descriptors from existing
  artifact, publication, Work and authority truth.
- Verify native platform, digest/signature, audience, expiry and exact prepared-session identity before
  launch.
- Exchange a short-lived opaque local handle for launch material without placing reusable capabilities
  in URLs, arguments, shell history or logs.
- Track child lifecycle and exact owned cache/temp paths; make repeated Open idempotent.
- Package Swift Arrival as the counted macOS client without changing its native ENet transport.

## Acceptance

Each released profile opens its exact target; wrong digest, platform, audience, subject, expiry and
revocation fail before useful access. Cancellation, client exit and expiry remove scoped material and
orphaned owned processes. Arbitrary executable paths and URLs are rejected.

## Makes deletable

Hand-written game commands, copied bearer tokens, mutable client downloads and project-specific UI
launch code.
