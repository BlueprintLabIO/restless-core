# EXP-14 execution record

**Status:** Blocked on provider capacity; product and harness findings compiled

**Started:** 29 August 2026

**Sprint:** [Swift Arrival tight game-development loop](../../../exp-sprints/exp-sprint-14-swift-arrival-tight-loop.md)

## Frozen starting point

- EXP-11 candidate: `41f4fa53a2cd05ab17aea473f3d1be28979b2dcf`
- Known independent blocker: route 40/40 was reached, but a fresh source-blind player could not obtain
  visible delivery completion after exit, repositioning and unload attempts.
- Requested model: exact `glm-5.3-flash` on the coding route at the host named by `ZAI_BASE_URL`.
- Admission: HTTP 429, provider code `1310`, reset reported for 30 August 2026.
- Frozen fallback: exact `litellm/gpt-5.6-terra`; it independently exhausted its seven-day quota.
- A mixed Anthropic-base/Chat-Completions probe returned HTTP 200 but no completion. Admission now
  requires a valid provider response envelope, not a successful transport status.

## Isolation

- Company: `exp14_swift_arrival_tight_loop_test`
- State root: `/tmp/restless-exp14`
- Port offset: `16000`
- Aggregate company ceiling: USD 60
- No public deployment, live effect or promotion is authorised.

## Durable evidence policy

Raw captures, process state and scratch tools remain inside the disposable company. The repository
will retain only terminal reports, metrics, hashes and a small text evidence index. It must contain
zero EXP-14 PNGs after cleanup.

## Terminal result

The final candidate is `bd32f71967e91a67b3a28156c8fb287e52a6d51d`. All five mechanical gates
pass and one source-blind player completed the full journey on its immediate predecessor. A separate
blind negative exposed missing rejection feedback, which the one allowed product repair fixed.

The required two fresh journeys on the final candidate were not run because both exact GLM and GPT
routes exhausted their provider allowances. The experiment is therefore blocked, not accepted.

- [Results and decision](RESULTS.md)
- [Friction ledger](FRICTIONS.md)
- [Metrics](metrics.json)
- [Final candidate source snapshot](candidate/README.md)
