# Dogfood 2 — public-only emerging robotics & AI after-action

**Status:** Delivered to owner review; owner judgement pending

**Scenario:** [dogfood-2.md](./dogfood-2.md) v0.1

**Live company:** `robotics_ai_dogfood2_recovery`

**Test company:** `robotics_ai_alpha_test`
**Run date:** 24 August 2026

## Result

Restless delivered a current, public-source dossier and automatically created one pending
`owner_judgement` handoff. Its only direct positive research conclusion is:

> **PONY — Speculative Buy only when its Nasdaq ADS close is at or below $8.50 and the latest
> quarterly issuer liquidity remains at or above $1.2bn; otherwise do not initiate. Revisit after
> Q3-2026 results on 20 November 2026.**

SERV, WRD, UMAC, RR and PDYN are **Watch**, not purchase conclusions. RR is explicitly **do not
purchase before its restated financials exist**. This is non-personal research direction, not
allocation, suitability, leverage, custody or execution advice. The price observation behind the PONY
condition was the 21 August 2026 close, observed on 24 August; the owner must check a live broker
price and the latest issuer filing independently before any external action.

The paired historical evaluator ran only in `robotics_ai_alpha_test` and returned **inconclusive**.
It is not attached to, or used as evidence for, the live market conclusion.

## Observed evidence

| Fact | Observed evidence |
| --- | --- |
| Native target | `/company/outputs/dogfood2-public-recovery/review-target.md`, prepared by Mira (`research-analyst`) on attempt `582f946c-8efc-41b7-9d6d-e7122867315d` |
| Frozen screen | U.S.-listed no earlier than 24 August 2019; issuer-evidenced physical robotics/autonomy exposure; $250m–$10bn market cap; 90 completed-session median daily dollar volume at least $2m; English issuer/SEC evidence; at most six survivors |
| Final universe | PONY, SERV, WRD, UMAC, RR and PDYN. ZENA was excluded for its $157.49m observed market cap; the $250m floor was not relaxed. |
| Liquidity route | Yahoo public regular-session bars, captured 24 August at 08:30Z, covering the 90 completed sessions from 15 April through 21 August 2026. Raw responses, calculation script and summary sit beside the target. |
| Source manifest | `/company/outputs/dogfood2-public-recovery/source-evidence-manifest.json`: 35 material-source records; its validator passed with exactly `available_public` and `unverified_provider` states. Raw SEC/issuer and public-market responses are retained under `raw/`. |
| Provider state | The original provider handoff remains unverified. This run made no signup, terms acceptance, credential request, authenticated probe or second provider handoff. |
| Owner attention | Handoff `a55fb53c-f5fc-4c54-9a2f-2581c9759e6b` was created at `2026-08-24T08:54:15Z` without an owner message. Its prepared owner brief then surfaced the conditional PONY call, five Watch calls, corrected-screen history, 35-source evidence and the exact accept/request-changes choice. The Work is blocked on that judgement and was not auto-accepted. |
| Runtime probe | `restless doctor -c robotics_ai_dogfood2_recovery` observed coordinator, OrgIntel, owner APIs, persistent Runtime and browser transport available. |

The current thesis uses issuer-reported operating and balance-sheet facts, not a synthetic test
result. Pony.ai’s official Q2 release reports $36.2m total revenue, $12.1m robotaxi-services revenue,
a 1,975-vehicle robotaxi fleet and $1.3905bn of stated liquidity at 30 June 2026. Growth and fleet
targets remain forward-looking claims, so the dossier sets price, liquidity, revenue-growth and fleet
invalidations rather than treating them as achieved facts.

## Correction path, attention and cost

| Time / attempt | What happened | Result |
| --- | --- | --- |
| 07:57:08Z — attempt 1 | The lead used a relaxed $50m market-cap floor and a 20-day volume average; its SERV card also carried copied PONY facts and a stale securities figure. | Owner feedback arrived before review. The terminal report was marked stale/superseded, with **zero** handoffs created. |
| 08:27:20Z — attempt 2 | The frozen screen and card-by-card source requirement were in the Work revision before launch. | Corrected 90-session screen, raw evidence, source manifest and native dossier were produced. All three declared gates passed. |
| 08:54:15Z | The qualified produced Attempt created the outcome-review handoff. | Exactly one pending owner judgement, no manual Exec wake or acceptance. |

Elapsed wall time from the first Attempt start to prepared owner attention was **57m 07s**. The
corrected Attempt itself took **26m 55s**. Accounted model spend was **$6.272 of a $25.00 ceiling**
($6.2055 research lead; $0.0665 Exec), with `poisoned: false`.

Direct feedback after attempt 1's frozen input automatically suppressed its owner handoff. The
corrected attempt was the only one that entered owner review.

## Historical candidate remains separate

`robotics_ai_alpha_test` preserved a frozen real-public-data pack and repeated its deterministic
calculation byte-for-byte (`6c2f65b45a9a1161100a61cfe56662583e3b367c7cf54e779201709535ffd041`). It returned
**inconclusive** because the pack could not establish point-in-time listing eligibility, delisting /
survivorship coverage, corporate-action adjustment or index membership. See
[dogfood-2-alpha-test-after-action.md](./dogfood-2-alpha-test-after-action.md). It supplies no live
price, company fact or alpha claim.

## Focused verification

- The Runtime source-manifest validator passed with `available_public,unverified_provider` required.
- `RESTLESS_TEST_DATABASE_URL=postgresql:///restless cargo test -p restless-orgintel --test outcome_review -- --nocapture` passed: 1 test. It covers qualified automatic preparation, a terminal replay after process recovery without a duplicate handoff, and missing-target / failed-gate blocked paths.
- `restless doctor -c robotics_ai_dogfood2_recovery` passed its real browser-to-Runtime probe.

## What worked

- The product, rather than an owner message, brought the finished native target to owner attention.
- A per-run Runtime manifest made the public route, as-of dates and unverified provider state
  inspectable without inventing a provider database.
- The 90-session liquidity rule and $250m floor were preserved after the first attempt showed why the
  bounds matter.
- The separate `_test` company kept a tempting but inconclusive historical result out of live research.
- GLM 5.3 produced a source-grounded revision within the bounded spend ceiling.
- The prepared owner brief now puts the live conclusion, evidence state and requested judgement at
  owner altitude, rather than making the owner inspect internal Work mechanics.

## What did not work yet

- The initial prompt did not make the frozen screen operational enough: the model substituted a lower
  cap floor and a 20-day average, then copied facts between cards. Human review caught both before
  owner attention; a next implementation slice should make declared screen facts machine-checkable
  before a research Attempt can qualify for review.
- The handoff was created at 08:54:15Z and its detailed owner brief was prepared at 08:58:58Z. That
  4m43s follow-through was still automatic and did not require an owner message, but the next run
  should measure it separately from target production so the owner-facing latency cannot hide behind a
  successful Work terminal state.
- Attempt 1 and attempt 2 reference the same target URI. Current-Attempt qualification kept the stale
  output out of review, but a future design should preserve an immutable content snapshot or reject
  target-path reuse across attempts.
- The provider lane remains deliberately unverified. Public sources were enough for this bounded
  result; no paid route or owner signup was needed.

## Purge and next smallest change

The host-side recovery prompt that duplicated the scenario contract has been removed. The canonical
scope is [dogfood-2.md](./dogfood-2.md), the versioned recovery company configuration, and the exact
Runtime copy retained with the run. Do **not** promote its one-off public fetch scripts, a generic
provider fallback, or the historical evaluator into a market-data/quant platform.

The next smallest implementation slice is not Dogfood 3: add a pre-review constraint check for the
declared research screen and source-to-card ownership, then preserve immutable target content per
Attempt. Keep the provider handoff pending until the owner chooses whether it is worth connecting.
