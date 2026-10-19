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
- [ ] Live smoke on a `_test` company
