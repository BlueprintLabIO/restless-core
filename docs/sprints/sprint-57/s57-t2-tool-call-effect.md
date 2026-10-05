# S57-T2 — Tool-call effect kind

**Layer:** Kernel. **Serves:** connected MCP writes had no receipt, no first-contact rule and no
protection against a blind retry after a lost response (sprint-57 "The problem").

`effect::request_tool_effect` records one durable intent per execution of
`(connection, tool, canonical arguments, parties, purpose)` and a receipt with the arguments as
sent. A server-reported error is a known failure. A lost response leaves the outcome unknown; only
`effect::settle_tool_effect` with a later read receipt on the same connection resolves it.

**Deletes:** nothing yet; T6 deletes the read-only profiles this generalises.

**Evidence:** `tool_gateway::tests::granted_tools_are_governed_by_class` (unknown, settle, retry,
replay, tampered key).
