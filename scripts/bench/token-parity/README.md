# Token parity

Owner target (4 November 2026): Restless must use no more tokens than running the same task
directly in a single agent harness such as Claude Code or Codex. This benchmark measures the ratio
on five fixed tasks and is repeatable on any checkout. Since 8 November it also measures what the
tokens buy: time to the result, how soon the Exec answers the owner while its team works, and a
pass or fail quality check, each against a solo agent and an agent that uses sub-agents.

| Task | Owner message | What Restless does |
| --- | --- | --- |
| a | send one templated email | Exec sends it under the email mandate the owner granted before the run |
| b | write a one-page document from a brief, and review it | Exec commissions a lead, the lead's Work writes and reviews it, the result reaches Exec |
| c | what is in progress, and who is doing it? | Exec answers |
| d | have someone write a one-page document and bring it to me | Exec commissions a lead (`restless commission`), the lead's Work writes and checks it, the result wakes Exec, Exec tells the owner |
| e | have it drafted, then checked by someone else, before it comes to me | Exec commissions a lead and says it will check back; the lead's Work drafts it and commissions its check from a different agent (`--requires` and `--revises` the draft); the result reaches Exec |

## Run

```sh
scripts/bench/token-parity/bench.sh                     # this checkout, current prompts
scripts/bench/token-parity/bench.sh <older checkout> scripts/bench/token-parity/restless-before.json
BENCH_ROUTE=subscription BENCH_MODEL=anthropic/claude-opus-5-5 scripts/bench/token-parity/bench.sh
```

### Routes

- `BENCH_ROUTE=api` (default): the company uses an API key, served by the Messages relay.
- `BENCH_ROUTE=subscription`: the company uses a Claude sign-in (`omp-oauth:anthropic`), the way
  the owner's companies run. Requests go through OMP's host gateway on the pi-native wire. To set
  this up, `seed-sign-in.mjs` gives the stack's broker a scripted sign-in: a synthetic
  `sk-ant-oat01-` token that is sent only to the mock. The company is then switched to that
  sign-in. The baseline uses one-hour cache entries, as Claude Code does on a Pro/Max seat.
- `BENCH_MODEL` sets the company's model. `BENCH_COMPANY_SETTINGS` takes a list of `key=value`
  `company set` pairs, for example `everyday_model=anthropic/claude-sonnet-5-5`.
- `BENCH_LATENCY` sets the mock's model latency: `model` (the default), `off` (answers at once, as
  before 8 November), or rates such as `ttft_ms=600,prefill_tps=5000,read_tps=50000,decode_tps=50`.

Before the tasks, the runner grants the email mandate task a sends under, the way an owner does:
the Exec asks for it (`restless mandate propose`), the runner approves the owner's Inbox card and
waits out its Undo window. The mock stands in for Resend (`RESTLESS_TEST_RESEND_API_BASE`), so the
send and the observation after it really run and nothing reaches a real provider.

Each task also reports three breakdowns:

- **`timing.profile`**: where its time to result went. `model` is the modeled model time; every
  other part is a stretch with no model call running, named by what ended before it and what
  started after it (the owner message to the first call, a container command, a commission to its
  Work's start, a Work result to the Exec's wake, the reply stored after the last call).
  Overlapping calls count once, so the parts add up to the time to result. The report prints
  them as a fourth table.

- **`actors`**, from Restless's own spend records: calls, input tokens per call and models, for
  Exec, `chat:<actor>` and `work:<actor>`.
- **`payloads`**, from the mock's request log: bytes per call of system prompt, tool schemas and
  messages, wakes (calls that start a turn) and input tokens per wake.

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
  A recorded baseline needs Claude Code installed and an account that is not the owner's.
  `ANTHROPIC_BASE_URL` would point it at the mock, but the mock cannot authenticate Claude
  Code's own sign-in, so it would run on an API key. That key may be the mock's, for counting
  only. Put the recorded `usage` of the same tasks in place of `--baseline-system-tokens`.
- **Ratios** are Restless over baseline for calls, total input tokens, and cost-weighted input
  (uncached 1x, cache read 0.1x, cache write 1.25x). Parity means each ratio is at most 1.0.
- **Cost by model.** Every task also reports, for each side, the calls, weighted input and output
  tokens per model, priced at that model's models.dev rates (read at the start of a run; a
  4 November 2026 snapshot when offline), and the cost ratio. Routing a turn to a cheaper model
  shows here and not in the token ratios. The baseline runs on the company's model
  (`BENCH_MODEL`), as a direct harness on the same login would. On `BENCH_ROUTE=subscription`
  nothing is billed per token, so the same figure is the run's share of the plan, assuming the
  plan's usage windows weigh models by their API price; providers do not publish those weights.

## Tradeoffs: time, availability, quality, and a parallel baseline

The third table puts each task's tokens beside what they buy, for three sides.

- **Solo agent**: the baseline above. On team work (d, e) the one session writes and checks its own
  document.
- **Agent + sub-agents**: one agent that hands work to sub-agents the way Claude Code's Task tool
  does. The parent's Task call blocks until its sub-agents report, and several Task calls in one
  message run in parallel. Each sub-agent is a new session with its own fixed prefix: a sub-agent
  prompt and tool schemas, 12,000 tokens by default (`--subagent-system-tokens`), plus the same
  project context. Its brief is its first message, and its final text is the parent's tool result.
  - On d, one writer sub-agent writes and checks the document.
  - On e, a drafter and then a separate reviewer each run as a sub-agent. The check needs the
    draft, so they run one after the other. None of tasks a–e has independent parts to run in
    parallel, so this baseline runs nothing in parallel here. The code is there for tasks that do.
  - On a, b and c a direct harness has nothing to hand off, so they run as solo.
  - Both baselines keep separate prompt caches.

Columns:

- **Time to result**: from the owner's message to the Exec's last message to the owner in the
  task (the result or the answer). For a baseline, it runs to its final text. The quiet window the
  runner waits afterwards is not counted.
- **First owner reply**: the Exec's first message to the owner. On team work it is the
  acknowledgement, sent while the team works.
- **Exec free while working**: the gap between the first reply and the result, which is how long
  the owner had an Exec free to talk to. A direct harness is busy until it finishes: the owner's
  next message waits for the turn. So for both baselines the first reply is the result, and the
  free time is 0.
- **Quality**: a scripted rubric, the same for all three sides. It reads the commands that ran and
  their results, and the messages the owner got. For Restless these are the real results from the
  company's computer, read back from the mock's request log; for the baselines, their scripted
  results.
  - a: the email went to the customer and was accepted, and the owner was told.
  - b and d: the document has zones, fees and cut-off times; it was read back after it was written;
    and it reached the owner afterwards.
  - c: the owner got an answer that names the open work.
  - e: as d, plus the checker is not the drafter. The checker is the session that read the
    document back. For Restless, it is the owner of the check's own Work (`work add --owner`), or
    the drafter when the check ran inside the drafting Work. The scripted drafter gives the check
    to a different agent it adds (`restless people create`), as the owner asked.
  - The model is scripted, so this checks that the work reached the owner complete and by the route
    the owner asked for. It does not measure how well a model writes.

### What is simulated in the timings

- The mock answers in no time unless `--latency` is set. bench.sh sets `model`. Each answer is then
  held for a modeled time:
  - 600 ms to the first token;
  - uncached input and cache writes at 5,000 tokens/s;
  - cache reads at 50,000 tokens/s;
  - output at 50 tokens/s.
- Streamed pieces are spread over the decode time, and a scenario step's own `delay_ms` is ignored,
  so every caller is timed by the same rule.
- These rates are assumptions about a hosted Claude-class model, not measurements. Real latency
  varies with load, region and model, and a real model writes and explores more than the scripted
  one. So treat the times as Restless's own orchestration and wake overhead plus a modeled share of
  model time that is equal for all three sides. They are not a prediction of real wall clock.
- What is real on the Restless side: the scheduler, mail settle windows, wake-ups, container
  commands and every extra call a turn makes.
- The baselines' time is their modeled model time; a real harness adds its own tool time.
- `BENCH_LATENCY=off` reproduces the earlier runs. Times then show orchestration alone.
- Token counts with and without latency can differ a little on e. There, the Exec's own check-back
  races the team's work.
