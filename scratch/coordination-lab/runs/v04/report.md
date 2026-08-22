# v04 — ACP-supplied MCP tools through Pi

## Change under test

Attach a stdio MCP server supplied in `session/new`, validate its name/command/arguments/environment
names against the hashed Restless launch contract, discover its tools, and expose those tools to Pi.
The harness remains an adapter; it does not implement the coordination command.

## Evidence

- Model: `cohere/north-mini-code:free`
- Live prices: prompt `0`, completion `0`
- MCP server: local `restless-coordination-fixture`
- Discovered tool: `coord_probe`
- Requested argument: `work-lumaara-7`
- Returned and repeated exactly: `WORK-CALLBACK:work-lumaara-7:MCP-OK`
- ACP lifecycle: thought chunks -> MCP tool call -> tool completion -> answer chunks -> `end_turn`
- Turns: 2; usage: 357 input / 274 output; cost $0
- Unknown, missing, extra, or command/argument/environment-mismatched servers are rejected before the
  model receives their tools.

## Score

Harness-only score: **95/100**.

| Harness criterion | Points |
| --- | ---: |
| Exact launch controls including MCP identity | 20/20 |
| Chronological live MCP tool/text/thought streaming | 20/20 |
| Tool discovery and result conversion | 20/20 |
| Secret values excluded from launch/event metadata | 20/20 |
| Transport coverage | 15/20 — stdio only; HTTP/SSE should wait for a real provider need |

## Decision

Retain stdio MCP as the thin structured-service adapter. Do not move scheduling or company policy
into MCP. HTTP/SSE are accepted gaps until a live integration requires them.
