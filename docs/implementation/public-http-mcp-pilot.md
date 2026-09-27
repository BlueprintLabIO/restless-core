# Public Streamable HTTP MCP read pilot

Core can broker a reviewed public Streamable HTTP read without an upstream
credential. This is a reusable profile boundary in `connected_tool.rs` and
`mcp_gateway.rs`: a profile pins the endpoint and exact tool allowlist, owns
its input validator, and uses Core's common MCP transport, Attempt grant,
upstream contract pin, revocation checks, bounded call, and receipts. Adding a
provider requires code review of that whole profile. Provider metadata or an
MCP `readOnlyHint` does not grant a tool.

The initial `deepwiki_structure_v1` profile permits only
`https://mcp.deepwiki.com/mcp`, `read_wiki_structure`, and one owner-selected
public `owner/repo`. Its sole argument must be `repoName` with exactly that
value. Core advertises the single value in the actor-visible schema and
enforces it again before invoking the upstream. The provider has no bearer,
the HTTP client refuses redirects, and Core rechecks the pinned version and
tool definition before each call. Calls have a 30 second timeout and a 128 KiB
decoded result limit; the current SSE response path also caps each event at
128 KiB. The bundled MCP client buffers a JSON-mode HTTP response before Core
applies the decoded limit, so a raw byte cap for that fallback is still needed
before claiming a general provider-safe transport. The entire provider result is untrusted data. The
profile makes no claim that DeepWiki's answer is true or current.

For an existing company, actor, and blocked Work, an owner can install the
profile with:

```sh
restless local-mcp -c COMPANY install-public-read \
  --profile deepwiki_structure_v1 \
  --name deepwiki \
  --endpoint https://mcp.deepwiki.com/mcp \
  --repository modelcontextprotocol/docs \
  --actor ACTOR --work WORK_UUID \
  --tool read_wiki_structure
```

Installation validates the repository shape, discovers and pins the live
upstream tool contract, and actually reads that one public repository before
enabling the connection. Only a fresh Attempt on the chosen Work receives the
MCP server. The owner can inspect the pin/status with `restless local-mcp -c
COMPANY list`, inspect the latest 100 append-only receipt events with
`restless local-mcp -c COMPANY receipts --name deepwiki`, and revoke the
connection with `restless local-mcp -c COMPANY disable --name deepwiki`.

Every accepted invocation writes a `started` event before upstream work and
then a `terminal` event sharing one `call_id`. The Core Authority row binds
company, actor, Work, Attempt, connection, tool, schema pin, policy revision,
request digest, result digest when observed, safe public subject, typed status, and elapsed
time. It stores no credentials or full result. A missing terminal event after
`started` is an explicitly unresolved call; a receipt write failure fails the
call rather than silently losing audit evidence. DeepWiki success is labelled
`response_observed_unverified`, since this tool returns repository wiki data,
not a verified real-world outcome. A tool error is `tool_error`, and an
interrupted or failed upstream call after invocation begins is
`outcome_unknown`.

This pilot does not migrate OAuth MCP or local stdio servers. It does not
authorize any provider writes. A production acceptance check still needs a
fresh actor Attempt invoking `read_wiki_structure` through Core and a matching
start/terminal receipt pair. A direct upstream probe alone is insufficient.
