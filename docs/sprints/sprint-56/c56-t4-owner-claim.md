# C56-T4 — Owner claim, Exec announcement, members projection

**Layer:** OrgIntel and Authority

**Friction served:**
- A local company could not turn on network entry without the owner losing their history: first
  entry always minted a `human-…` Actor.
- Exec never heard that a colleague had joined.

**Change:**
- `consume_human_access_context` binds the first `owner`-role principal to the existing `owner`
  Actor. It does this only if no principal ever held the owner role or that Actor.
- restlessd records `owner_actor_claimed` in Authority.
- A newly created human Actor produces one `daemon` → Exec message in the same transaction.
- `human_members()` projects the verified bindings.

**Makes deletable:** the self-hosted README's "does not convert an existing local-owner company"
caveat (deleted).
