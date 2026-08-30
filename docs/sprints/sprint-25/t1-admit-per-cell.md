# S25-T1 — Admit companies per cell, not per plane

Make model admission a per-company fact. The account plane validates its own credentials and records
which companies it cannot start, with the reason, instead of refusing to boot for all of them.

**Observed friction:** two abandoned `_test` companies naming a provider with no credential, plus
`aris` naming `anthropic` with no OAuth in the broker, made the whole installation unbootable — for
every company, including those whose own credentials resolved. The first symptom the owner saw was
`restless company list` failing.

**Layer:** Authority Plane (account plane). Admission is a credential concern, and credentials are
owner-scoped, so the decision belongs beside them.

**Deletion target:** whole-plane boot validation of every configured company's full model chain.

## Scope

- `provider_credentials` resolves; a new `admit` decides startability after broker reconciliation,
  because a reference that resolves from config can still have no account behind it.
- A company whose **primary** model has no admitted provider is unstartable. An unadmitted **failover**
  candidate is dropped with a warning — the chain is a fallback, not a requirement.
- `ordered_candidates` filters candidates whose provider was never admitted, so the boot-time warning
  and the runtime chain agree rather than failing one request deep into a wake.
- A provider that fails broker canonicalisation is dropped, not fatal.
- `runtime::up` refuses an unstartable company with the exact reason rather than deferring the failure
  into its first Attempt.
- The owner catalog carries `unstartable_reason`.

## Closure evidence

Booted against `~/.restless` — the exact configuration that previously bailed:

- plane listened; `aris` admitted with both `anthropic/*` failover candidates dropped;
  `exp12_attio_test` marked unstartable naming `credentials.model.inference.openai-codex`.
- owner API reported the reason on that company alone.
- `restless up -c exp12_attio_test` refused with the reason; `restless status -c aris` worked.
- `model_gateway::tests::one_companys_unroutable_model_does_not_stop_the_others` covers the invariant;
  185 daemon tests pass.
