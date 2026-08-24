# Dogfood 1 — Robotics & AI public-equity research

**Status:** Run 1 evidence recorded; owner outcome review pending

**Version:** 0.2

**Type:** Live dogfood / outcome smoke — not a mechanical integration smoke

**Operating phase:** Exploration

**Company shape:** Research

**Run report:** [`dogfood-1-after-action.md`](./dogfood-1-after-action.md)

## Why this run exists

This is the first test of whether Restless can turn an ambiguous owner request into a
source-grounded, decision-useful research outcome with little owner coordination.

> Research a bounded universe of robotics and AI public equities, form falsifiable theses from
> real sources, and bring the owner a prepared directional trading decision.

It tests research judgement, source handling, evidence presentation, bounded delegation,
freshness, provider enrolment handoffs, and continuation across wakes. It does **not** test
automated execution, portfolio management, or investment performance.

This is a live-source dogfood. It must use real public and, where connected, real authenticated
data. It must not use simulated market data, invented provider responses, or synthetic research
evidence.

## Relationship to sprints, experiments, and mechanical smokes

- A **sprint** builds or improves a durable product capability.
- An **experiment** resolves a focused causal uncertainty.
- This **outcome smoke** runs one real owner job through the current product and records whether
  Restless actually produced a useful result.
- A **mechanical smoke** remains the fast deterministic integration check defined in
  `evaluation-dogfood` §10.4; this document does not redefine it.

Any platform work following this run must link to observed friction, not to an imagined research
platform.

## Success contract

### Desired outcome

The owner receives one native, reviewable research dossier covering a small, explicitly dated
universe of robotics and AI public equities. It distinguishes observation, claim, hypothesis,
assumption, and judgment; reaches a pointed directional conclusion; and states what would
invalidate every material thesis.

The output is private, owner-directed trading research. Its conclusion must be usable: it says
whether the current research stance is **Buy**, **Watch**, or **Avoid**, rather than ending at
generic information or an invitation to do more work.

### Accountable owner

The Exec appoints one accountable Research lead. The lead may work alone or commission Staff only
where a real semantic seam improves the result, such as a separate evidence challenge or a
different part of the value chain. The lead owns integration and the final preparation of the
review target.

### Initial scope

- **Theme:** publicly investable companies materially exposed to robotics and/or AI.
- **Initial universe:** no more than eight primary candidates, selected from U.S.-listed equities
  with accessible English-language primary sources. Up to two non-U.S. candidates are allowed only
  when the lead can cite equally reliable primary sources and label the market-specific limits.
- **Horizon:** potential catalysts, risks, and falsifiers over the next 6–18 months, not a promise
  of price movement.
- **Research question:** which candidates, if any, have a defensible Buy, Watch, or Avoid stance
  today; what is the time-bounded thesis; and what evidence would change that stance?

The lead may revise the universe when real evidence changes it, but must preserve the reason and
the before/after scope in the dossier.

### Explicit exclusions

- No brokerage connection, order entry, trade execution, rebalancing, position sizing, leverage, or
  custody.
- No personalised suitability assessment and no use of the owner’s finances, holdings, risk
  tolerance, tax situation, or personal circumstances.
- No performance guarantee and no assertion that this is a quant fund or licensed financial adviser.
- No paid data subscription, recurring charge, or provider upgrade without an exact, separate
  owner decision.
- No provider-root password, MFA factor, session cookie, recovery code, or API secret in Runtime
  files, agent text, chat, logs, or OrgIntel.

### Directional-advice posture

For every candidate that survives initial screening, the dossier must state one of:

- **Buy:** the research view is that a purchase is warranted at the stated as-of valuation or only
  under an explicit entry condition;
- **Watch:** the thesis is plausible but a named catalyst, valuation level, or evidence gap must
  resolve before a purchase is warranted; or
- **Avoid:** the available evidence does not support a purchase, with the reason and any condition
  that would reopen the case.

Each stance includes the as-of price/valuation reference when available, catalyst window, thesis
logic, disconfirming evidence, invalidation or exit condition, and a review date or event. A Buy
stance is a non-personal research conclusion the owner may independently act on; it deliberately
does not prescribe dollars, portfolio weight, leverage, or use personal financial circumstances.
Without a current, inspectable price or valuation basis and an explicit entry condition, the lead
must use Watch or Avoid rather than Buy.

This dogfood does not settle the regulatory classification of advice. Before the capability is
distributed, monetised, personalised, or connected to execution, obtain jurisdiction-specific legal
and compliance advice.

### Time, cost, and owner-attention envelope

- One bounded research run using the existing model envelope.
- Public primary-source research is the default path.
- At most one provider-enrolment handoff may be requested during the initial run.
- The default paid-data budget is **$0**. Any proposed paid tier must name the exact provider,
  product, price, terms, requested scope, and expected decision value before the owner sees it.
- Expected owner attention: the existing mandate, at most one provider handoff if genuinely needed,
  and one final outcome review. The owner must not troubleshoot tools, locate sources, or relay
  secrets.

### Sources of trust

Prefer, in order:

1. issuer filings, earnings releases, investor presentations, and other issuer-published records;
2. regulated exchange, government, or official public-company records;
3. a connected market-data provider whose scope and live response have been observed;
4. inspected secondary research, clearly labelled as secondary and never used to silently replace
   a missing primary source.

Every material factual claim needs a locator, source type, observed time, scope, and a freshness
limit appropriate to the claim. A source that cannot be opened, queried, or otherwise observed is
an **unknown**, not a working data pipeline.

## Working research pattern

This is a recommended pattern, not a required workflow. The Research lead changes it when evidence
requires a better approach.

1. Define an investable value-chain map and an initial candidate universe.
2. Gather primary-source evidence and establish an explicit as-of time.
3. Separate company facts from the lead’s causal thesis.
4. Form a base thesis, a serious counter-thesis, key catalysts, valuation-relevant questions, and
   falsification conditions for each candidate.
5. Seek evidence that could disconfirm the preferred story. A second model, withheld rationale, or
   adversarial researcher is useful only if it has genuine independence from the original claim.
6. Produce a Buy, Watch, or Avoid stance for each surviving candidate from its valuation,
   catalyst, counter-thesis, and invalidation logic — not a synthetic universal score.
7. Prepare the native review target with the precise independent action the owner can take now and
   the condition that would reverse it.

The lead must not substitute a generic narrative about AI or robotics for company-specific evidence.
If valuation inputs, market-data history, licensing terms, or an issuer claim cannot be established,
the dossier must name the gap and reduce the confidence accordingly.

## Data-source and pipeline probes

The first run grows only the smallest useful research path. It does not build a market-data platform
or a provider catalogue.

### Public-primary lane

The Runtime directly browses or retrieves public company and official records. This is ordinary
research work, not a governed external effect. The output retains the source locators and the
observed-as-of time.

### Optional authenticated-data lane

If public sources leave a material, decision-blocking gap, the lead may recommend **one** provider
with a read-only, bounded research scope. The recommendation must compare the public fallback with
the proposed provider’s actual observed access path, price, licence/usage constraint, and the exact
question the provider would answer.

An authenticated lane is usable only after:

1. the owner has completed the provider-hosted enrolment flow outside the Company Runtime;
2. any issued credential has entered through owner-authenticated Authority ingress and is stored or
   applied by the credential backend, not revealed to agents;
3. a live authenticated probe confirms the requested, read-only scope; and
4. the Research lead records the result, limitation, and freshness in the evidence bundle.

Until then, its state is `unverified`; the company continues with the public-primary lane rather
than narrating the provider as connected.

### Continuity check

The run prepares one lightweight refresh path: either a scheduled recheck or a defined trigger such
as an issuer filing, earnings release, or material source update. The later wake must report what
changed, what remained unverified, and whether the original thesis or directional stance changed.
It must not silently overwrite the original as-of research.

## Prepared owner moments

Owner participation and authority remain separate. Each item is an ordinary prepared owner handoff
or outcome review with a source Work, exact action, and observable continuation — never a new
onboarding workflow.

| Moment | Trigger | Exact owner action | Observable resume condition | If declined or unavailable |
| --- | --- | --- | --- | --- |
| **Scope confirmation** | The initial universe would materially exceed the stated theme or market boundary. | Confirm or narrow the proposed scope in the prepared review target. | OrgIntel records the bounded directive and the lead continues. | Keep the narrow initial scope; do not broaden by inference. |
| **Provider enrolment** | A specific data gap cannot be resolved credibly from public sources. | Open the exact provider-hosted signup/connection page in the owner’s normal browser; complete identity, MFA, and any provider-native steps; use the dedicated secret ingress if a key is issued. | A provider callback, authenticated probe, or other provider observation verifies the promised read-only access. Returning to Restless is not proof. | Continue with public sources and show the unresolved data gap; never ask the owner to paste a secret into chat. |
| **Paid-plan approval** | The lead proposes a non-free plan or recurring service. | Review the exact product, price, renewal/termination terms, scope, and alternative. Approve or decline that one spend decision. | Authority records the decision; a provider connection is still separately verified by live probe. | Stay on the free/public route; no hidden trial conversion or recurring spend. |
| **Outcome review** | The research dossier and evidence bundle are ready. | Open the prepared native target and accept, reject, or request revision of the Buy/Watch/Avoid conclusions. Any independent trade placement remains outside Restless. | The owner decision is recorded as a directive/judgment; the lead owns the resulting next work. | The dossier remains a completed research artifact; Restless does not place a trade. |

Financial-account administration, provider-root enrolment, identity verification, MFA, and initial
credential issuance always occur in the owner’s external browser. Restless must not treat an owner
click as verified access, nor materialise privileged browser sessions in the agent-accessible
Runtime.

## Required review target and evidence bundle

The final artifact is a native review target, such as a rendered research dossier with linked source
cards, positioned for owner review. It contains:

1. a one-page executive view: the question, as-of time, bounded universe, top Buy/Watch/Avoid
   conclusions, and the precise action and review trigger for each;
2. an investable value-chain map explaining each candidate’s relevant robotics/AI exposure;
3. a research card per candidate with observations, thesis, counter-thesis, catalysts, risks,
   valuation-relevant unknowns, falsification conditions, and a Buy/Watch/Avoid stance;
4. a source table containing locators, source type, observed time, claim supported, and freshness;
5. an evidence-quality and directional-conviction comparison, with the reason a candidate is or is
   not actionable now;
6. the state of every data lane: connected and live-probed, public-only, unverified, unavailable,
   or deliberately declined;
7. a concise method and limitations note, including missing data, licence limits, and any unresolved
   conflicts; and
8. an appendix or linked raw research files sufficient for a reviewer to trace material claims.

The memo must express judgment clearly enough for an owner to act independently. It must not imply
that Restless will execute, size, finance, or custody a transaction.

## Acceptance criteria

The named evaluator is the owner, aided by source inspection. A passing run requires all of the
following:

1. A real, reviewable dossier exists at a concrete Runtime path or URL.
2. The dossier covers the stated robotics/AI scope and records its as-of time and any scope changes.
3. Every material factual claim can be traced to an inspectable source or is honestly marked unknown.
4. At least one material counter-thesis or disconfirming evidence path appears for each candidate.
5. The lead distinguishes observed evidence, hypothesis, assumption, and judgment instead of
   laundering prose into facts.
6. Any authenticated data connection is proven by an observed live probe; unverified or declined
   connections are not presented as live.
7. The owner receives a prepared review target and needs no ordinary research coordination or secret
   handling.
8. Each surviving candidate ends in a Buy, Watch, or Avoid stance with a dated action condition and
   an invalidation/review trigger — not a generic trend report.
9. A Buy stance includes a current, inspectable price or valuation basis and explicit entry
   condition; otherwise the stance is Watch or Avoid.

Market movement after publication is not an acceptance criterion for this first run. A later
timestamped watchlist can test calibration across issuer events and time, but it is a separate
follow-up observation — not a paper portfolio, a trading system, or proof of investment returns.

## Failure, branch, and stop criteria

| Condition | Reading | Next action |
| --- | --- | --- |
| The lead produces polished but uncited market prose. | Research is not externally grounded. | Reject the outcome; repair source capture or reduce scope before rerun. |
| A provider signup is completed but cannot be live-probed. | Connection is unverified, not failed or live. | Continue with public sources or ask for a different bounded access path. |
| The owner must hunt for sources, debug provider setup, or relay a secret. | Restless failed the prepared-last-mile contract. | Record the exact friction; do not hand back instructions as the remedy. |
| Sources materially contradict a preferred thesis. | Valuable negative evidence. | Revise or kill that branch; preserve the contradiction. |
| The only available evidence is simulated or agent-authored. | The run has no real-world research grounding. | Stop; do not enter it into company research memory as market evidence. |
| The output drifts into personalised suitability, execution, sizing, leverage, or an actual transaction. | Scope/authority breach. | Stop the affected work and require a new owner mandate and legal/compliance review before any further action. |

## Risk dispositions

| Risk | Disposition | Treatment in this dogfood |
| --- | --- | --- |
| A research run places, sizes, finances, or executes a real trade. | **Invariant** | No brokerage, execution, allocation, leverage, or transaction capability is in scope. |
| Provider-root credentials or privileged browser state reach agents. | **Invariant** | Owner-only external-browser enrolment and Authority-owned credential ingress. |
| A provider connection is assumed to work after an owner click. | **Guarded** | Require a callback or authenticated live probe; otherwise display `unverified`. |
| Time-sensitive source claims become stale. | **Guarded** | Record as-of time, freshness, and a visible refresh path. |
| The same model agrees with its own thesis. | **Guarded** | Demand source-level counter-evidence and, when used, genuinely independent challenge conditions. |
| A useful source has no usable automated access path. | **Accepted** | Keep it manual/owner-operated or use the public path; do not invent a provider adapter. |
| The first research memo does not predict future returns. | **Accepted** | The first proof is decision usefulness and evidence quality, not alpha. |

## Evaluation and after-action report

The run report records:

- scenario/version, model(s), tools, source lanes, and resource envelope;
- the initial and final research question and universe;
- outcome, owner acceptance decision, and concrete review target;
- evidence links and source freshness;
- each owner moment, why it was necessary, time spent, and observed continuation;
- cost, elapsed time, wakes, rework, duplication, and any recovery;
- branches explored, contradictions found, and hypotheses killed;
- which result came from public sources, provider-confirmed data, or remained unknown;
- comparison limitations and the next smallest informative run; and
- the exact friction that would justify a sprint or experiment next.

The first run establishes feasibility. It does not by itself prove that OrgIntel beats a strong single
agent. If it produces a credible dossier, freeze a dated source pack and run a matched baseline or
repeat scenario with comparable source access, budget, and time before making that claim.

## Continue / branch / pivot / stop

- **Continue:** the owner can audit the research quickly, independently act on a clear Buy, Watch,
  or Avoid thesis, and Restless required only the justified owner moments.
- **Branch:** compare a public-primary-only run with one verified, narrowly scoped provider lane if
  the same material data gap recurs.
- **Pivot:** reduce the universe or change the research artifact if the dossier is not decision-useful
  despite sound sources.
- **Stop:** if useful research requires continuous owner coordination, unverifiable sources, or a
  prohibited move toward advice/execution, record that honestly and do not build a quant platform to
  disguise the failed premise.
