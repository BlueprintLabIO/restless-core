# Implications for OrgIntel and Sprint 06

## Decision

This research should sharpen Sprint 06 without pulling in the full teamwork-pattern library, health
scoring, or recommendation machinery. Founder alignment on 17 August 2026 made one behavioural
division explicit: the Exec commissions an outcome and appoints its lead; the lead assembles and
reshapes the one-level team, repairs it locally, and records one evidence-backed improvement.

Sprint 06 should establish the substrate and produce evidence:

- durable actors;
- one Exec-commissioned outcome with a named accountable lead;
- lead-led assembly of the smallest useful roster, with a stated difference for each member;
- Work attribution to that team;
- blocker escalation and repair by the lead;
- reasoned guidance lead → Exec → owner only when needed;
- one failed review causing a specific mechanism change on the next Attempt;
- an owner conversation with the lead that changes team Work; and
- a run showing reduced Exec coordination load.

Skills and external registries can inform how that test team is staffed. They should not become new
Sprint 06 entities or services.

## What OrgIntel should learn from the research

### 1. Team composition is the appointed lead's judgement over Work, not capability matching

The Exec chooses the outcome charter and accountable lead. The lead inspects the Work shape,
dependencies, quality risks, and available actor evidence, then assembles the roster. A future
recommendation mechanism may assist, but no current registry supplies a reliable machine-readable
answer. Cross-team resource conflicts return to the Exec because they exceed one lead's charter.

The composition rationale can start as a sentence in the team brief:

```text
This team uses a frontend design engineer for implementation, a copy specialist for buyer clarity,
and an independent visual critic because the output is public, judged, and prone to producer blind
spots. The work is mostly sequential, so no additional parallel coders are added.
```

That explanation is useful evidence without becoming a schema.

### 2. Actor expertise must be earned

A durable actor profile may eventually reference:

- applicable skills and their pinned versions;
- relevant tools and environments;
- accepted and rejected prior outcomes;
- domains in which critics or owners repeatedly accepted its work; and
- known failure modes.

The role remains identity. The evidence supports a changing expertise claim. Do not encode
"frontend-design-expert" merely because an actor loaded a skill once.

### 3. Attempt inputs need reproducibility, but only when useful

If skill use materially affects a run, the Attempt should be reconstructable from the exact skill
version, Work brief, model/provider, relevant files, and tool environment. Existing artifact
references or a readable execution brief may be enough at first. Add dedicated fields only after the
same need recurs.

### 4. The lead owns synthesis and repair, not authority

External research supports centralized synthesis as a way to contain ambiguity and error. It does
not justify giving the lead effect permissions, approvals, budgets, or credentials. Sprint 06's
Kernel-untouched slice is correct.

### 5. Knowledge should be assembled progressively

OrgIntel should know where relevant doctrine, actor evidence, skills, and task files live. It should
assemble the compact shared spine and give each actor local depth. It should not create a universal
knowledge base or copy every skill into every context.

### 6. The review target is part of the team design

For a judged outcome, the Work graph must lead to the real thing the user experiences. A website team
needs the running page; a paper team needs the rendered PDF; a game team needs the playable build.
This is not merely owner-surface presentation. It gives the producer, critic, and lead common
objective media for revision and debrief.

## How Sprint 06 can validate the function inside the sprint

Use the existing Aris outcome path and keep the emails unsent. A useful test scenario is a bounded
revision of one owner-visible Aris asset or campaign package, not another internal data-plane demo.

### Before the run

Record:

- the current artifact and owner's concrete objections;
- the outcome charter commissioned by the Exec and its named lead;
- the Work graph and why a team is warranted;
- the durable actors selected by the lead and the difference each one contributes;
- any candidate skill, exact pinned version, and why it is being tested; and
- the success criteria and primary review target.

### During the run

Observe:

1. the lead assembles and briefs each member from the Work graph;
2. ready Work wakes its responsible member without a second manual kickoff;
3. a member produces or reviews a real artifact;
4. a member raises one genuine blocker or revision conflict;
5. the lead changes roster, assignment, brief, dependency, context, skill, or revision Work and the
   member resumes;
6. the Exec does not perform that local repair;
7. a question outside the charter reaches the Exec with prepared state and only reaches the owner if
   the Exec cannot resolve it;
8. the owner sends free-form feedback to the lead; and
9. the lead answers for the team and changes the graph.

Do not manufacture a fake blocker solely to pass the acceptance test. Select work with a real open
quality or dependency question, and record honestly if no blocker occurs.

### At the end

Judge two layers separately:

**Sprint contract:** Did durable identity, lead-led assembly, Work-driven operation, accountable
leadership, owner addressability, local repair and improvement, reasoned escalation, and reduced Exec
load occur?

**Team-quality hypothesis:** Did this particular team produce a better accepted outcome for a
reasonable increase in time and cost?

The second result informs the later pattern library. It must not be used to claim the whole library is
implemented.

## Minimal research artifacts to retain

For now, ordinary files are enough:

- a small candidate-skill index;
- pinned copies or repository references for tested skills;
- team composition rationale in the Work or run brief;
- before/after review artifacts;
- critic report and owner decision; and
- a short debrief stating what changes next time.

OrgIntel can reference those paths and the related Work/Attempt identifiers. This follows §6.1's
file-first guidance and avoids a second source of truth.

## Possible post-Sprint-06 experiments

These are research questions, not tickets:

1. **Skill effect:** On the same frontend brief, does a pinned design skill reduce owner revisions or
   improve blind visual preference over the actor's normal method?
2. **Critic independence:** Does a narrow-context critic find more accepted defects than a critic
   given the producer's full reasoning?
3. **Team size:** Does adding an art-direction actor beat a lead plus design engineer enough to pay
   for the added coordination?
4. **Durable expertise:** Does the same actor improve over three comparable outcomes when its profile
   includes accepted/rejected evidence and prior debriefs?
5. **Pattern fit:** On sequential coding Work, does one strong actor beat a multi-agent pipeline; on
   independent market research, does a led parallel team win?
6. **Skill routing:** Do explicit Work-linked skill choices outperform implicit routing for important
   company tasks?

Each experiment should branch, run against a concrete outcome, compare, and purge the losing canon.

## What not to build yet

- a hosted Restless skill marketplace;
- a universal capability or expertise ontology;
- automatic installation from public registries;
- a scalar actor-quality score;
- permanent fixed departments for design, marketing, coding, and research;
- a team-pattern state machine;
- skills as Kernel permissions;
- a second governed store for skill packages or knowledge; or
- an LLM-only evaluator that replaces owner judgement for taste and commercial outcomes.

## Risks and dispositions

| Risk | Disposition in or after Sprint 06 |
|---|---|
| Team adds coordination but no useful difference | **Guarded:** require a composition rationale and compare outcome/cost |
| Lead becomes a new singleton bottleneck | **Guarded:** provider fall-through remains explicit; observe load and unavailable-lead behaviour |
| Same-context producer and critic agree falsely | **Guarded:** withhold producer reasoning or use a genuinely different model/context |
| Public skill changes or disappears | **Guarded:** pin the tested version in Runtime/Git |
| Skill package requests more tools than Work needs | **Guarded:** least capability in test; Kernel authority unchanged |
| External research does not transfer to Aris | **Accepted:** use it to choose experiments, not to claim results |
| File-first notes become messy | **Accepted until repeated retrieval failure:** complexity is cheaper than premature schema |

## Success criterion for this research

This dossier is useful only if it makes the next real team easier to compose, makes its choices
explainable, and produces evidence that changes a later run. If it merely grows a library of links or
causes OrgIntel to model unobserved concepts, it has failed.
