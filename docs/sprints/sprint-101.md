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

## Result (2026-10-20, n=2 per arm: cheaper per run, not yet per delivered site)

Two `_test` companies per arm, run in parallel, same four bakery owner messages, each followed by an
Exec reply and 5 quiet minutes (150-minute cap per message). Both arms used primary
`openrouter/poolside/laguna-s-2.1`, effort `high`. Baseline is `af454281` (before 466bcdb4), with its
own daemon and company image. Treatment is `a64df149` with everyday `laguna-xs-2.1` and decisions
from `typesafe/jev-1.14`. Numbers come from `scripts/token-report`, which reads the owner spend
report.

| Run | Calls | Input | Cached | $ | Calls by actor | Site |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| baseline 1 | 839 | 44.6M | 88% | 1.21 | Exec 328, worker 315, lead 196 | built and revised; "five seeds" not stated |
| baseline 2 | 666 | 35.7M | 89% | 0.86 | Exec 249, worker 231, lead 186 | built and revised; "five seeds" not stated |
| treatment 1 | 458 | 18.1M | 90% | 0.42 | lead 211, Exec 173, critic 74 | every brief fact; name written "Crumb and Co" |
| treatment 2 | 656 | 28.9M | 90% | 0.82 | lead 398, Exec 258 | **none delivered** |

- **Per run, the treatment is cheaper:** on average 557 against 752 calls (-26%), 23.5M against
  40.1M input (-41%), and $0.62 against $1.04 (-40%). Input per call fell from about 53k to about
  42k. No site used corporate language or stock imagery.
- **Per delivered site, it is not cheaper yet:** treatment 2 never built the site. Its lead raised
  three successive owner handoffs of category `identity`: accept the Brand release, then pick one of
  three "first look" directions. That happened after the owner had said "go ahead, no need to check
  with me again", and it left three Works abandoned. Exec spent its remaining turns trying to clear
  those handoffs and never answered the weekday question. Counting that, it is $1.24 per delivered
  site against the baseline's $1.04.
- **The no-waiting rule held.** No Exec in either arm used `sleep`. Baseline Exec turns still ran
  for up to 57 minutes with up to 28 `work graph` reads, and two hit an expired turn capability.
  Treatment Exec turns ran for at most 28 minutes with at most 11 reads, and none expired. The
  remaining reads parse the graph or verify results rather than wait, so no deterministic guard was
  added.
- **The baseline's own waste** was the supervisor/worker loop. In baseline 1, the lead rejected the
  worker's revision Attempt over a leaked preview service; that one message consumed the whole
  150-minute cap.
- **Biggest remaining driver:** owner-taste checkpoints that ignore an explicit go-ahead.
  `align-and-learn` said customer-facing Work waits until a Brand release is proposed and never
  treats a proposal as approved. It and the `landing-page-design` skill also required a "first look"
  judgement with no exception. Fix: the Work waits only for the owner's answers, and an owner's
  go-ahead skips the first look. Verification run pending.
