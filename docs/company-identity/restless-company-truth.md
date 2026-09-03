# Restless Company Truth — architectural convictions

**Status:** Owner-approved source evidence
**Effective date:** 3 September 2026
**Scope:** Company-wide product truth and public positioning
**Authority:** Owner direction

## Position

Restless is the operating system for company work.

Give Restless an outcome. A persistent company plans, delegates, does the work, checks the evidence,
and returns the decisions that need the owner.

Most agent platforms help a model complete a task. Restless builds a company that remains accountable
for the outcome.

## Product promise

Restless runs accountable company work and returns evidence-backed decisions to the owner. Its
optimisation target is useful economic output per unit of owner attention, time, cost and bounded risk.

## Architectural truth

These are product facts supported by the current architecture and exercised dogfood:

### The company, not the session

- `product.unit.company` — The product unit is a persistent company, not an agent or conversation.
- `product.persistence.actor` — Actor identity, responsibility and organisational memory persist when a
  model session or process ends.
- `product.persistence.work` — Work is a durable responsibility; an Attempt is one replaceable execution
  against an exact revision and evidence set.
- `product.models.labour` — Models supply labour and judgement but do not constitute the institution.
- `product.autonomy.proactive` — The company can wake, follow up, recover and continue without another
  owner prompt.

### Accountable delegation

- `organisation.exec.dispatch` — The Exec dispatches executable outcomes and remains available for
  portfolio judgement rather than becoming the producer.
- `organisation.lead.accountable` — Every executable outcome has one accountable lead.
- `organisation.lead.nonproducing` — The lead frames, guides, reviews and repairs the process; Staff
  performs content-changing production.
- `organisation.staff.semantic` — Delegation transfers an independently useful semantic responsibility,
  not merely a tool call.
- `organisation.team.minimum` — One end-to-end worker is the default; additional Staff must repay
  coordination cost through specialisation, independent evidence or real parallel value.
- `organisation.work.sparse` — The Work graph records responsibility and recovery boundaries rather than
  mirroring every reasoning step.
- `organisation.feedback.lineage` — Material rejection creates revision lineage and invalidates affected
  descendants rather than disappearing into chat.
- `organisation.messages.context` — Messages carry organisational meaning but cannot pretend that Work
  was commissioned or attempted.

### Owner Attention

- `owner.attention.scarce` — Owner attention is a scarce operating resource, not a feed of agent activity.
- `owner.attention.consequential` — Routine compliant work remains quiet; genuine judgement, authority and
  irreducible ambiguity reach Attention.
- `owner.attention.derived` — Attention is projected from authoritative unresolved facts rather than an
  urgency score or notification mode.
- `owner.outcome.native` — Prepared sites, documents, interfaces and media are reviewed in their native
  environment.
- `owner.completion.not_acceptance` — A finished process is not an accepted outcome.
- `owner.rescue` — The owner can inspect the same company computer, stop an actor, revoke consequences,
  restore and resume.

### Company Linux Runtime

- `runtime.linux` — Each company works on a real Linux computer with persistent files, processes, tools,
  repositories and browser state.
- `runtime.files.primary` — Files are the primary representation of productive work.
- `runtime.git.checkpoints` — Git records meaningful checkpoints, attribution, integration and rollback;
  it is not the real-time state bus.
- `runtime.browser.persistent` — Browser and desktop state belong to the company workspace and can be
  inspected by agents and the owner.
- `runtime.freedom` — Agents may build ordinary internal tools and services without first extending a
  global Restless ontology.
- `runtime.mess.recoverable` — Internal mess is an operational condition to repair, not a constitutional
  failure.
- `runtime.orgintel.optional_continuity` — Existing files and running internal work remain useful while
  organisational coordination recovers.
- `runtime.mature.infrastructure` — Restless prefers Linux, OCI, Git, Postgres and established process
  supervision over agent-specific substitutes.

### Authority at the edge

- `authority.consequences` — Governance constrains consequences, not thought.
- `authority.deterministic` — No LLM is authoritative for authentication, capability checks, budgets,
  approvals, external-effect execution or receipts.
- `authority.credentials.location` — Effects execute where credentials live; the company cell requests
  and never holds owner-root credentials.
- `authority.credentials.scope` — Model and external capabilities are scoped to company, actor, session,
  provider and purpose where applicable.
- `authority.effects.receipted` — Consequential external effects use explicit capabilities, idempotency,
  authoritative receipts and unknown-outcome reconciliation.
- `authority.spend.visible` — Missing terminal charged usage fails closed rather than becoming invisible
  spend.
- `authority.external_history` — Runtime restoration cannot roll back or erase external-effect history.
- `authority.failure.local` — Failure pauses the affected actor, capability or effect rather than freezing
  unrelated company work.
- `authority.kernel.small` — The Constitutional Kernel owns authority and blast radius, not project plans,
  team structure or ordinary file edits.

### Stable Company Identity

- `identity.executable` — Company Truth, Voice, Visual Language and Culture are versioned, evidence-backed
  organisational assets.
- `identity.pillars.distinct` — Truth, language, visual expression and conduct retain distinct evidence
  semantics rather than becoming one brand score.
- `identity.release.bound` — Identity-bearing Work binds once to an immutable release before execution.
- `identity.generated.cannot_canonise` — Generated repetition, model majority and evaluator taste cannot
  promote themselves into company policy.
- `identity.evidence.examples` — Human passages, product facts, accepted and rejected examples, and
  exercised decisions outrank generic adjectives.
- `identity.humans.visible` — Named human authors remain distinguishable and need not impersonate a single
  synthetic company narrator.
- `identity.drift.concrete` — Drift names the exact stale claim, dependency or artifact; no universal
  consistency score substitutes for evidence.
- `identity.no_authority` — Identity fitness never grants publication or another external effect.
- `culture.conduct` — Culture is inferred from observed decisions, disagreement, uncertainty, correction
  and customer treatment—not slogans or worker scoring.

### Quality and learning

- `quality.lead.convergence` — The accountable lead owns native review, repair, bounded replacement and
  convergence to the declared outcome standard.
- `quality.native` — Quality is judged in the artifact's native medium, not inferred from source code or
  model confidence.
- `process.outcome.flexible_plan` — Business processes keep a stable outcome contract and control points
  around a freely adaptable execution plan.
- `process.playbooks.earned` — Useful improvisation may become a versioned playbook after repeated evidence;
  a hard invariant is the last step, not the first.
- `process.rules.reversible` — Rules and mechanisms can be weakened or deleted when evidence shows they no
  longer help.
- `process.recovery` — Detection and repair are preferred to speculative state machinery for ordinary
  organisational failure.
- `process.evidence.before_ontology` — New abstractions must earn their way in through observed work.
- `process.dogfood` — Real Restless teams must produce native outcomes, expose harness defects and survive
  independent criticism.
- `process.negative_results` — Failed and invalid runs remain labelled evidence rather than being rewritten
  as successful demonstrations.
- `process.clean_room` — A contaminated production path may be purged and restarted from ground truth while
  preserving failure evidence.
- `process.deletion` — Removing losing machinery is product progress.

### Company-cell Cloud

- `cloud.tenancy.company` — A company cell, not a human user, is the isolation and tenancy boundary.
- `cloud.planes.separate` — Company cell, owner account plane and infrastructure fleet have separate state,
  authority and restart boundaries.
- `cloud.inspect.no_wake` — Reading the owner cockpit does not wake agents or spend model money.
- `cloud.dedicated_first` — Dedicated company deployments precede shared multi-tenancy until measured
  economics justify sharing.
- `cloud.publication.separate` — The public Restless site is outside company Runtime; every public revision
  remains a separately governed effect.

## Public voice

- Lead with the owner consequence, then show the mechanism as proof.
- Use direct, concrete language and short sentences.
- Prefer “company”, “outcome”, “work”, “evidence” and “decision” to generic agent terminology.
- Say what Restless does; do not narrate its internal review procedure to customers.
- Present contrarian architecture only where it demonstrates a customer benefit.
- Keep people visible. Do not make every channel sound like one synthetic narrator.

## Negative evidence

Do not position Restless as:

- a chatbot with more tools;
- a fixed planner–writer–critic workflow;
- a swarm whose activity is itself the outcome;
- a passive reporting dashboard;
- a permission layer around every thought or file edit;
- a generic “AI workforce” with invented employees;
- an always-on notification stream;
- a proprietary replacement for Linux, Git, browsers or ordinary applications; or
- a system that silently acts, spends, publishes or rewrites accepted company truth.

Avoid academic abstraction such as “bounded evidentiary substrate” in public copy. Architectural terms
belong on the site only when paired with the plain owner benefit and inspectable product proof.

## Landing-page expression

The landing page should lead with one customer truth:

> Your company should keep working when the chat ends.

Its main proof is a live, restrained depiction of one outcome moving through the persistent Exec,
accountable lead and Staff organisation, accumulating native evidence and returning one consequential
owner decision. The architecture should be visible as product behaviour, not presented as a feature
inventory.

The six public proof chapters are:

1. A company, not a chat session.
2. Delegation with an accountable chain.
3. A real Linux workplace.
4. Attention is the scarce resource.
5. Freedom inside; control at the edge.
6. The institution outlives the model.

This source does not authorise publication. The built site remains a local candidate until a separate
owner-authorised publication effect occurs.
