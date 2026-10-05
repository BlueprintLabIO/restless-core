# S57-T3 — Connection grant record

**Layer:** Kernel. **Serves:** owners could not say which tools an actor may use or how each is
governed.

`restless_authority.connection_grants` holds one live grant per (connection, grantee) with each
tool's class, party arguments and contract digest; grant, revoke, freeze and disconnect are
Authority records. `reserved` calls emit an `approval_required` with a `call_key`; the owner's
`tool_call_approved` or `tool_call_declined` binds that exact command digest, and the decision is
announced to Exec like a party decision.

**Evidence:** the same test (actor scoping, freeze, reserved approval in Attention, contract change).
