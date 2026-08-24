# Sprint 16 — Research evidence and decision integrity

**Status:** Draft for founder alignment — starts after Sprint 15 exits

**Date:** 24 August 2026

**Depends on:** Sprint 15's trusted Runtime boundary. Sprint 12's connected desktop/mobile cockpit
review remains a separate release gate and is not silently closed here.

**Dogfood evidence:** [dogfood-1-after-action.md](../scenarios/dogfood-1-after-action.md)

**Spec refs:** ARCHITECTURE.md §2.1–§2.7, §4.4–§4.5, §5.2–§5.5, §6.2–§6.3,
§8–§9; evaluation-dogfood §9.6.1 and §18; owner-cockpit §1–§2; and ADR 0002.

**Salvage:** None. Reuse the existing Work, Attempt, artifact-reference, gate, owner-handoff,
Authority secret-ingress and Runtime-file paths. Each is revalidated by this sprint's observed run;
no legacy provider adapter, workflow engine, artifact-custody machine, or quant framework is lifted.

---

## Observed friction

Dogfood 1 produced a real, source-grounded and directional research dossier, but its run exposed four
specific product gaps:

1. The research Attempt produced at 03:02:09Z; the owner review did not appear until an owner message
   woke the Exec at 03:03:14Z, which then prepared the handoff at 03:07:32Z. Completion alone did
   not bring the prepared last mile to the owner.
2. The source bundle was useful but did not make freshness and route health a compact, inspectable
   fact. A later public-price re-probe returned HTTP 429; this must read as rate-limited/unknown, never
   as fresh, connected, or permanently failed.
3. The output made a pointed Buy/Watch/Avoid research call, but had no point-in-time historical test,
   benchmark, cost/liquidity assumptions or out-of-sample result. It cannot substantiate an alpha
   claim.
4. Public primary sources were sufficient for Run 1, so no provider signup was needed. The tighter
   emerging-company scope for the next run does need a testable, owner-prepared route to point-in-time
   market/reference data. No such route is yet connected.

These are observed product frictions, not a rationale for a market-data catalogue or quant platform.

## Founders' decision

We considered three directions before choosing one canon:

| Candidate | Result |
| --- | --- |
| Rerun broad public research and rely on the Exec to prepare the final review. | Rejected: it repeats the completion-to-attention failure and cannot distinguish fresh price evidence from a degraded route. |
| Build a generic provider registry, signal database, scorecards and backtest platform. | Rejected: no run has demonstrated that machinery; it would recreate the speculative generality the architecture forbids. |
| **One vertical evidence-to-decision loop for an emerging-company research outcome.** | **Chosen:** reuse the existing handoff path, make one source manifest an ordinary artifact, evaluate one declared alpha candidate in _test, and live-probe one owner-approved provider lane. |

## Outcome

An accountable research lead can finish a current, source-grounded equity research outcome and the
owner receives exactly one prepared native review without prompting for it. The review makes the
source/freshness state visible and ends in a direct **Speculative Buy**, **Watch**, or **Avoid**
research conclusion. In parallel, a separate _test company can reproduce a bounded historical test
of one declared alpha *candidate* — a result that may be rejected or inconclusive and is never
presented as live market evidence.

This sprint builds the evidence spine for one outcome. It does not build a full quant firm in the
product, nor does it turn a research judgment into a hard-coded universal stock score.

## Success contract

1. **Prepared outcome delivery.** For a Work that explicitly requires owner review and has a
   materialised, live-probed ReviewTarget plus its required gates, the existing OrgIntel path creates
   exactly one owner-judgement outcome-review handoff without an owner tell or manual Exec wake.
   A duplicate completion event, restart or review refresh does not create a second request. A missing
   target or failed gate remains honestly blocked; it is not replaced with a generic link or a claim of
   success. Nothing auto-accepts an outcome or acts on a market conclusion.
2. **Source facts are inspectable.** Each current research run emits one ordinary Runtime evidence
   manifest linked from its ReviewTarget. For each material source it records locator, source type,
   observation time, as-of time where applicable, claim supported, freshness expectation and observed
   retrieval/probe state. Unverified, unavailable, rate-limited and unknown remain distinct from a live
   authenticated observation. This is a per-run file, not a provider-state database or custody
   lifecycle.
3. **An alpha candidate is falsifiable.** A named _test company runs a small deterministic evaluator
   against a frozen pack of real historical inputs. Before execution it declares universe and
   point-in-time eligibility, hypothesis/signal, holding/rebalance horizon, benchmark, liquidity and
   transaction-cost assumptions, data cut-off, out-of-sample segment and rejection conditions. It
   reports raw and cost-adjusted results, known bias/data limitations and a verdict of supported,
   rejected or inconclusive for the candidate — never “alpha proved.” No simulated market facts enter
   the live research company.
4. **One real owner provider moment.** The company prepares one provider-hosted, read-only market-data
   handoff. The initial candidate is Polygon's U.S. Stocks API because its documented reference-ticker
   date parameter, daily aggregates and financial-source fields are relevant to the narrow evaluation
   path; exact product, price, licensing terms and permitted use are re-read in the handoff rather than
   assumed from this spec. The owner completes any signup, terms, MFA and account action in their normal
   browser. Any credential enters only through Authority/Infisical ingress, and a successful
   authenticated read-only probe — not an owner click — establishes the connection. If the owner
   declines or the probe fails, the state is explicit and Dogfood 2 either uses public-only evidence or
   blocks honestly.
5. **A harder real outcome.** Dogfood 2 runs a bounded emerging robotics/AI scale-up research job using
   live sources and the verified provider lane if available. It gives direct non-personal research
   direction, not generic information, and records the actual owner attention, cost, latency,
   source-health facts and limitations.

## Layer slices

| Concern | Authoritative layer | Sprint 16 change |
| --- | --- | --- |
| Review-required Work, exactly-once outcome handoff and duplicate repair | OrgIntel | Reuse source Work/Attempt/artifact/gate facts to create one existing handoff when the ReviewTarget is ready |
| Review presentation | Owner cockpit | Project the existing handoff's native target, evidence summary, exact judgment and resume condition; no new dashboard or trading UI |
| Research evidence, raw data pack and deterministic evaluator | Company Runtime | Ordinary scenario files, source manifests and a project-local _test evaluator/output |
| Provider credential and connection observation | Authority Plane | Use the existing owner external-browser handoff and Authority secret ingress; record only the resulting scoped connection/probe observation |
| Dogfood evidence and after-action | Evaluation | Freeze the alpha-candidate contract separately from live research; compare observed outcome/attention rather than a synthetic score |

## Problem classification

**Deterministic and enumerable:** whether a declared ReviewTarget/gate set is ready; deduplicating a
handoff; source probe outcome; preserving a frozen test-input manifest; calculations under a fixed,
declared alpha-candidate method.

**Judgment and open-ended:** what emerging-company universe is worth researching, the causal thesis,
the strongest counter-thesis, whether a provider's actual terms are worth accepting, and the final
Speculative Buy/Watch/Avoid conclusion. Models and the owner retain these decisions; a deterministic
evaluator does not pretend to make them.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| A system-generated trade, sizing, portfolio action or suitability conclusion slips into the outcome. | **Invariant** | Research output stays non-personal and directional only; no brokerage, execution, allocation, leverage or custody capability. |
| A provider-root session or raw credential reaches the Company Runtime. | **Invariant** | Owner browser outside Runtime; Authority secret ingress; Runtime receives only a bounded access path. |
| A provider click or configured secret is reported as a live connection. | **Invariant** | Only a successful authenticated read-only probe may say live; other states remain explicit. |
| The evaluator's historical fixture is mistaken for live market evidence. | **Invariant** | Run in a dedicated *_test company and label every test result; no test artifact is attached to the live company as factual market evidence. |
| The first candidate backtest is read as proof of alpha. | **Guarded** | Predeclare method, preserve limitations and use supported/rejected/inconclusive terminology only. Repeated independent out-of-sample evidence is required before the word alpha is used as a conclusion. |
| An automated review rule creates attention spam. | **Guarded** | Restrict it to Work that explicitly requires owner review, with one handoff keyed to the qualified outcome/revision. |
| A narrower emerging-company universe is too sparse or too illiquid for a meaningful historical comparison. | **Accepted for this run** | Report inconclusive and adjust the next scenario boundary; do not lower data standards or fabricate a result. |
| Source staleness persists between review and action. | **Accepted with visible treatment** | Preserve exact as-of time, expiry/review trigger and source-health state; owner independently verifies a current price before acting. |

## Non-goals

- a provider marketplace, registry, universal data adapter, OAuth/onboarding workflow or credential UI;
- a general signal language, factor-model library, universal score, portfolio optimiser, paper-trading
  system, execution connection, broker integration or investment-management product;
- synthetic stock prices, agent-authored market evidence or a test run in the live research company;
- recurring paid data, provider-plan acceptance or account creation by Restless; and
- a new owner dashboard, team/task administration surface or second source of truth.

## Tickets

Ticket status lives only in this checklist.

| Status | Ticket | Slice | Observed friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [ ] | [**S16-T0 · Freeze Run 1 evidence and the Dogfood 2 decision contract**](sprint-16/t00-freeze-contract.md) | Cross-layer + evaluation | Run facts, next-run assumptions and alpha limitations otherwise live only in conversation | Ad hoc rerun prompts and ambiguous “quant” aspiration |
| [ ] | [**S16-T1 · Deliver a qualified outcome review exactly once**](sprint-16/t01-automatic-outcome-handoff.md) | OrgIntel + owner cockpit | Dogfood 1 needed an owner message before its final review appeared | Manual final tell intervention and research-specific review glue |
| [ ] | [**S16-T2 · Make research source freshness a Runtime evidence artifact**](sprint-16/t02-research-evidence-manifest.md) | Runtime + owner projection | A later quote request hit HTTP 429 without a compact source-health surface | Unstructured, duplicated freshness narrative in review prose |
| [ ] | [**S16-T3 · Reproduce one alpha-candidate evaluation in a test company**](sprint-16/t03-alpha-candidate-evaluation.md) | Runtime + evaluation | A directional thesis has no historical evidence for an alpha inference | One-off manual return calculations and untestable “quant-like” claims |
| [ ] | [**S16-T4 · Prepare and verify one provider owner handoff**](sprint-16/t04-provider-owner-handoff.md) | Authority + Runtime + cockpit | The next scope needs point-in-time market/reference data; no live lane exists | Secret copy/paste, assumed connections and generic signup instructions |
| [ ] | [**S16-T5 · Run Dogfood 2, inspect, purge and report**](sprint-16/t05-dogfood-2.md) | Full vertical slice | The stronger outcome loop has not yet run end to end | Losing adapters, duplicate source paths and speculative evaluator affordances |

Existing code, a green evaluator or a polished report does not close a ticket. Each closes on its named
observed evidence.

## Exit evidence

Sprint 16 exits only with:

1. a no-owner-message scenario showing one qualified completed outcome creates one prepared owner
   review, and a restart/repeated completion check showing no duplicate handoff;
2. a review target with a linked per-run source manifest that visibly distinguishes a fresh public
   source, rate-limited/unavailable observation and unverified provider from a live authenticated
   observation;
3. a dedicated robotics_ai_alpha_test (or equivalent *_test) run whose frozen real-historical input
   pack, method, raw/cost-adjusted results, benchmark and out-of-sample conclusion can be inspected
   without entering the live company evidence base;
4. either a successful one-provider external-browser handoff plus Authority-owned credential ingress
   and authenticated read-only probe, or an honest declined/unverified outcome with no invented lane;
5. the Dogfood 2 native target and after-action, including an owner-review handoff created by the
   product rather than a manual prompt; and
6. only the focused verification relevant to touched code, followed by the documented Sprint 15 release
   checks and a real restless doctor -c <test company> probe. The connected visual gate remains
   separately honest until actually run.

## Entry, stop and exit gates

**Entry:** Sprint 15's capability boundary exits; Dogfood 1 remains historically intact with owner
acceptance pending; a founder confirms the proposed Dogfood 2 scope and accepts that the alpha result
is a candidate test rather than a claim.

**Stop:** pause for founder direction if the work requires a provider registry, a general quant
language/platform, a broker/execution capability, a provider-root session in Runtime, synthetic market
facts in the live company, or a recurring paid plan not covered by a specific owner decision.

**Exit:** the owner gets a prepared research outcome without prompting the company; the evidence says
what was actually observed; an alpha candidate is testable but not overstated; and Dogfood 2 produces
a harder, current decision with the system's actual limitations visible.
