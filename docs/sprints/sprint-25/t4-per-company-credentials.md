# S25-T4 — Per-company credential custody

Let the account plane hold more than one account per provider and bind each to the cell that needs it.

**Observed friction:** `model_gateway.rs` bails with *"V0 model gateway refuses different {provider}
credentials across companies; separate provider custody before multi-account use."* It reads like an
isolation boundary and is the opposite: an owner-scoped resource that has not learned to serve
companies with differing needs. Today the only workaround is running a second daemon — which is what
made "how many daemons should there be?" unanswerable in the first place.

**Layer:** Authority Plane (account plane).

**Deletion target:** the multi-account bail, and second-daemon-as-workaround.

## Scope

- Provider credentials key on (provider, company) with an owner-level default, rather than provider
  alone.
- Broker canonicalisation reconciles per binding, not per provider.
- A company inherits the owner default unless its config names its own reference.

## Acceptance

Two companies of one owner, each with a different account for the same provider, both start and both
route to their own credential. Neither can reach the other's.

## Blocked — external capability not established

The model path cannot currently select a credential per request:

- The Runtime relay forwards **one** bearer to **one** OMP `auth-gateway`
  (`model_gateway.rs`, `start_runtime_relay`).
- OMP's broker canonicalises to exactly one credential per provider — that is what
  `canonical_api_key_credential` and `canonical_oauth_credential` enforce, and why superseded rows are
  disabled.

So per-company custody needs either an OMP capability for per-request credential selection, which has
not been probed, or one gateway process per credential set — a real design choice with its own cost.

**Not guessed at.** The bail stays until the external capability is established by a live probe
rather than assumed from an API's shape. Probe OMP's gateway for a credential/account selector first;
if absent, price one gateway process per credential set against the actual owner need.
