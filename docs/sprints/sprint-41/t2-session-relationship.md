# S41-T2 — Implement the proved Claude session relationship

**Layer:** Runtime Bridge + OrgIntel  
**Serves:** Native takeover must say whether it continued private state or started from company context.

## Work

- Implement the strongest T0-proved path: exact resume, fork or explicit reconstruction.
- Validate company, harness build, actor, responsibility, Work, conversation/attempt, cwd and checkpoint
  before exposing any stored session.
- Prevent resumed history from becoming new canonical messages, effects or usage.
- For reconstruction, compile one bounded canonical context capsule and record the predecessor and
  reason exact continuation was unavailable.
- Handle stale/corrupt storage, version drift, cross-host state and abandoned prior control.

## Acceptance

Exact-resume copy appears only when the upstream session and prior turn are observed. Reconstruction
creates a new identity, preserves the company outcome context and never implies access to private
history it did not receive.

## Makes deletable

Session-id string equality as proof and hidden fresh-session fallbacks.
