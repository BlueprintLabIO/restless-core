# T3 spike report — ACP session client

**Date:** 2026-08-12 · **Agent probed:** `@agentclientprotocol/codex-acp` **v1.1.4** (npm, wrapping
`@openai/codex` 0.144.5) · **Model:** `anthropic/claude-sonnet-4` via OpenRouter (direct, no gateway yet)

## Live probe results (CLAUDE.md → "Probe, never guess")

| Step | Branch (a) extract | Branch (b) fresh |
|---|---|---|
| `initialize` | ✅ v1 negotiated | ✅ agent info `@agentclientprotocol/codex-acp v1.1.4` |
| `authenticate` | ✅ `api-key` method, key in `_meta` | ✅ same |
| `session/new` | ✅ | ✅ |
| model live-verify | ✅ `anthropic/claude-sonnet-4` offered + current | (not in minimal flow) |
| `session/prompt` + streamed output | ✅ EndTurn, 211 B captured, 2 tool updates | ✅ EndTurn, chunks + tool calls streamed |
| `session/cancel` | (not probed) | ✅ cancel after first tool call → `StopReason::Cancelled` |
| observable artifact | ✅ `hello-extract.txt` = `extract-ok` | ✅ `hello-acp.txt` = `acp-ok` |

Probe times: branch (b) end-to-end 21s (13:40:44→13:41:05 UTC); branch (a) same shape.
Codex config note: codex 0.144.5 **requires `wire_api = "responses"`** — `"chat"` is rejected at
session creation. Relevant for the T2 gateway route (it proxies `/v1/responses` only — matches).

## What the extraction actually yielded

Branch (a) lifted four self-contained helpers from `contained.rs`: `require_protocol_v1`,
`Capture`/`capture_notification`, `select_session_model`/`require_selected_model_option`.
Everything else in the legacy path — `AcpTunnel`, the WebSocket fence, the `DuplexStream` bridge,
`finish_bridge` lifecycle, `GovernedToolPermissionPolicy`, the `LiveFailure` phase taxonomy — exists
only to reach into a per-turn disposable container and has **no counterpart to extract onto** when the
transport is stdio pipes (6 lines with `tokio::process` + `ByteStreams`).

## Decision (frame 3: branch → run → purge)

**Canon: branch (b), the fresh client.** Both probes pass; the surviving extraction content is four
helpers that would be written identically fresh. Two of them are genuinely load-bearing and are
carried into the canon:

- `Capture`/`capture_notification` — turn output accumulation.
- `select_session_model`/`require_selected_model_option` — live-verifies the session model
  (the "probe, never guess" discipline made code).

**Purged:** branch (a) spike deleted in the follow-up commit; Git holds it. The legacy
fence/tunnel/permission-policy machinery is deleted *by absence* — it was never ported.

## Second fork: where the client runs

**(i) host `restlessd` spawns agents via `docker exec -i` and speaks JSON-RPC over that stdio.**
Chosen per the ticket's default: the probe's stdio semantics are identical for a local child and
`docker exec -i`; an in-container supervisor (ii) would be a whole extra component with no observed
need. Re-probe through `docker exec` happens in T4 against the real company image.

## Auth methods codex-acp 1.1.4 advertises (recorded for T2)

`api-key` (key via `_meta["api-key"].apiKey` or `CODEX_API_KEY`/`OPENAI_API_KEY` env), `chat-gpt`,
and **`gateway`**: `_meta.gateway = { baseUrl, headers, providerName }`, protocol `"openai"`. The
gateway method is how containers will authenticate through the T2 gateway with a purpose token —
the container never sees the provider key.
