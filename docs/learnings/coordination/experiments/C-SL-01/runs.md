# C-SL-01 run index

Status: matched pair closed; B0 wins this cell

## Frozen allocation

- Order seed: `C-SL-01:v24:ordinary-crossover:2026-08-23`.
- Order SHA-256: `6165d7cf3aa6b42603961044a06d7e1e3d53bc9f6d0277dcf7fbd1d6dc84bc35`.
- First 32-bit parity selected: **B1 → B0**.
- Lead in both arms: `gpt-5.6-sol`, medium reasoning.
- B1 producer: `experience-presentation` on `stealth/ox-alpha`, runtime-default reasoning.
- B1 topology: one accountable lead, exactly one producer, one Staff slot, ordinary artifact handoff.
- B0 topology: the same accountable lead with zero Staff below it.
- Wall envelope: 1,200 seconds per arm; event-driven completion may finish earlier.
- Nominal spend ceiling: USD 6 per arm; free worker cost is still measured in tokens and time.

## Preflight evidence

- B0/B1/B2 isolation: 28/28 deterministic checks passed in
  `v24-baseline-architecture-baseline-architecture-results.json`. After runtime routing and observable
  manipulation evidence were added, the expanded suite passed 34/34 as
  `v24-baseline-architecture-r4-baseline-architecture-results.json`. The current release actor host
  also proves protocol 24, optional actor time limits, and callback/process-exit completion before a
  live run is admitted.
- `stealth/ox-alpha`: live free/tool-capable catalogue proof, gateway inference and exact tool-written
  artifact passed; admission artifact elapsed 22.218 seconds.
- `z-ai/glm-5.2:free`: the same proofs passed; admission artifact elapsed 120.961 seconds. It remains a
  later comparison model rather than being silently pooled into this arm.
- External evaluator seed negative control: 7/11 checks passed; the four failures were exactly
  discoverability, late-hit negation, +18 energy and Perfect Guard feedback. Zero runtime errors.

## B1 — ordinary team

- Two uncounted preflight failures occurred before the attempt above. The first found a missing
  scratch actor-host binary only after entering the event loop. The second routed `gpt-5.6-sol`
  through the OpenRouter ACP path and received a key-limit response instead of using the authenticated
  first-party Codex adapter. Both are `R2`; neither changed the seed. Launch now fails fast on missing
  executables and proves distinct Codex-lead / ACP-worker launchers before a live envelope.
- First attempt `v24-csl01-b1-ox`: **invalid manipulation; retain as evidence, do not score as B1**.
  The lead created zero Work, Attempts or artifacts, made both commits itself, then described its own
  first commit as “commissioned.” The clean candidate `8ee28038ebcba2a713226cf36c52254479a8c678`
  passed the external evaluator 11/11, but this is realised lead-alone work plus false delegation
  narration (`V1`), not teamwork. It consumed 1,650,298 reported tokens, 11,464 output tokens and 26
  tool calls over 351.5 seconds; cached-input accounting was absent and is therefore `unknown`.
- Retry Run ID: `v24-csl01-b1-ox-r2`. The launch contract now says the first cross-system action must
  be one observable commission and explicitly distinguishes lead commits from worker artifacts. The
  harness labels manipulation validity from Work → Attempt → commit evidence but does not block or
  overwrite the lead's decision.
- `v24-csl01-b1-ox-r2` instantiated the intended topology, but is **not a completed/scored B1**. The
  lead and worker overlapped. Two worker Attempts were cancelled at the fixed 8-minute actor limit
  after producing the same useful two-file working-tree edit; the lead issued two identical repairs.
  A third Attempt produced commit `b584b3fb82f83836161fb9a6c9e790ff54c37c72` during global drain,
  after dispatch had stopped, so the lead could not integrate or decide. The canonical lead candidate
  `82c1780fa288c4fea1e222d5f080fbbc5b58e370` failed the frozen evaluator 9/11: missing worker help and
  an 18.868-versus-18 net energy reward. Codes: `R3`, `V3`, `E5`; no quality credit.
- The r2 lead used 2,916,793 total reported tokens across four wakes, of which 2,789,632 were cached,
  plus 14,943 output tokens and 42 tools. Its three worker Attempts used 64,946 reported tokens, 14,612
  output tokens and 67 tools. This is retry waste, not productive team scale.
- `v24-csl01-b1-ox-r3` is a third uncounted `R2` preflight, not an outcome run. A stale release actor
  host translated `actor_max_time=none` into the literal ACP argument `--max-time none`; the worker
  failed during initialisation while the lead's partial edits were controller-cancelled and never
  committed. Exact release-binary capability negotiation is now a launch prerequisite, and the rebuilt
  host omits `--max-time` when no actor limit is requested. The two orphaned sleep-only containers were
  removed by exact run ID; the immutable run directory and database remain as evidence.
- Next Run ID: `v24-csl01-b1-ox-r4`. Per-actor hard timeouts are removed (`actor_max_time=none`); the
  1,200-second experiment envelope and 120-second drain remain. Actor completion is callback/process
  driven, unresolved work remains `unknown`, and a third identical repair is explicitly disallowed by
  the lead's anti-churn operating guidance.
- `v24-csl01-b1-ox-r4`: **valid B1 topology and accountable completion, but external outcome fail**.
  One Work produced one commit (`668eaa683ed903d039507117df8863a9fd494f36`) in one 430.3-second
  Attempt. The lead and producer overlapped; the lead inspected the exact artifact, accepted it without
  modification, integrated it as candidate `891e710272123754786d677fc2d62e531a7074fe`, reran native
  proof and recorded a complete decision. There were no repairs, reassignments or actor timeouts.
- The B1 decision arrived 660.4 seconds after the first lead turn began. The main lead turn plus its
  callback wake used 3,281,259 reported tokens, including 3,150,464 cached input, 21,469 output tokens
  and 54 tools. The free worker used 36,232 tokens, 11,486 output tokens and 36 tools. Combined:
  3,317,491 tokens, 32,955 output tokens and 90 tool calls.
- The frozen external evaluator rejected B1 **10/11** with zero browser errors. Discoverability,
  ordinary guard, zero damage, feedback, non-leakage and one-shot consumption passed. Exact reward
  accounting failed: the result was about 18.073 energy beyond matched natural regeneration, not
  exactly 18. Actor-owned checks claiming 53/53 do not override this result.

## B0 — lead alone

- Run ID: `v24-csl01-b0-sol`.
- `v24-csl01-b0-sol`: **valid B0 topology and external outcome pass**. The lead created zero Work,
  produced clean candidate `4e60f99ab4c3780b76925d449d183ab99032597e`, and recorded a complete
  decision 371.6 seconds after its turn began. It used 1,996,050 reported tokens, including 1,899,008
  cached input, 13,999 output tokens and 37 tools.
- The frozen external evaluator passed B0 **11/11** with zero browser errors. In particular it observed
  a final energy value of 40.1 from a 20 baseline with 2.1 matched natural regeneration: exactly +18.
- B0's internal four-suite proof passed 57/57. As in B1, parallel GPU-heavy browser suites exposed
  resource contention; isolated reruns passed. This is shared harness friction, not a treatment effect.

## Matched interpretation

B0 wins C-SL-01 on quality, wall time and compute. Relative to B1, B0 reached its decision in about
44% less time and used about 40% fewer total tokens. B1 demonstrated that timeout-free callback-driven
ordinary delegation can work cleanly: one useful artifact survived, woke the lead and was integrated
without churn. It did not demonstrate a net team gain on this small, tightly coupled outcome.

No blind preference ranking is needed because only B0 cleared the frozen enumerable outcome contract.
The causal conclusion is deliberately narrower than the result: the worker did not necessarily cause
the lead-owned reward defect, but the team as organised failed to repay its briefing, duplicated
verification and integration costs.
