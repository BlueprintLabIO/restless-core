# Sprint 06 research dossier: skills, expertise, and powerful teams

**Research date:** 17 August 2026  
**Status:** Research only. This dossier does not change the Sprint 06 contract or add implementation
scope.

## The short answer

Restless should not try to build a powerful team by installing the most popular skills or by adding
more agents. A reliable team needs five separate things:

1. a real outcome and work shape;
2. one accountable lead;
3. members that contribute genuinely different role, model, context, evidence, or tools;
4. a coordination pattern suited to the dependencies in the work; and
5. a closed evaluation loop against the finished outcome.

A skill can improve how an actor performs a known workflow. It cannot prove that the actor has
expertise, make a badly shaped team coherent, provide tool authority, or demonstrate that the work is
good. A registry is a discovery source, not a hiring system.

The practical recommendation is therefore:

- use the open Agent Skills directory format as a portable runtime convention;
- treat public registries as untrusted lead lists;
- pin and inspect a skill before testing it in a `_test` company;
- attach expertise claims to observed outcomes, not role names or install counts;
- let the accountable lead compose the smallest team that fits the current Work graph; and
- keep the full teamwork-pattern recommender out of Sprint 06, as the sprint spec already requires.

## What each document covers

- [01-skill-standards-and-registries.md](01-skill-standards-and-registries.md) distinguishes skills,
  tools, actors, knowledge, and registries, and reviews the current ecosystem.
- [02-skill-trust-quality-and-evaluation.md](02-skill-trust-quality-and-evaluation.md) proposes a
  skeptical adoption and evaluation process.
- [03-team-composition-and-delegation.md](03-team-composition-and-delegation.md) derives a compact
  theory of team formation from agent research and human-team evidence.
- [04-frontend-design-team-case-study.md](04-frontend-design-team-case-study.md) applies the findings
  to the recent Aris landing-page failure.
- [05-orgintel-implications.md](05-orgintel-implications.md) states what the research should change in
  our thinking now, what to test in Sprint 06, and what not to build yet.

## Vocabulary that must stay separate

| Concept | What it is | What it is not |
|---|---|---|
| **Skill** | Versioned procedural instructions, references, assets, and sometimes scripts | Proof of expertise or authority |
| **Tool** | A capability to observe or act, such as a browser, CLI, MCP server, or SDK | Guidance on how to do a job well |
| **Actor** | A durable organisational identity with a role, history, model, context, and sessions | A prompt assembled for one Work item |
| **Expertise** | A claim supported by relevant, repeated outcome evidence | A role label or installed skill |
| **Team pattern** | A relationship between actors suited to a work shape | A fixed roster or workflow engine |
| **Knowledge** | Current facts, doctrine, examples, and lessons available from files or systems | An ever-growing prompt or universal ontology |
| **Authority** | Permission to create a consequential external effect | Anything a skill declares it is allowed to do |

This separation follows the existing Restless architecture: OrgIntel owns work and coordination, the
Company Runtime owns files and tools, and the Kernel owns authority and effects.

## How sources were judged

The notes deliberately avoid treating every online claim equally.

| Evidence class | Useful for | Main limitation |
|---|---|---|
| Official specification or protocol documentation | What a format or registry currently guarantees | Does not prove effectiveness |
| Controlled study or meta-analysis | Direction and boundary conditions | Benchmarks and human teams do not transfer perfectly to Restless |
| Vendor production report | Practical failure modes and operating lessons | Selective, product-specific, usually not independently reproduced |
| Marketplace telemetry or badges | Popularity and provenance clues | Install count is not quality; provenance is not performance |
| Recent preprint | Emerging risks and hypotheses | Not yet peer reviewed or independently replicated |
| Community advice | Leads and vocabulary | Anecdotal until tested |

The test used throughout is: what exactly does this source demonstrate, what does it leave unknown,
and what evidence would make the claim false in a real Restless company run?

## Enduring conclusions versus current ecosystem facts

The current packaging standard and registries will change. The following conclusions are more likely
to endure:

- Extra coordination has a cost, so a team must buy either useful difference or parallelism.
- Sequential dependencies reduce the value of adding agents.
- One accountable synthesiser limits ambiguity and error propagation.
- Instructions are only reliable when grounded in the actual environment and checked outcome.
- Knowledge should be loaded for the job, not copied wholesale into every actor's context.
- Independent criticism requires real information or judgement separation.
- A repeated, accepted outcome is stronger evidence of expertise than self-description.
- Debriefs improve future work only when they examine objective evidence and change the next attempt.

Those principles are the basis for the OrgIntel recommendations in this dossier.

