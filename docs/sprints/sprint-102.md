# Sprint 102: Decision models, small and large models — each turn on the cheapest mind that fits

Owner direction (2026-10-20): "agents in companies are as cheap as possible with appropriate tasks
going to decision agents, small LLMs and large LLMs and quality isn't hurt badly, small quality
regression is ok." Builds on [sprint 101](./sprint-101.md).

## Evidence

- **Short wakes are common and expensive.** On sydney_resale 44% of Exec wakes and 55% of lead wakes
  made three calls or fewer, each replaying a ~150k-token session. In the bakery runs several Exec
  wakes only restated a result already reported ("Everything is verified") or quiesced.
- **Every call ran on the largest model.** sydney_resale ran 100% of 10,059 calls on gpt-6-sol,
  including acknowledgements and coordination.
- **Decision models answer these questions correctly for almost nothing.** OpenRouter's
  `/api/alpha/decisions` serves 27 typed-question models (state + choice/yes-no/score questions →
  calibrated probabilities; input-priced, output free). On 14 hand-labelled wake events from the
  bakery runs and 28 done-judge checks against the produced site and mutated copies:

  | Model | Gate | Unsafe skips | Judge | $/M in | Median s |
  |---|---|---|---|---|---|
  | Jev 1.14 (TypeSafe) | 14/14 | 0 | 28/28 | 0.042 | 0.48 |
  | Perplexity Decider v1.2 | 14/14 | 0 | 28/28 | 0.020 | 1.01 |
  | GPT-6 Luna Decisions | 14/14 | 0 | 28/28 | 0.100 | 0.35 |
  | Cloudflare Clef | 14/14 | 0 | 28/28 | 0.042 | 0.61 |
  | Liquid d1, Mercury Decide, Drex 2.0, Microsoft Decision-1 | 11-12 | 1 | 25-27 | ≤0.06 | 0.7-1.1 |
  | DeepSeek V4.1 Flash (generative JSON control) | 14/14 | 0 | 28/28 | 3-25x | 3.29 |

  42 decisions rule models out; they do not rank the top four. A gate costs ≈ $0.00002 against a
  ~150k-token wake.

## The three minds

| Mind | Used for | Never used for |
|---|---|---|
| **Decision model** (`decision_model`) | Whether a successful result needs an Exec wake; whether a first Attempt is routine enough for the everyday model | Writing anything, effects, owner judgement |
| **Everyday model** (`everyday_model`) | Staff conversation and coordination turns (leads included); the first Attempt of Work the decision model rates routine | Exec turns, repair, review, Work with feedback |
| **Primary model** (`model`) | Exec, demanding Work, every repair and review Attempt, and failover behind the everyday model | — |

Decisions **fail open to intelligence**: no decision route, an error or low confidence means the
primary-model turn runs as before. A result skips its Exec wake only when the decision model gives
both "owner should hear" and "changes what happens next" below 0.3, the shadow threshold that saved
11 of 13 needless wakes on labelled results and missed none (a live routine backup scored 0.20 and
0.11, so a stricter 0.1 would never fire); Work starts on the everyday
model only at P(routine) ≥ 0.8. Owner messages, handoffs, mentions and failures are never gated.

A **cascade** keeps quality: the primary model stays behind the everyday model as failover, and
any second Attempt (after a failed gate, changes requested or feedback) runs on the primary.

## What was built

Reuses the existing `decisions.rs` client and its shadow record rather than a new gateway service.

| # | Change | Layer |
|---|---|---|
| 1 | `everyday_model` and `decision_model` in company config, settable with `restless company set`; the broker loads and pins the everyday model's provider. | Account/Runtime |
| 2 | `DecisionClient::for_company`: a company that names `decision_model` calls OpenRouter's decisions API directly with its OpenRouter inference credential; otherwise the hosted relay as before. `restless decide` uses the same route. | Account plane |
| 3 | The Exec result gate turns active for a company that names its own decision model: a confident double no is consumed in the scheduler pass and recorded `mode=active`; anything else stays shadow and wakes the Exec. | Runtime |
| 4 | Work tiering at claim time (`work_tier` decisions, recorded per Work). | Runtime |
| 5 | Staff conversation turns run on the everyday model at `low` effort (S101 item 5). | Runtime |

Deferred, with reasons:

- **Done judge** (per-requirement yes/no against the rendered artifact). The Runtime does not read
  produced artifacts today, the lead's native review already owns acceptance, and deterministic gates
  already run at Attempt end. Revisit when a run shows the lead accepting a candidate that misses a
  brief fact.
- **Small-model Exec turns.** Exec turns carry owner judgement; the gate removes the needless ones
  instead.
- **Doctrine router** (load only the prompt sections a turn needs): highest quality risk.

## Acceptance

- The active gate against OpenRouter with Jev: a routine result is consumed only on a confident
  double no; a deliverable the owner asked for always wakes the Exec
  (`tests/decisions_shadow.rs::an_active_gate_consumes_only_confident_routine_results`).
- Bakery scenario on a `_test` company with a free primary model, a free everyday model and Jev: the
  site still carries every brief fact; per-model call counts and token totals are reported.
- Decision route unset → behaviour identical to sprint 101.

## Checklist

- [x] 1 Config fields
- [x] 2 Decision route for a company
- [x] 3 Active Exec result gate
- [x] 4 Work tiering and cascade
- [x] 5 Everyday model for conversation turns
- [x] Live smoke on a `_test` company (see Result)

## Result (2026-10-20)

- Integration test against OpenRouter: a routine result was consumed (0.22 / 0.12, `active`) and
  the owner-asked deliverable woke Exec (0.92 / 0.74).
- Live bakery run: the site Work was rated 0.28 routine and the critic Work 0.16, so both correctly
  stayed on the primary model. A completed site result scored 0.83 / 0.73 and woke Exec. The
  everyday model took the lead's conversation turns; under a free-tier 429 it failed over to the
  primary as designed.
- The everyday tier carried 1 of 7 recorded turns (34k of 294k turn tokens) in this run. Exec and
  Work turns dominate cost, so the everyday tier only pays off where conversation turns are
  frequent.
