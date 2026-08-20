# Sprint 07 — Present the smallest truthful story the owner needs

**Status:** Active. Implementation, automated verification and the genuine Aris owner item are
complete; T5 remains open for the founder's ten-second account and source-owned decision. Ticket
status lives only in the checklist below.
**Date:** 18 August 2026
**Spec refs:** `orgintel` §5.2 / §6.2 / §8.2, `owner-cockpit` §5 / §7.4 / §10.3 / §11.4 / §12.2,
`cross-layer-contract` §3.1, `ARCHITECTURE.md` §2.2 / §4.3 / §9.1 / §16.1 / §16.2 / §16.5,
Sprint 06 T5

---

## Observed live friction

The live Aris Attention surface presented this owner item:

> **Release integrity: centre-chrome Terms/Privacy with legal audit, sitemap DB decoupling, honest
> /api/health with commit exposure (supersedes wedged c70eaa68)**

The first screen then repeated worktree paths, commit hashes, Attempt and gate identifiers, ENOSPC
diagnosis and validation-chain topology across the main detail, “Why it matters”, recommendation and
Exec rail. The underlying owner story was:

> The release is prepared, but validation is paused because the Mac has about 300 MiB free. Free
> 10 GB and Restless will resume automatically.

This is not one unusually verbose actor. It exposes the current seam:

- the Work title is reused as the owner-facing title;
- `owner_handoffs.prepared_state` is projected verbatim as “what happened”;
- the BFF invents generic “why it matters” and recommendation copy;
- `owner_judgement` is rendered as an outcome review even when the immediate condition is an
  operational blocker;
- implementation evidence and owner meaning receive the same visual weight;
- the Exec rail repeats the selected item instead of remaining a conversation about it;
- the owner must read internal coordination detail to discover the one consequential point.

The company has done the work of coordinating itself, but has not done the final work of explaining
itself. The cockpit is showing an internal record rather than a prepared last mile.

## Outcome

> **When Restless asks for owner attention, the accountable lead presents the smallest truthful
> story needed to understand and act: what happened, why it matters, the recommendation, the exact
> owner ask and what happens next. The first screen is written at owner altitude; implementation
> detail remains inspectable as supporting evidence. The accountable lead owns this translation,
> uses one shared owner-presentation skill, and records the resulting brief with the existing
> OrgIntel handoff. Exec authors only company-wide, cross-team or otherwise unowned briefs. The
> cockpit renders the prepared brief without silently rewriting it.**

A cleaner card, shorter hard-coded copy, or a model summary generated when the page opens does not
complete Sprint 07. One observed run must show that a real owner understands and handles a real item
without reconstructing the story from internal Work detail.

### Operating loop

```text
team produces outcome, evidence, blocker or decision
→ accountable lead completes every machine-doable step
→ lead decides whether owner attention is genuinely required
→ lead uses the shared present-to-owner skill to prepare one stable brief
→ OrgIntel records the brief, exact ask, resume condition and source references
→ cockpit renders owner meaning first and evidence behind it
→ owner acts, asks or decides through the source-owned operation
→ resolution returns to the affected Work; the company observes and resumes
```

For a company-wide, cross-team or platform-originated matter with no accountable lead, Exec performs
the preparation. Exec is not the mandatory editor for every team and the cockpit is not an
unaccountable editor at all.

### Success contract

The sprint passes when one observed run demonstrates all of the following:

1. **Ten-second comprehension.** From the initial focused item, without expanding evidence, the owner
   can state: what happened, why it matters, what Restless recommends, whether the owner must act, and
   what happens after action or inaction. The owner's account matches the source evidence.
2. **Lead-owned translation.** A team outcome is translated by its accountable lead. The durable
   record names that actor and the exact Work/Attempt or source state it prepared. Exec is not woken
   merely to shorten another team's prose.
3. **Shared capability, not a presentation department.** The lead and Exec use the same
   present-to-owner skill; organisational briefs use the same handoff operation. There is no new
   global presentation agent, service, queue or LLM call in the BFF/browser render path.
4. **Attention is earned before it is polished.** Before creating the brief, the actor determines
   whether the need is genuinely owner-only and records what machine-doable preparation or local
   resolution is complete. A test item the lead or Exec can resolve never reaches the owner queue.
5. **Correct meaning and action.** An outcome review, bounded decision, Authority approval and
   irreducible human last mile are not all presented as “Review outcome”. The primary action names
   the real consequence and invokes the existing source-owned transition. Conversation never
   silently resolves a handoff.
6. **One clear story.** The primary view contains one plain-language headline, a concise situation
   and impact, one recommendation and one exact owner ask where needed. “Why it matters” does not
   restate the summary, and the rail does not repeat the main narrative.
7. **Evidence remains complete.** Commits, worktrees, gates, receipts, logs, critic findings and raw
   implementation notes remain reachable behind evidence disclosure. Material uncertainty,
   counter-evidence and adverse consequences are never removed merely to make the brief shorter.
8. **Native outcome first.** Where there is a live site, rendered document, playable build, media
   artifact or prepared browser state, that ReviewTarget remains the primary evidence surface.
   Process prose cannot replace an available outcome.
9. **Stable and attributable.** The same handoff renders the same authored brief across refreshes and
   clients. A material source change causes an attributed refresh against the new state; it is not a
   fresh anonymous summary on every read.
10. **Graceful failure.** A missing or malformed owner brief does not dump raw internal state into the
    primary view or invent reassuring copy. It stays with the responsible actor for preparation, or
    identifies itself honestly when an urgent irreducible human condition cannot wait.
11. **The queue becomes scannable.** Each row has an owner-facing title and a short statement of the
    ask or state. Two real items can be distinguished without opening both or deciphering truncated
    implementation titles.
12. **Owner effort falls without lost truth.** Against the captured baseline, the owner opens fewer
    technical details and spends less time identifying the ask, while still finding the exact
    evidence and source state when challenged.

## Why this is the next slice

Sprint 05 brought the real outcome and evidence into the owner surface. Sprint 06 gave teams an
accountable lead and moved ordinary coordination below Exec and the owner. Those gains now expose the
next boundary: an organisation can run and review its work internally, yet still communicate upward
like a build log.

The current screen also weakens the value of the new lead. A lead that assembles a team, repairs its
graph and applies critic evidence but hands the owner raw Work state has stopped one step short of
accountability. “Speak for the team” must mean selecting and explaining what matters, not concatenating
everything the team knows.

This is a judgement problem. Plain-language translation, relevance, consequence, recommendation and
the decision about what not to foreground are open-ended. Regexes, field truncation, canned domain
copy and character-count gates cannot perform that work. Deterministic machinery should preserve the
author, source references, action semantics and stable delivery; the accountable model actor should
exercise the editorial judgement.

## The translation boundary

### Semantic owner: accountable lead

The actor accountable for the outcome prepares the brief because that actor has the deepest relevant
context and owns the consequences of omission or emphasis. Preparation is the existing final step
before independent or owner review, extended to include owner-level explanation.

A lead must not escalate a question merely because it is difficult to explain. It first resolves
machine-doable work, identifies the irreducible decision or participation, and prepares the exact
resume condition.

### Shared capability: present to owner

The sprint may add one shared skill and a narrow operation for recording its result. The skill should
help any accountable actor:

1. classify the situation as outcome, decision, approval, blocker/recovery, opportunity,
   contradiction or irreducible human last mile;
2. decide whether owner attention is actually required;
3. identify the one consequence that matters now;
4. separate observed fact, interpretation, recommendation and owner choice;
5. write a plain-language headline and concise context;
6. state the exact ask, no-action consequence and observable resume condition;
7. link the strongest native outcome and supporting evidence;
8. check for hidden uncertainty, counter-evidence, jargon and duplication.

The skill performs judgement. The recording operation may validate identity, source references and
required semantics, but must not become a templated copywriter or universal command algebra.

### Durable home: the existing source record; OrgIntel handoff for organisational items

For team outcomes, decisions and blockers, the owner-ready brief is organisational communication and
belongs with the existing OrgIntel owner handoff. It is not a new source of operational truth:
Runtime still owns files and outcomes, Authority still owns approvals/effects/receipts, and external
providers still own external reality.

Do not funnel an Authority approval into an OrgIntel handoff merely to obtain nicer copy. Its exact
party, amount, command, consequence, state and receipt remain in Authority and its action writes
there. Where the requesting actor's recommendation materially helps, carry it as attributed
organisational context linked to the source reference using the smallest existing seam the run can
support; do not create a second approval or duplicate authoritative effect facts.

The minimum semantic envelope is:

```text
owner-facing headline
plain-language situation
why it matters now
recommendation and material uncertainty
exact owner ask, if any
what happens without action
real deadline or decision window, if any
observable resume condition
native ReviewTarget and supporting evidence references
accountable author and source snapshot
```

This is a working presentation convention, not permission to create a second `OwnerBrief` lifecycle,
document custody system or global presentation database. First attempt to express it by tightening
the existing handoff preparation and projection. Add only the minimum durable fields that a real run
proves cannot be represented honestly through the current handoff; then purge the losing shape.

### Exec's bounded role

Exec authors the brief when:

- the matter is company-wide or crosses team charters;
- recommendations from multiple leads conflict;
- the accountable lead is unavailable and the recorded fall-through reaches Exec;
- the source is a platform/company condition with no responsible lead;
- the remaining decision concerns company strategy, mandate or allocation at Exec altitude.

Exec may return an item to the lead if owner attention is not earned or the brief is unprepared. It
does not rewrite every lead's handoff and does not become a central communications bottleneck.

### Cockpit's bounded role

The cockpit owns layout, hierarchy, progressive disclosure and the affordance that invokes the
source-owned action. It does not decide what the event means, generate a recommendation, or summarize
the handoff on read. Its cross-plane projection may combine already-authored owner meaning with live
source health and evidence availability.

## Editorial defaults, not hard-coded truth

The primary viewport should normally contain:

- a one-line owner-facing headline;
- one or two short sentences explaining the situation and consequence;
- one recommendation;
- one primary action or a clear statement that no action is required;
- a real deadline or decision window when delay changes the consequence;
- the no-action/resume condition where consequential;
- a compact evidence entry point.

These are review standards, not database length constraints. A material legal, financial or
irreversible decision may need more context. Brevity never outranks truth, and technical terms remain
when the owner must reason about that exact technical fact.

The brief distinguishes:

- **Observed:** what source evidence shows;
- **Interpretation:** why the company believes it matters;
- **Recommendation:** what the accountable actor advises;
- **Owner choice/action:** the exact decision or participation requested.

The UI need not label all four habitually when prose makes them clear. The distinction is semantic,
not another set of decorative eyebrows.

## Tickets

| ✓ | Ticket | Layer | Evidence served | Depends |
|---|---|---|---|---|
| [x] | [**S07-T1 · Shared present-to-owner skill and accountable preparation**](sprint-07/t01-present-to-owner.md) | OrgIntel + Runtime context | Leads currently place internal Work reports directly into `prepared_state`; no actor owns the upward translation step | S06-T5 |
| [x] | [**S07-T2 · Authored attention meaning and truthful action semantics**](sprint-07/t02-attention-admission.md) | OrgIntel + owner projection | The BFF reuses Work titles, invents generic why/recommendation copy and renders every `owner_judgement` as review | S07-T1 |
| [x] | [**S07-T3 · Owner meaning first, evidence progressively disclosed**](sprint-07/t03-owner-brief-surface.md) | Owner surface | The first viewport gives implementation detail the same weight as consequence, recommendation and ask | S07-T2 |
| [x] | [**S07-T4 · One narrative, stable across the main surface and conversation**](sprint-07/t04-one-narrative.md) | Owner surface + OrgIntel messages | The Exec rail repeats the selected item and can compete with the accountable lead's brief | S07-T2, S07-T3 |
| [ ] | [**S07-T5 · Dogfood three attention shapes, measure comprehension, purge the losing representation**](sprint-07/t05-dogfood-and-purge.md) | All touched layers | One screenshot can produce an overfit template; the boundary must work for a real outcome, decision/approval and blocker/last mile | S07-T1–T4 |

The expected path is **T1 → T2 → T3 → T4 → T5**. T3 may begin from the agreed semantic envelope while
T2 lands, but the final surface must render source-backed fields rather than fixtures or UI-invented
copy.

## Slice per layer

**OrgIntel.** Own the accountable author's prepared owner meaning, author/source attribution,
explicit owner ask and refresh against a changed source snapshot. Reuse the existing owner handoff
and Work/Attempt/artifact references. Only add durable columns or a compact payload if the first
implementation attempt proves the current handoff cannot carry the brief without parsing prose or
duplicating source truth.

**Owner surface.** Render the authored brief as the dominant first-screen story. Keep the native
ReviewTarget and material evidence available through progressive disclosure. Queue rows become
scannable. Actions reflect the source-owned transition. The conversation rail stays available for
questions but does not duplicate the brief or become a second decision path.

**Runtime.** Supply the shared presentation skill through ordinary actor context/skills and preserve
the real ReviewTarget and supporting artifacts. No presentation daemon, renderer service, process
class or content-custody layer.

**Kernel / Authority.** No new authority or approval mechanism. Authority requests keep their
deterministic source facts and existing grant/decline operations. An OrgIntel/Exec brief may explain
those facts but cannot rewrite the envelope, amount, party, command, consequence or receipt.

## Verification and dogfood

This sprint combines deterministic provenance checks with human judgement. A styling snapshot alone
cannot prove comprehension; a subjective review without source checks cannot prove truth.

### 1. Capture the baseline

Preserve the current Aris item screenshot and record:

- time until the owner can state the situation and ask;
- which sections and evidence the owner had to read;
- whether the owner initially misclassified the item;
- repeated claims across main detail and rail;
- technical identifiers visible before evidence is requested.

This baseline is product evidence, not a fixture to reproduce.

### 2. Scripted `_test` company

Produce three source-backed items:

1. **Outcome review:** a running ReviewTarget with independent evidence and accept/request-changes
   semantics.
2. **Bounded decision or Authority approval:** a real choice with alternatives, consequence and an
   exact source-owned action.
3. **Blocker or human last mile:** a prepared state with an observable external resume condition.

Also create a fourth machine-doable blocker that the lead or Exec can resolve. Expected result: it is
resolved below the owner and never appears in Attention.

For each surfaced item, assert headlessly that:

- the author and source reference are durable;
- the owner action maps to the correct existing source operation;
- refreshing does not regenerate or alter the brief;
- evidence references resolve to the same artifacts/receipts/source state;
- conversation alone does not resolve the item;
- material source change creates an attributed refresh or honest stale state;
- no primary-view field is synthesized from a raw log by the browser.

### 3. Independent briefing review

Give an independent critic the source evidence and prepared brief, without the internal team
conversation. It checks:

- every material brief claim is supported;
- the main consequence and uncertainty are preserved;
- the recommendation follows from the evidence rather than merely sounding decisive;
- the owner ask is the smallest one that unlocks progress;
- implementation detail omitted from the first screen remains available as evidence.

A critic score is supporting evidence, not the pass condition. The owner comprehension run decides
whether the story works for its intended human.

### 4. Live Aris run

Use the next genuine Aris outcome or decision; do not manufacture a simulated commercial fact. The
accountable lead prepares it through the landed path. From the first viewport, the owner explains the
situation, consequence, recommendation, ask and next state, then inspects at least one technical
evidence item and confirms it supports rather than contradicts the brief.

The sprint fails if the owner must open Work, read chat history, decode hashes or ask “what exactly do
you need from me?” before acting.

### 5. Measures

Compare with baseline:

- time to correct comprehension;
- technical disclosures opened before decision;
- owner clarification messages needed to discover the ask;
- duplicated claims visible in the first viewport;
- unsupported or contradicted material claims;
- attention items withdrawn because owner attention was not genuinely required.

Do not optimise a word count or empty queue as a proxy for these outcomes.

## Degraded and change behaviour

- If source evidence becomes unavailable, the brief remains visible with explicit source health and
  cannot imply that evidence was re-probed successfully.
- If the underlying Attempt, amount, party, effect command or ReviewTarget changes materially, the
  old brief does not silently attach to it. The accountable actor refreshes or withdraws it.
- If the responsible lead is unavailable, the existing recorded fall-through reaches Exec. The
  owner never receives an anonymous machine summary merely because an actor crashed.
- Irreducible identity, CAPTCHA, MFA, legal-attestation and payment-confirmation handoffs retain their
  direct human boundary. Their wording may use the shared standard, but they do not wait for Exec to
  perform an impossible action.

## Risks and dispositions

| Risk | Disposition | Reason |
|---|---|---|
| Concision hides adverse evidence, uncertainty or irreversible consequence | **Guarded** | The semantic envelope separates evidence, interpretation and recommendation; independent review compares the brief to source evidence; material counter-evidence is mandatory even when the brief grows. |
| A polished brief becomes persuasive spin | **Guarded** | Material claims retain source references, the owner can inspect exact evidence, and recommendation is distinguishable from observed fact. |
| Untrusted evidence instructs the presentation actor or contaminates its recommendation | **Guarded** | Runtime/web content is evidence, not instruction; the actor retains the owner mandate and source provenance, and the critic compares claims to the source rather than trusting embedded prose. |
| Evidence disclosure leaks secrets or privileged credentials | **Invariant** | Progressive disclosure preserves the existing secret/redaction boundary; “show the evidence” never means copying raw credentials into OrgIntel or the browser. |
| Exec becomes a central copy editor and recreates the Sprint 06 bottleneck | **Invariant** | The accountable lead authors team briefs; Exec handles only its own altitude, recorded fall-through and genuinely cross-team/company matters. |
| The BFF becomes a global model-powered presentation layer | **Invariant** | Meaning is authored once at preparation/refresh and retained with its existing source record—an OrgIntel handoff for organisational items; rendering performs no semantic model call. |
| A rigid template replaces actor judgement | **Accepted** | The shared envelope is an editorial default, not a fixed domain form. Add structure only where source/action semantics need deterministic preservation. |
| Short copy becomes stale while Work changes | **Guarded** | Briefs name their source snapshot; material change requires attributed refresh, withdrawal or an honest stale state. |
| Malformed briefs delay urgent human last miles | **Guarded** | Direct irreducible categories retain a minimal honest fallback; ordinary unprepared judgement stays with the responsible actor instead of dumping raw internals on the owner. |
| The sprint beautifies machine-doable escalations | **Pending fix** | T1/T2 and the `_test` negative case require attention admission before presentation. A brief that cannot name the irreducible owner need is returned or resolved below the owner. |

## Deliberately out of scope

- A global presentation service, presentation agent, semantic BFF or LLM-on-page-load summarizer.
- Rewriting all inter-team messages into executive prose. Internal communication remains detailed and
  task-shaped.
- A new universal `OwnerBrief` entity or lifecycle before the existing handoff is tried and observed.
- A domain-specific wizard, fixed provider catalogue, regex jargon filter or canned CEO-copy
  templates.
- Hiding raw evidence, removing logs, or replacing native outcome review with summaries.
- A redesign of Work, People or Authority beyond the seams needed to preserve correct context and
  source actions.
- Solving every platform blocker such as disk cleanup. Sprint 07 must stop presenting a machine-doable
  blocker as owner judgement, but host storage policy and cleanup mechanics need their own observed
  scope if they remain a repeated failure.
- Multiplayer, owner roles, notification channels or a general reporting/analytics system.
- Per-owner tone profiles or automatic style learning. Begin with one accepted owner-brief standard;
  use observed corrections as evidence before adding personalisation machinery.

## Deletion

Per `ARCHITECTURE.md` §16.5, the sprint should make these current representations deletable:

- generic OrgIntel judgement copy invented in `restlessd::attention` for “why it matters” and
  recommendation;
- reuse of the internal Work title as the owner-facing Attention title;
- primary-view rendering of the whole `prepared_state` implementation report;
- duplicated selected-item narrative in the Exec rail;
- generic “Review outcome” actions where the source is not an outcome acceptance decision;
- any candidate owner-brief representation not selected after T5's comparison.

Do not delete technical evidence or historical Work detail. The sprint changes default prominence,
not custody or truth.

## Founder decisions and sprint defaults

1. **Who owns translation?** The accountable lead for team work; Exec for company-wide, cross-team,
   unowned platform matters and recorded fall-through. The cockpit never owns semantic translation.
2. **When is translation performed?** Once during final owner-handoff preparation and again only on
   an attributed material refresh—not on every read.
3. **Where does it live?** Organisational briefs live in the existing OrgIntel owner handoff, using
   the smallest extension a real run proves necessary. Authority items remain in Authority; evidence
   stays in Runtime, Authority and providers.
4. **What is shared?** A present-to-owner skill and narrow record/update operation, not a new actor or
   service.
5. **What reaches the first screen?** Situation, consequence, recommendation, exact ask and next
   state. Technical implementation belongs behind evidence unless it is itself the consequential
   fact the owner must judge.
6. **What does the rail do?** Conversation and clarification with the accountable actor. It does not
   echo the brief or decide the handoff.
7. **What happens to an unprepared item?** Ordinary judgement returns to its accountable actor. An
   urgent irreducible human handoff uses a minimal honest fallback and never invents missing context.
8. **Does brevity win?** No. Truth, material uncertainty, consequence and reversibility win; brevity
   removes irrelevant implementation detail and repetition.

## Salvage

No legacy component is assumed. If ticket decomposition identifies a candidate in `docs/SALVAGE.md`,
the ticket must name it and re-probe it against the authored-brief boundary before lifting it.

## Exit evidence

Sprint 07 closes only with:

1. the baseline and final first-viewport captures;
2. the scripted `_test` run covering three surfaced shapes and one locally resolved non-item;
3. source/action/provenance checks and restart-stable rendering;
4. the independent briefing review against exact evidence;
5. one genuine Aris owner item prepared by its accountable lead;
6. the owner's correct ten-second account and completed source-owned action or decision;
7. a deletion report naming the generic/duplicate representation removed;
8. a friction note for anything the owner still had to decode, repeat or ask.

If the final evidence is only a cleaner screenshot, the sprint is incomplete.
