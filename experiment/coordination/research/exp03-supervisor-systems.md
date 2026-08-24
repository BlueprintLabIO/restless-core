# EXP-03 research note — supervisor/worker agent systems

**Status:** External priors frozen before counted EXP-03 calls
**Date checked:** 24 August 2026
**Evidence class:** Research prior only; none of these sources proves Restless works

## Question

What is the smallest strong design for a non-producing model lead that hires, observes, guides,
redirects and judges model workers across real economic work?

## Primary-source findings

### Architecture must follow work shape

[Towards a Science of Scaling Agent Systems](https://arxiv.org/abs/2512.08296) controls model family,
tools, prompts and compute across single-agent and four multi-agent architectures. Its strongest useful
result is conditionality: centralised verification contains error propagation, decomposable financial
reasoning improved substantially, and sequential planning degraded substantially. It also reports
capability saturation and extra overhead on tool-heavy tasks.

**Imported prior:** keep one central accountable supervisor, but vary worker count and boundary by
coupling, sequence, independent evidence and unit independence. Do not search for one universal team
size or topology.

### Orchestrator/worker systems win on independent breadth

Anthropic's
[multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)
uses a lead agent to decompose a question and parallel specialist agents to explore independent
directions. Anthropic reports large latency gains on its breadth-heavy research tasks, but also much
higher token use and weak fit for tightly shared-context coding work. Its delegation guidance names
the objective, output format, tools/sources and boundaries. Subagents return artifacts or references
so the lead need not receive every intermediate token.

**Imported prior:** test breadth and replicated units, not another arbitrary coauthoring team. A worker
brief must include purpose, responsibility, observable completion, exact inputs/tools, constraints and
return surface. Use artifact references instead of transcript relay.

### Supervisors plan, monitor evidence and repair

Microsoft's
[Magentic-One](https://www.microsoft.com/en-us/research/articles/magentic-one-a-generalist-multi-agent-system-for-solving-complex-tasks/)
uses an Orchestrator to plan, assign, track progress and replan when execution stalls. Google's
[AI co-scientist](https://research.google/blog/accelerating-scientific-breakthroughs-with-an-ai-co-scientist/)
uses a supervisor that allocates specialised agents through a worker queue and increases compute where
the problem benefits from it. Both retain central judgement while workers pursue bounded activity.

**Imported prior:** the Restless lead needs commission, inspect, message, redirect, reassign, stop and
accept/reject. It needs material events and native evidence. Restless does **not** import Magentic-One's
exhaustive plan/progress ledgers or create a second workflow engine; sparse Work/Attempt/artifact facts
already cover responsibility and returned evidence.

### Business agents remain fragile outside simple workflow execution

[CRMArena-Pro](https://arxiv.org/abs/2505.18878) contains 19 expert-validated CRM tasks spanning sales,
service and configure/price/quote in B2B and B2C scenarios. Its reported results are much stronger on
single-turn workflow execution than on multi-turn business work, and inherent confidentiality
awareness is poor without targeted instruction.

**Imported prior:** use native business artifacts and hidden checks, not persuasive prose. Include a
customer-operations change cell and keep consequential constraints visible at lead altitude. Frozen
fictional or `_test` inputs cannot validate demand, persuasion or market response.

### GLM-5.3 is suitable but not free

The live [OpenRouter GLM-5.3 page](https://openrouter.ai/z-ai/glm-5.3) and models API identify
`z-ai/glm-5.3` as a text reasoning model with a 1,048,576-token context, tool calling, structured JSON
output, and `low`, `high` and `max` reasoning effort. Reasoning is always on. On 24 August 2026 the
listed prices were $1.40 per million input tokens, $4.40 per million output tokens and $0.26 per million
cached input tokens.

**Imported prior:** use the same GLM-5.3 family for leads and workers to isolate organisation. Live
catalogue claims are admission hints only; a counted programme requires a real gateway inference,
exact tool-written artifact and observed cost. The current Restless credential broker exposes the
first-party `zai/glm-5.3` route, not OpenRouter. EXP-03 therefore records the exact `zai` provider
identity rather than pretending the publicly listed OpenRouter route is authenticated.

## Convergent minimal design

```text
Exec dispatches and leaves the path
  ↓
non-producing supervisor retains mission, constraints and final judgement
  ↓ commissions the smallest useful set
workers own end-to-end outcomes, independent evidence regions or disjoint units
  ↓ return exact artifact references, checks and unresolved uncertainty
supervisor inspects native evidence, redirects/reassigns or accepts
```

Required information:

- mission, exact owner outcome and consequential constraints;
- current native evidence and material changes;
- observed worker capabilities and availability;
- sparse responsibility and dependency facts;
- exact returned artifact/observation references.

Required behaviour:

- lead builds the whole-outcome causal model before delegating;
- worker boundaries close locally or produce independently useful units;
- lead and workers communicate changed information, not activity;
- completion is callback/process observation, not timeout inference;
- lead quiesces while work proceeds and wakes on material events;
- correction remains worker production; lead never silently repairs;
- final acceptance comes from native evidence and fresh checks, not team agreement;
- worker pool expands and contracts with useful independent work.

## Deliberately excluded until a measured failure activates them

- periodic polling or status meetings;
- a second plan/progress ledger;
- full shared transcripts or hidden-reasoning exchange;
- a semantic blackboard or common room;
- peer consensus without one accountable decision owner;
- fixed role counts, fixed span limits or deterministic task routing;
- a new durable workflow engine.

## EXP-03 consequences

1. Supervisor separation is fixed; direct and player-coach runs are diagnostic counterfactuals.
2. Test one whole-outcome worker on coupled work before adding coauthors.
3. Test complementary specialists only on mixed marketing and independent research breadth.
4. Test same-role scale on disjoint sales units.
5. Test material-event intervention on volatile customer operations.
6. Measure lead attention, drift, recovery and hidden lead production—not just wall time.
7. Generalise by work shape, not department name.

## Limits

- Vendor systems are selected examples, not neutral proof that their architecture is optimal.
- Published benchmark tasks differ from Restless's persistent company runtime and authority model.
- The scaling paper predicts relative architectural preference imperfectly; local native outcomes still
  decide.
- Same-family GLM-5.3 actors can make correlated mistakes. Deterministic gates and fresh artifact-only
  review reduce but do not remove that dependence.
