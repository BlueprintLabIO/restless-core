# Token parity

Owner target (4 November 2026): Restless must use no more tokens than running the same task
directly in a single agent harness such as Claude Code or Codex. This benchmark measures the ratio
on three fixed tasks and is repeatable on any checkout.

| Task | Owner message | What Restless does |
| --- | --- | --- |
| a | send one templated email | Exec sends it under the mandate |
| b | write a one-page document from a brief, and review it | Exec commissions a lead, the lead's Work writes and reviews it, the result reaches Exec |
| c | what is in progress, and who is doing it? | Exec answers |

## Run

```sh
scripts/bench/token-parity/bench.sh                     # this checkout, current prompts
scripts/bench/token-parity/bench.sh <older checkout> scripts/bench/token-parity/restless-before.json
```

`bench.sh` starts `tools/mock-model` (which counts calls and tokens), brings the checkout's
`restless-dev` stack up on it with a `tokparity_test` company, runs `run.mjs`, and destroys the
company and stops every process it started. Results are JSON plus a Markdown table.

After every merge to main, `scripts/release/local/post-merge-gates.sh` runs it through
`scripts/release/local/token-parity.sh`, appends the ratios to
`~/.local/state/restless-release/token-parity/history.jsonl` and posts them as the `token-parity`
commit status. It never blocks a release.

## Method: what is measured and what is scripted

- **Scripted:** inside each model turn, the mock takes the fewest tool calls the actor's prompt
  tells it to (`restless.json`; `restless-before.json` scripts the procedures the earlier prompts
  prescribed). It does not judge, so the numbers do not show a real model's extra exploration.
- **Measured:** everything Restless itself decides: which actors wake and how often, each turn's
  extra end-of-turn call, and every request's tokens (system prompt, tools, history, tool output),
  with Anthropic's caching rules (a `cache_control` breakpoint caches its prefix for 5 minutes, or
  1 hour with `ttl: "1h"`; a byte-identical prefix within its TTL is a cache read).
- **Tokens** are estimated at 4 characters per token, the same way for both sides.
- **The baseline** is one agent session doing the same task with the same scripted moves: a fixed
  prefix the size of Claude Code's system prompt and tool schemas (16,000 tokens by default,
  `--baseline-system-tokens`) plus 1,500 tokens of project context, cached for 5 minutes, and the
  conversation tail cached as Claude Code does. Neither `claude` nor `codex` is installed on the
  benchmark host, so this is an estimate of the direct harness, not a recording of it; the size is
  a parameter so a recorded `claude -p --output-format json` usage can replace it.
- **Ratios** are Restless over baseline for calls, total input tokens, and cost-weighted input
  (uncached 1x, cache read 0.1x, cache write 1.25x). Parity means each ratio is at most 1.0.
