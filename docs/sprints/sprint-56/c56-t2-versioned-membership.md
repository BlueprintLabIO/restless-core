# C56-T2 — Versioned membership and durable controls

**Layer:** issuer

**Friction served:**
- Self-hosted handoffs always carried version 1, so a role change never reached Core.
- Suspension did not exist.
- A failed removal needed an owner to click **Retry removal**.

**Change:**
- `restless_membership_state` holds role, status and version for each membership. Every role
  change, suspension, reinstatement and removal advances the version in the same transaction as
  the change.
- Suspension and removal enqueue into `restless_membership_controls`, a durable outbox: stable
  `jti`, lease, backoff of 2/5/10/30/60 s, and a verified receipt.
- Entry is refused while a control is undelivered.
- On first start, any legacy `restless_membership_removals` rows migrate into the outbox.

**Makes deletable:** the synchronous `revoke()` path, the **Retry removal** state and the constant
versions (all deleted). In Cloud: `fleet_outbox` membership delivery.
