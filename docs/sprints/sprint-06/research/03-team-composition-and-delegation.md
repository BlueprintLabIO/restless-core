# Team composition and delegation

## Finding

The best current evidence rejects both a flat swarm and a fixed cast of specialists. OrgIntel should
form the smallest team whose differences match the work, put one lead in charge of synthesis and
repair, and disband or reshape it when the dependency structure changes.

This directly supports `orgintel` H4, "task-shaped teams beat fixed multi-agent structures," while
keeping it a hypothesis to be tested in Restless rather than a doctrine borrowed from benchmarks.

## Start with work shape

| Work shape | Default pattern | Why | Main failure signal |
|---|---|---|---|
| Coherent, tightly coupled, or strongly sequential | Single accountable owner | Preserves one reasoning stream | Actor lacks a genuine specialty or context window |
| Independent hypotheses or separable search spaces | Parallel exploration with lead synthesis | Buys breadth and latency reduction | Duplicate searches or inconsistent assumptions |
| External-facing, subjective, or hidden-error-prone output | Producer-critic | Buys independent judgement | Critic shares producer reasoning and merely agrees |
| Genuinely different sequential specialties | Specialist pipeline | Gives each stage focused methods and tools | Handoffs lose context or serial latency dominates |
| Repeated failure, contradiction, or blocker | Recovery huddle | Replans around evidence | Meeting produces discussion but no changed Work |

The table mirrors `orgintel` §6.3 because current external evidence supports its shape. Sprint 06
still does not implement the pattern library.

## Current agent-system evidence

### Controlled scaling evidence

Google Research's 2026 [controlled evaluation of 180 agent configurations](https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/)
is the strongest current warning against "more agents" as a default. Across its benchmarks:

- centralized multi-agent work improved a parallel financial-analysis task by about 81 percent;
- every multi-agent variant degraded a sequential planning task by 39 to 70 percent;
- coordination cost rose with tool count; and
- independent-agent designs amplified errors more than centralized designs.

These are benchmark results, not universal constants. The authors' predictive model explains only
part of the variance. The robust lesson is the boundary condition: decomposability, sequential
dependencies, and tool density should influence team shape.

### Production research systems

Anthropic's [multi-agent research report](https://www.anthropic.com/engineering/multi-agent-research-system)
describes strong gains for breadth-first research with an orchestrator and parallel workers. The same
report says multi-agent runs used roughly 15 times the tokens of ordinary chat and that coding tasks
often have fewer truly parallel parts. Its 90.2 percent improvement is an internal, domain-specific
evaluation, not independent proof.

Useful operational lessons are:

- delegate with a clear objective, output shape, boundaries, and source/tool guidance;
- scale the number of workers to query complexity;
- use separate context windows for independent exploration;
- return concrete artifacts to the lead; and
- evaluate end outcomes because valid runs may follow different paths.

### Manager versus handoff

[OpenAI's Agents SDK orchestration guide](https://openai.github.io/openai-agents-python/multi_agent/)
distinguishes manager-style orchestration from handoffs:

- a manager calls specialists and retains responsibility for synthesis and the final response;
- a handoff makes the specialist the active owner of the interaction.

That is a useful vocabulary, not a required SDK. For Sprint 06, the accountable team lead is closer
to a manager for bounded member work and closer to a handoff when the owner addresses the lead
directly. The implementation should remain Restless-native and provider-neutral.

### Prefer simple compositions

Anthropic's [Building effective agents](https://www.anthropic.com/engineering/building-effective-agents)
recommends increasing orchestration complexity only when the task needs it. It distinguishes fixed
workflows, which suit enumerable and predictable work, from model-directed agents, which suit
open-ended judgement. This aligns with `LLM_CURE.md`: classify the problem before choosing the tool.

## Human-team evidence and what transfers

Human teams are not language-model teams, but the older evidence helps identify durable coordination
problems.

### Shared mental model

A [meta-analysis of team cognition](https://pubmed.ncbi.nlm.nih.gov/20085405/) combined 65 independent
studies and found that team cognition contributed information about performance beyond behavioural
and motivational dynamics. The transferable mechanism is a compact common operating picture: goal,
constraints, current plan, decision rights, and dependency state. It is not a full shared transcript.

### Transactive memory

A [meta-analysis of transactive memory systems](https://pubmed.ncbi.nlm.nih.gov/30024196/) examined
how context affects the relationship between "who knows what" and team performance. For Restless,
the actionable interpretation is modest: the lead should know which durable actor has which evidence,
methods, tools, and artifacts. Every member does not need every file.

### Task interdependence

A [meta-analysis of structural interdependence](https://iro.uiowa.edu/esploro/outputs/journalArticle/Structural-interdependence-in-teams-An-integrative/9984380517902771)
used 107 independent samples and 7,563 teams. It found that task and outcome interdependence work
through different team processes. The enduring principle is that team design should follow how work
actually depends on other work, not a generic org chart.

### Brief, monitor, support, debrief

The US Agency for Healthcare Research and Quality's
[TeamSTEPPS framework](https://www.ahrq.gov/teamstepps-program/curriculum/intro/explain.html) organises
teamwork around communication, leadership, situation monitoring, and mutual support. It also warns
that training without implementation and sustained commitment may produce no positive result.

The healthcare setting is specialised, but four mechanics transfer well:

- a lead names the goal, roles, and plan;
- members check back when messages or handoffs matter;
- the lead monitors state and shifts work when a member is blocked; and
- the team debriefs against the real artifact or outcome.

These are behaviours to elicit and observe, not new state machines.

## Difference is the reason to delegate

OrgIntel's current §6.3.1 is stronger than most vendor guidance because it asks what a second actor
actually buys. A member should contribute one or more meaningful differences:

- durable role and accumulated outcome history;
- a model with different strengths or failure modes;
- deliberately narrower or different context;
- task-specific skill or domain references;
- unique tools or environment access; or
- independent search or judgement capacity.

Parallel copies can still be useful for independent exploration. They should be described honestly as
parallel capacity, not a multidisciplinary team.

## A reliable delegation brief

The lead should give a member enough local depth without copying the whole company transcript:

```text
Outcome: the concrete result this member must produce
Why this actor: the difference or evidence that justifies the assignment
Inputs: exact files, URLs, decisions, and upstream artifacts
Output: exact artifact or decision expected
Boundaries: what is out of scope and what must not change
Dependencies: Work that blocks or consumes this output
Decision rights: what the member decides and what returns to the lead
Checks: deterministic gates and/or judgement criteria
Escalation: what counts as blocked and how to reach the lead
Done: the observable exit condition
```

This is a generated brief or Work-linked artifact, not a universal command API.

## Health signals worth observing before modelling

- duplicate or contradictory Work;
- members reading company-wide state to decide local completion;
- blocker age and whether the lead repairs it;
- missing or stale artifact references at handoff;
- critic agreement without specific evidence;
- lead rewrites that erase specialist value;
- owner interventions below the lead;
- cost and latency added per accepted revision; and
- repeated actor success on comparable outcomes.

These signals can begin in run reports and queryable existing events. Do not create a team-health
subsystem until repeated runs show which signals change decisions.

## Enduring operating rule

> Add an actor only when the work can state what difference or independent capacity that actor buys.
> Keep one lead accountable for synthesis, repair, and the team's answer. Judge the team by the
> accepted outcome and owner load, not by activity or headcount.

