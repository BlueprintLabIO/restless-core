# Dogfood 2 — Emerging robotics & AI scale-up alpha candidate

**Status:** Ready after Sprint 16 and one verified provider owner moment

**Version:** 0.1

**Type:** Live outcome dogfood paired with a separate historical evaluation in a _test company

**Operating phase:** Exploration

**Company shape:** Research

**Depends on:** [Sprint 16](../sprints/sprint-16.md) and the evidence recorded in
[Dogfood 1's after-action](./dogfood-1-after-action.md).

## Why this run exists

Dogfood 1 established that Restless can assemble a source-grounded, pointed robotics/AI research
dossier. It did not establish that the system can find or test an alpha candidate, preserve
point-in-time source facts, or automatically deliver the result to the owner.

Dogfood 2 deliberately makes the research problem harder: a small, emerging public-company universe
where valuation, cash runway, dilution, liquidity and listing mechanics can matter as much as the
technology story. The output still needs real direction. It is not allowed to retreat into a generic
industry report.

## What is being tested

There are two distinct runs, with different evidence status:

| Run | Company/world | Purpose | What it may conclude |
| --- | --- | --- | --- |
| **Current research outcome** | A live company using current issuer, official and verified provider sources | Give the owner a source-grounded, non-personal Speculative Buy/Watch/Avoid research conclusion today | A dated directional thesis with entry, invalidation and review conditions |
| **Historical alpha-candidate evaluation** | Dedicated robotics_ai_alpha_test (or equivalent *_test) company with a frozen pack of real historical inputs | Test whether one predeclared causal/signal hypothesis survived a bounded point-in-time evaluation | Supported, rejected or inconclusive for that candidate only |

The historical evaluation never supplies “facts” to the live company and never proves alpha. A positive
result is a reason for another independent out-of-sample test, not a license to assert investment
performance.

## Desired owner outcome

The owner receives one prepared native review target containing no more than six emerging robotics/AI
companies and can make an independent decision from it without coordinating research, finding sources
or handling a credential. Every surviving candidate ends in exactly one of:

- **Speculative Buy** — the research conclusion supports a purchase only at the stated current
  price/valuation or entry condition, with an explicit catalyst window and invalidation;
- **Watch** — the opportunity is plausible, but a named evidence, valuation or catalyst condition
  must resolve before a purchase is supported; or
- **Avoid** — the evidence, balance-sheet/liquidity risk or implied expectations do not support a
  purchase now.

The result is deliberately pointed but non-personal: it does not prescribe portfolio size, use the
owner's finances or holdings, recommend leverage, or execute any transaction. Any independent trade is
the owner's external decision and requires checking a current broker price first.

## Initial universe boundary

Freeze the following screen before selecting names or examining historical results:

- U.S.-listed operating common equities whose first public listing, direct listing or de-SPAC public
  listing occurred within the preceding seven years;
- material revenue, product or contracted economic exposure to robotics or AI, evidenced from issuer
  filings rather than an AI label alone;
- current market capitalisation between USD 250 million and USD 10 billion at the stated as-of time;
- 90-day median daily dollar volume of at least USD 2 million, measured from a source whose timestamp
  and market-session treatment are recorded;
- accessible English-language issuer and SEC filings, including cash, capital structure and dilution
  disclosures; and
- no more than six final candidates. A de-SPAC or reverse-merger history is eligible but must be
  prominent in its evidence card, not silently normalised away.

The boundaries are intentionally narrow enough to produce an owner-actionable result and a finite
historical cohort. If a real input makes the cohort too sparse, that is an explicit inconclusive result
or a founder decision for the next run — never an unnoticed relaxation after results are visible.

## Live evidence lanes

1. **Issuer and official lane:** SEC filings, earnings releases, investor presentations, proxy
   statements and Form 4/insider records when relevant. These support business, cash, dilution,
   concentration, lock-up and governance facts.
2. **Market/reference lane:** one authenticated, read-only provider connection if it is actually
   live-probed. The current candidate is [Polygon's U.S. Stocks API](https://polygon.io/docs/rest/stocks/overview?auth=signup),
   whose reference-ticker, daily aggregate and financial-source fields map to the frozen scope. The
   selection is provisional: current product, exact price, licensing and permitted-use terms are part of
   the owner handoff, not a fact assumed in this document.
3. **Public fallback lane:** inspectable public price/reference sources may supplement the result, but
   their route health and freshness must be explicit. A rate limit is not an inference about the price.

Every material fact appears in the per-run evidence manifest with a locator, source type, observation
time, as-of time, claim supported, freshness expectation and observed access state. A configured
credential, an owner click or an agent's statement is never a live data connection.

## Alpha-candidate contract

The research lead chooses one causal alpha *candidate* before evaluating it. It might concern the
interaction of fundamental quality, valuation reset, cash durability and post-listing risk; it must not
be a retrospective model score chosen because it worked.

Before the historical calculation runs, the test-world artifact must name:

1. the universe, inclusion/exclusion rules and data-availability timestamps needed to avoid look-ahead;
2. the causal hypothesis and exact signal/portfolio construction;
3. entry, rebalance and holding horizon;
4. benchmark and any appropriate risk/factor comparison;
5. liquidity floor, transaction cost, slippage and corporate-action handling;
6. train/validation/out-of-sample partition and the date each decision could have been made; and
7. rejection conditions, including what would make the output inconclusive.

The evaluator reports both raw and cost-adjusted returns, benchmark comparison and known data/bias
limitations. It must include survivorship and missing-delisting checks where the available data allows,
and must make an unknown data field visible rather than substituting zero.

## Prepared provider owner moment

| Moment | Exact owner action | Observable resume condition | If declined or unavailable |
| --- | --- | --- | --- |
| **Read-only market-data connection** | Open the exact provider-hosted Polygon flow in the owner's normal browser; inspect current plan, price, renewal and permitted-use terms; complete identity/MFA only if the owner chooses to proceed; put an issued API key only through the Authority secret ingress. | Authority performs a scoped authenticated read-only probe for the required reference, daily aggregate and financial-source data and records the response/freshness. | Mark the lane unverified or unavailable; use the public-only scope if it remains decision-useful, otherwise block the run honestly. |

Restless must not sign up, accept a plan, begin a paid trial, enter payment details, access a provider-root
browser session or ask the owner to paste a key into chat. Returning from the provider page is not
verification.

## Required review target

The final native dossier contains:

1. a one-page decision table with the dated universe, current source-health state and a Speculative
   Buy/Watch/Avoid call for every survivor;
2. a clear description of the company's actual robotics/AI economic exposure, not only its narrative;
3. the valuation and reverse-expectations view: what growth, margin, cash conversion or market share the
   current price appears to require, with the source and calculation shown;
4. cash runway, debt, expected dilution, capital-raise history, lock-up/insider selling or ownership
   facts, revenue quality/customer concentration and liquidity facts;
5. a primary thesis, serious counter-thesis, near-term catalysts and explicit disconfirming evidence;
6. entry condition, invalidation/exit condition and dated event or time review trigger;
7. the source evidence manifest and its unresolved conflicts, unavailable routes and freshness limits;
   and
8. a separate clearly labelled link to the historical alpha-candidate result, if it ran. That link says
   test-world result, never live market evidence.

## Acceptance criteria

A passing live run requires all of the following:

1. The native dossier exists at a concrete Runtime path or prepared browser target, and is linked to the
   actual Work/Attempt.
2. The final universe obeys the predeclared boundary or documents a founder-approved change before
   research conclusions are formed.
3. The artifact is current-dated and every material claim has traceable evidence or an explicit
   unknown/unavailable state.
4. The system creates exactly one owner outcome-review handoff automatically; the owner does not need
   to send a message just to obtain the prepared review.
5. Each survivor receives a direct Speculative Buy, Watch or Avoid stance with valuation, catalyst,
   counter-thesis and invalidation logic. A missing current price/valuation basis permits Watch or Avoid
   only.
6. The provider lane is called live only after the specified authenticated probe. Its absence never
   becomes a simulated substitute.
7. The paired alpha-candidate evaluator runs only in _test and reports supported, rejected or
   inconclusive with its limitations; it is not described as proof of alpha.
8. No brokerage, trade, sizing, leverage, custody, personal suitability or money movement is attempted.

## Failure and stop conditions

| Condition | Reading | Next action |
| --- | --- | --- |
| Provider signup is complete but the scoped probe fails. | Connection is unverified, not live. | Continue public-only if enough evidence remains; otherwise block and surface the exact gap. |
| The planned historical data pack cannot establish availability dates or an out-of-sample segment. | The alpha candidate cannot be evaluated honestly. | Report inconclusive; do not backfill a weaker test. |
| The cohort becomes too small after liquidity, filing and listing checks. | The scope is insufficient for the intended historical comparison. | Preserve the live research result if useful; ask founders to revise the next-run boundary before rerunning the test. |
| The report has good sources but no direct stance. | Restless avoided the core owner outcome. | Reject/revise the target rather than calling it a completed research result. |
| The report chooses a stance from a stale or rate-limited price route without making that state visible. | Source state has been laundered into fact. | Stop the relevant conclusion or downgrade to Watch/Avoid until a current source is observed. |
| A path requires a generic data platform, provider registry or broker integration. | Scope has outgrown observed need. | Stop and return to a new founder decision; do not smuggle the platform into the sprint. |

## After-action and progression

The after-action records actual source outcomes, data-lane state, owner minutes, model spend, elapsed
time, conclusion quality, historical-test verdict and the first point at which the owner could act.
It then chooses one smallest change or deletion. A second independent out-of-sample run is required
before any claim of durable alpha; Dogfood 2 itself is evidence about Restless's research-and-decision
loop, not about guaranteed investment returns.
