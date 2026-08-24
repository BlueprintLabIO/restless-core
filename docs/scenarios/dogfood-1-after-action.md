# Dogfood 1 — Robotics & AI public-equity research: Run 1 after-action

**Status:** Evidence recorded; owner outcome acceptance is still pending

**Scenario:** [`dogfood-1.md`](./dogfood-1.md) v0.2
**Company:** `robotics_ai_dogfood`
**Run date:** 24 August 2026
**Run mode:** live public-source research; no simulated market capability

## Result

Restless produced a source-grounded, action-directed eight-company robotics and AI thesis at
`/company/outputs/dogfood1-robotics-ai-equity-thesis/review-target.md`. It made a clear,
non-personal research call — Buy NVIDIA and Intuitive Surgical; Watch Tesla, Symbotic, Teradyne,
Rockwell and Deere; Avoid Palantir — with entry conditions, counter-theses, invalidations and review
triggers. It did not place a trade, open an account, seek a credential, or claim alpha.

The outcome is not accepted yet. The prepared owner review remains pending as handoff
`98bb492a-5ef4-4ab9-ab41-975f32214b42`; no later discussion should be mistaken for accepting or
rejecting that Work.

## Observed run record

| Fact | Observed evidence |
| --- | --- |
| Accountable Work | `2fbfd5cc-782a-4b61-8486-f3c5fcbe3fe1` — status `blocked`, awaiting owner judgment |
| Attempt | `728d074b-5c7c-475b-b02a-723927a216e0`, owned by `research-analyst`, model `zai/glm-5.3` |
| Attempt timing | Started `2026-08-24T02:25:22.969730Z`; produced `2026-08-24T03:02:09.895269Z` |
| Native target | Artifact `846eeefd-8381-4b61-a865-1c9098344f8d` at the Runtime path above, created `2026-08-24T03:01:14.360081Z` |
| Objective completion checks | `review-target-nonempty` and `raw-research-present` both passed at attempt completion |
| Owner review | `owner_judgement` handoff created `03:06:49Z`, brief prepared `03:07:32Z`, still pending when inspected |
| Source bundle | 114 cached raw files; issuer/SEC filings plus a dated public-price lane; no authenticated data lane was claimed |
| Model/spend envelope | GLM-5.3 only, $25.00 ceiling; $1.4897 accounted when inspected, $23.5103 remaining; no spend poison |
| Browser/runtime probe | Persistent browser, Chromium, automation, desktop and web transport were all `available`; controller `unclaimed` |

The report is anchored to the 21 August 2026 public close for prices and 24 August 2026 filing
retrieval. That makes it reviewable, but not a continuously fresh price feed.

## What worked

- The Exec appointed one accountable research lead and the lead completed the coupled research work
  solo. No fictional Staff contribution or simulated data source entered the evidence.
- The output met the first-run decision-usefulness bar: it names directional calls rather than ending
  in generic sector commentary, and each Buy had a dated price/valuation basis plus entry and
  invalidation conditions.
- Primary-source evidence, raw research and a native review target were preserved as ordinary Runtime
  files and linked to the actual Attempt. The two completion gates proved the target and raw evidence
  existed; they did not pretend to prove the investment thesis.
- The public-primary lane was sufficient for this bounded first thesis, so Restless correctly did
  not manufacture a provider signup merely to exercise an owner moment.

## What failed to prove

- The thesis is not a claim of alpha. It has no frozen point-in-time historical evaluation, benchmark
  comparison, transaction-cost assumption, factor exposure check, estimate-revision lane or
  out-of-sample result.
- A later bounded public quote re-probe returned an HTTP 429 response. That is evidence of a
  rate-limited/freshness-degraded route, not evidence that the route is connected or permanently
  failed. The Run 1 artifact must remain anchored to its recorded as-of time.
- Completion did not itself create owner attention. The attempt ended at `03:02:09Z`; the next Exec
  wake at `03:03:14Z` was explicitly caused by an owner message, and the owner brief appeared only at
  `03:07:32Z`. The event stream contains no automatic completion-to-review transition.
- The owner has not accepted, rejected, or requested a revision. Review usefulness and any later
  return observation therefore remain unknown.

## Owner moments

| Moment | What happened | Reading |
| --- | --- | --- |
| Scope | The existing directive bounded the research; no further scope action was needed. | Good: ordinary work stayed with the company. |
| Data provider | No provider handoff was requested because public sources answered the first-run question. | Correct restraint, not an untested live connection. |
| Final outcome review | A prepared review exists with target, gates, requested judgment and resume condition. It required an explicit owner message to be created and remains unresolved. | Product friction: the owner should not have to prompt the last-mile preparation. |

## Observed-friction decisions

| Friction | Disposition | Smallest response |
| --- | --- | --- |
| A produced research Attempt did not autonomously yield its final owner review. | **Pending fix** | Reuse the existing Work, artifact, gate and handoff path to create exactly one outcome-review handoff once a qualified ReviewTarget is ready. |
| Source notes make price freshness and transport degradation difficult to compare at a glance. | **Pending fix** | Add one ordinary per-run evidence manifest with locator, as-of/observed time, claim, source type and observed route state. |
| A pointed thesis can sound like an alpha claim without a falsifiable historical evaluation. | **Guarded** | Specify one small deterministic alpha-candidate evaluation in a dedicated `_test` company; it may report supported, rejected or inconclusive — never “proved alpha.” |
| A future emerging-company run needs higher-quality point-in-time price/reference data. | **Pending owner moment** | Prepare one specific provider-hosted owner handoff and require Authority ingress plus an authenticated read-only probe. No provider registry or assumed connection. |
| The report could cause a reader to infer portfolio advice or automated execution. | **Invariant** | Continue to prohibit brokerage connection, orders, sizing, leverage, custody and personal suitability. |

## Next informative run

The next change is a focused implementation sprint, not a generic quant platform: make research
evidence-to-owner delivery reliable, distinguish fresh/unavailable source observations, and make one
historical alpha-candidate experiment reproducible in `_test`. Then run Dogfood 2 on a tighter,
higher-upside emerging robotics/AI universe using a genuinely verified provider lane if the owner
chooses to connect it.

See [`../sprints/sprint-16.md`](../sprints/sprint-16.md) and
[`dogfood-2.md`](./dogfood-2.md).
