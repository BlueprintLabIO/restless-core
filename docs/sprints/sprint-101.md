# Sprint 101: Fewer calls per outcome, and a prefix the cache can share

Owner direction (2026-10-19/20): get as close to 10x cheaper as possible without hurting quality
badly; a small regression is acceptable. "Agreed on lower reasoning effort, shared cacheable prefix
and fail cheap … agree on lever 2." Builds on [sprint 100](./sprint-100.md).

## Evidence this sprint answers

Measured on the sprint-100 A/B (bakery scenario, free OpenRouter model) and sydney_resale's 30 days
(10,059 calls):

- **The OMP bash tool's service mode wasted 38-42% of all calls.** OMP 18.8.6 enables
  `launch.enabled` by default. A `name` or `timeout` argument either errors ("Service mode does not
  accept async or timeout") or backgrounds the command and returns only `running pid=…`, so the model
  redirects output to `/tmp` and reads it back. Baseline 258 calls: 18 errors, 41 outputless, 39
  follow-up reads. Treatment 438 calls: 42, 93 and 51.
- **Fresh sessions never share a cached prefix.** On sydney_resale a responsibility's first call is
  0% cached (median of 236; median input 18k), while a resumed wake's first call is 97% cached.
  Exec's operating-rules header differs from Staff's at character 60 ("…every actor, always]").
- **Orchestration is most of Exec's and leads' calls.** Exec's first bakery wake: 26 of 29 calls
  were `work add` (9), `teams` (7), `people` (4) and lookups. The lead's first wake: 42 calls, almost
  all status reads. `work graph` ran 23 times across the run.
- **Failures re-run whole wakes.** "Exec mixed its quiet decision with an owner reply" failed two
  treatment Exec wakes after they had done their work; the lead interrupted its own Attempt twice to
  "make the scheduler re-evaluate", blocking its own result.
- **One reasoning effort for everything.** `reasoning_effort` is company-wide (default `medium`).

## Changes

| # | Change | Layer | Deletes |
|---|---|---|---|
| 1 | `launch.enabled: false` in the OMP overlay. Long-lived services already run under the company supervisor (`company-tools` skill). | Runtime | The service-mode tool surface |
| 2 | One byte-identical operating-rules header for Exec and Staff; Staff's shared capability guidance moves ahead of actor-specific text. | Runtime | Exec's private header variant |
| 3 | A quiet decision mixed with an owner reply keeps the reply instead of failing the wake. | Runtime | One failure class and its retry wake |
| 4 | An actor cannot interrupt the Attempt it is running (refused with the reason). | Runtime | — |
| 5 | Reasoning effort by turn kind: `low` for conversation and coordination wakes, the company setting for Work and Exec. | Runtime | — |
| 6 | `restless commission`: appoint or reuse the lead, write the charter and add the lead-owned Work in one call. Composed client-side from the existing people, team and Work commands, so no new daemon protocol. | CLI | The 4-6 call orchestration ritual |
| 7 | A lead's context carries its team's current Work and each one's latest Attempt result. | Runtime | Repeated `work graph`/`teams` lookups |
| 8 | Team appointment text and Exec doctrine match lead-direct production; Exec asks the owner before appointing. | OrgIntel | Stale "commission Staff" text |
| 9 | Cache-expiry retirement: a session idle past the prompt-cache TTL (60 min, measured) above 32k starts fresh. | Runtime | — |

Gates already run deterministically when an Attempt ends (`staff/gates.rs`); no new gate machinery.
A dedicated preview tool was considered and dropped: the browser and `company-tools` skill cover it
once bash returns output.

## Acceptance

- The bakery scenario on a `_test` company: no service-mode errors or outputless bash calls; total
  calls and input tokens fall against the sprint-100 baseline arm, and the produced site carries every
  brief fact.
- Exec's first production wake commissions in one `restless commission` call.
- Unit tests for the self-interrupt refusal and the cache-expiry rule; full engine and OrgIntel suites
  pass.

## Checklist

- [x] 1 Bash service mode off
- [x] 2 Shared prefix
- [x] 3 Quiet+reply keeps the reply
- [x] 4 Self-interrupt refused
- [x] 5 Effort by turn kind
- [x] 6 `restless commission`
- [x] 7 Snapshot carries status
- [x] 8 Lead-direct text and ask-before-appoint
- [x] 9 Cache-expiry retirement
- [x] Live smoke on a `_test` company (see Result)

## Result (2026-10-20, one paid bakery run, not a win yet)

Run on `tokpaid_test`: primary `openrouter/poolside/laguna-s-2.1`, everyday
`laguna-xs-2.1`, decisions from `typesafe/jev-1.14`, effort `high` as in the baseline arm. The
free-tier run before it was abandoned: `:free` models share one 20 req/min cap per key, and a
concurrent session on the same key hit it too.

- Working as built: Exec commissioned in one `restless commission` call (third try, after two id
  errors); bash returned output and needed no service-mode read-backs; the produced site carried
  every brief fact, used no corporate language, and passed an independent critic apart from two small
  defects.
- **Not cheaper:** the run had reached 467 calls / 20.6M input (84% cached, $0.63) when stopped
  during the critic-driven revision, against the baseline's 266 calls / 10.8M for the whole scenario.
  Where it went: one Exec wake made 131 calls over 45 minutes, polling with `sleep` + `work graph`
  until its turn capability expired. Exec also appointed before asking (so the first Work was
  abandoned once the owner answered), and it added a critic review the baseline never ran. These
  are behaviours of this model on this path, and n=1.
- Fixed from the run: the shared rules now forbid waiting inside a turn; Exec never posts a
  streamed provider error or the gateway's own refusal as an owner reply (both happened).
- Next: rerun both arms, n≥2 each, with the no-waiting rule, before claiming a saving.

## Result (2026-10-20, n=2 baseline, n=4 treatment: cheaper per run, dearer per delivered site)

Same four bakery owner messages on every `_test` company, each followed by an Exec reply and 5 quiet
minutes (150-minute cap per message). Every run used primary `openrouter/poolside/laguna-s-2.1`,
effort `high`. Baseline is `af454281` (before 466bcdb4), with its own daemon and company image.
Treatment is `a64df149` with everyday `laguna-xs-2.1` and decisions from `typesafe/jev-1.14`.
Treatment 3 and 4 also carry the owner-go-ahead wording below. Numbers come from
`scripts/token-report`, which reads the owner spend report.

| Run | Calls | Input | Cached | $ | Calls by actor | Outcome |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| baseline 1 | 839 | 44.6M | 88% | 1.21 | Exec 328, worker 315, lead 196 | site built and revised; "five seeds" not stated |
| baseline 2 | 666 | 35.7M | 89% | 0.86 | Exec 249, worker 231, lead 186 | site built and revised; "five seeds" not stated |
| treatment 1 | 458 | 18.1M | 90% | 0.42 | lead 211, Exec 173, critic 74 | every brief fact; name written "Crumb and Co" |
| treatment 2 | 656 | 28.9M | 90% | 0.82 | lead 398, Exec 258 | **no site** |
| treatment 3 | 457 | 18.8M | 90% | 0.52 | lead 275, Exec 182 | every brief fact, both revisions live |
| treatment 4 | 754 | 35.3M | 88% | 1.03 | lead 530, Exec 224 | site built; revision reply never reached the owner |

- **Per run, the treatment is cheaper:** on average 581 against 752 calls (-23%), 25.3M against
  40.1M input (-37%), and $0.70 against $1.04 (-33%). Input per call fell from about 53k to about
  43k. No site used corporate language or stock imagery.
- **Per delivered site, it is dearer:** 2 of 4 treatment runs delivered, against 2 of 2 in the
  baseline. That makes $1.40 per delivered site against $1.04.
- **No Exec used `sleep`** in either arm. Long turns remain, and none of them were waiting: Exec
  re-parses `work graph` while it struggles with a handoff. Baseline turns ran up to 57 minutes with
  up to 28 reads; treatment 4 had one of 71 minutes with 28 reads. A guard on unchanged reads would
  not have ended those turns, so none was added.
- **Driver 1: owner answers in chat cannot close an owner handoff.** Leads open `owner_judgement` or
  `identity` handoffs (the Brand release, a "first look", `--owner-review`). The owner answers in the
  conversation ("go ahead", "looks good, two changes"), but Exec has no way to record that answer
  against the handoff. It tries resolving `--as owner` (refused), resume, interrupt and re-adding
  Work. Treatment 2 never built its site this way; treatment 4 abandoned 9 Works.
- **Driver 2: an Exec turn longer than its 45-minute capability loses its reply.** Twice in
  treatment 4, Exec finished 100+ tool calls, and the completion prompt then returned
  `invalid model capability: capability expired`. The turn was marked blocked and nothing reached
  the owner. The baseline posted that same error text as its reply. Work Attempts already get 12
  hours (`capability.rs`); Exec conversation turns still get 45 minutes.
- **The baseline's own waste** was the supervisor/worker loop. In baseline 1, the lead rejected the
  worker's revision Attempt over a leaked preview service; that one message consumed the whole
  150-minute cap.
- **Owner-go-ahead wording** (`align-and-learn`, `landing-page-design`): customer-facing Work now
  waits only for the owner's answers, not for the Brand release, and a go-ahead skips the first
  look. Neither gate recurred in treatment 3 or 4, against 1 of 2 runs before. That is too few runs
  to count as proof; drivers 1 and 2 still stopped treatment 4.

## Result (2026-10-20, drivers 1 and 2 fixed: cheaper per delivered site)

- **Driver 1:** `restless work resolve-handoff --source-message <id> --as exec` records the owner's
  conversation answer against a handoff owed to them. The cited message must be the owner's to Exec;
  the owner's words are kept verbatim. An outcome review takes `resolved` (accept) or
  `request_changes` without a card brief; any other step resolves or declines. A payment stays
  provider-observed, and a handoff owed to another person is refused. The answer settles a handoff
  still sitting with an agent below the owner. Covered by `outcome_review.rs`.
- **Driver 2:** a conversation turn's capability lasts 3 hours instead of 45 minutes.

Same bakery scenario, same models as treatment 3-4, two `_test` companies on this commit:

| Run | Calls | Input | Cached | $ | Outcome |
| --- | ---: | ---: | ---: | ---: | --- |
| fix 1 | 409 | 20.5M | 93% | 0.44 | every brief fact incl. "five seeds"; both revisions live; weekday answer given |
| fix 2 | 586 | 24.0M | 93% | 0.68 | every brief fact incl. "five seeds"; both revisions live; weekday answer given |

- 2 of 2 delivered, so **$0.56 per delivered site against the baseline's $1.04 (-46%)**, with 34%
  fewer calls (498 against 752) and 44% less input (22.3M against 40.1M). Both runs finished inside
  90 minutes; the baseline took up to 3h50. No site used corporate language or stock imagery.
- Exec recorded the owner's answer with `--source-message` once in each run: fix 2 sent "Looks good.
  Two changes" back as `request_changes` on the review, fix 1 closed the brand-direction judgement.
  No "not currently owed" refusals, against 300 in treatment 4.
- **The everyday model cost more than the primary here.** OpenRouter prices cached input at
  $0.009/M for laguna-s and $0.03/M for laguna-xs, and 93% of input is cached, so xs costs about
  twice as much per token. Fix 2 sent 373 lead conversation calls to xs for $0.50; at s rates they
  would have cost about $0.23. Set `everyday_model` only to a model whose cached-input price is below
  the primary's; with laguna-s as primary, leave it unset.
- **DeepSeek V4.1 Flash is not cheaper either** (one run each, 20 October). Its catalogue price is
  the lowest tool-capable one, but OpenRouter spreads it over ~35 providers with cached-input rates
  from $0.0016/M to $0.033/M. Primary deepseek-v4.1-flash alone delivered the scenario for $0.71
  (459 calls, $0.038/M input); laguna-s with deepseek as everyday model had spent $0.50 (426 calls)
  when stopped. laguna-s alone measured $0.021/M. The cheapest measured configuration is laguna-s
  with no everyday model and Jev for decisions: $0.56 per delivered site against $1.04 (-46%).
