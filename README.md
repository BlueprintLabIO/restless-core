# Restless

**Run your business with AI. Spend your attention on the work that needs you.**

Restless is an open-source AI workspace for founders and owner/operators. It brings
executive chat, documents, ongoing work, and a persistent company computer together.
You set direction, collaborate on the output, and make the decisions that need your
judgement. Agents carry the work forward.

The goal is simple: **more useful business output for less of your attention.**

[Why Restless](#why-restless) · [Features](#features) · [Getting started](#getting-started) ·
[Alternatives](#alternatives) · [Architecture](#architecture) · [Contributing](#contributing)

> **Development preview.** Restless Core is an active rebuild with a runnable local
> company appliance, not yet a production release. Native document collaboration is
> implemented, but agent editing, review integration, and deployment verification are
> still being completed. Expect a developer setup and changing interfaces.

## Why Restless

AI can do a growing share of business work. Coordinating it can become a job of its own:
repeating context, moving drafts between tools, checking whether something is actually
finished, and working out which interruption needs a decision.

Restless starts with the owner's experience. The main surface holds the work; an
executive conversation stays alongside it. The system should bring you a useful result,
a prepared decision, or the exact place where your participation is needed.

### Your attention is part of the budget

A decision should arrive with context: what happened, why it matters, a recommendation,
and what happens if you do nothing. Work assigned to a lead should stay with that lead
until it genuinely needs the owner. Internal activity is not automatically a reason to
interrupt you.

### Collaboration belongs beside the output

Business work involves revising a proposal, questioning a source, changing a plan, or
working through a decision together. Restless puts documents, conversation, review and
execution in the same workspace. Documents have versions and anchored comments;
revision feedback stays connected to the work.

### Orchestration has to earn its cost

Restless uses persistent roles and accountable leads internally. The default production
shape is one capable worker for a coherent outcome, with additional workers when
specialisation, independent evidence or parallel work justify the coordination.

We test that assumption. In a [small internal experiment](experiment/coordination/experiments/EXP-17/RESULTS.md),
extra supervision produced similar assessed quality across four paired tasks while
adding substantial time and cost. That informs the design; it is not a benchmark
against other products or a claim that collaboration never helps.

### Completion means something you can review

The review surface should be the current document, live page, file or other usable
result. Restless tracks review targets, checks availability, rejects stale review
state, and carries requested changes into the next revision. When you need to step in,
the handoff records the exact action, prepared state and condition for resuming.

## Features

These are components in the current codebase. The development-preview status above
applies to the complete experience.

| Surface | What it is for |
| --- | --- |
| **Executive chat** | Set direction, discuss the business, and steer work from a conversation alongside the main surface. |
| **Attention** | Review prepared decisions, approvals and human handoffs with context and recommended action. |
| **Work** | Follow outcomes, inspect the current result, and request revisions with durable feedback. |
| **Documents** | Edit native documents with a collaborative editor, versions, anchored comments and access controls. End-to-end agent collaboration remains in progress. |
| **People and conversations** | Keep accountable roles and durable discussion context around ongoing work. |
| **Browser and computer** | Work in a persistent Linux environment with files, applications and a browser; use the owner surface to take control when needed. |
| **CLI** | Operate the local company appliance and inspect its state from the terminal. Command coverage is evolving. |

The intended scope includes research, sales preparation, planning, operations and
software development. A useful first evaluation is a bounded business brief: research a
customer segment, draft an offer with sources, revise it with the owner, and prepare
the next action. Judge the result and the supervision required, not the number of
agents involved.

## Getting started

The current entry point is a **local development checkout**. You need Rust/Cargo,
Node.js/npm, Docker with a running daemon, curl, and credentials for the configured
model provider. The development script builds the daemon and company image, so the
first run is substantial.

```sh
git clone https://github.com/BlueprintLabIO/restless-core.git
cd restless-core
npm --prefix web install
./scripts/restless-dev --help
```

Configure model credentials in your shell before starting. The script currently defaults
to `zai/glm-5.3` using `ZAI_API_KEY`. `RESTLESS_DEV_MODEL` and
`RESTLESS_DEV_CREDENTIAL_REFERENCE` select the model and credential reference for a newly
created development company; other routes require the corresponding provider setup.
See [the launcher](scripts/restless-dev) for the current behaviour.

```sh
# First run: build the company image and start an isolated development profile.
./scripts/restless-dev demo_test --reconcile

# In another terminal, from the same checkout:
./scripts/restless-dev doctor demo_test
```

Open the address printed by the launcher; ports are specific to the checkout. Ctrl-C
stops its foreground host processes. Company containers and volumes persist. A rendered
page alone is not a connected appliance: use the doctor check to inspect the local path.

See [web development](web/README.md) and [build storage guidance](docs/BUILD_STORAGE.md)
for more detail. This is a development path, not a verified one-command production install.

## Alternatives

There are good reasons to choose another tool. Restless shares capabilities with many
of these projects; its emphasis is the combination of founder attention, collaboration
on deliverables, and proportionate orchestration.

This comparison describes product approaches, not a performance ranking. Links point
to the projects' own descriptions. Features change; please open an issue or PR if a
comparison is inaccurate.

| Tool | Consider it when… | How Restless differs in emphasis |
| --- | --- | --- |
| [Paperclip](https://github.com/paperclipai/paperclip) | You want to manage an agent organisation through roles, goals, issues, budgets and bring-your-own runtimes. | Restless starts from the owner's conversation, attention and collaborative work surface. Both target business operations and retain internal accountability. |
| [OpenClaw](https://github.com/openclaw/openclaw) | You want a general assistant across your devices and communication channels, with extensible tools and agent routing. | Restless centres an ongoing business workspace and prepared decisions. Browser, terminal and file capabilities overlap. |
| [Lindy](https://docs.lindy.ai/teammate/home) | You want a hosted business teammate working through connected tools and routines. | Restless Core provides a local company appliance with an inspectable implementation. Lindy also documents attention cards and editable files. |
| [Manus](https://manus.im/desktop) | You want to delegate general tasks to an agent with computer-based execution. | Restless emphasises continuing business context, accountable work and collaboration around review. General execution is shared territory. |
| [Relevance AI](https://relevanceai.com/workforce) | You want to configure specialist agents and connect them into a workforce. | Restless aims to reduce the workforce design the owner must do before getting useful work. |
| [n8n](https://github.com/n8n-io/n8n) | You can describe a repeatable process and want explicit workflows and integrations. | Restless focuses on work whose approach changes through judgement, discussion and revision. Explicit workflows can be the clearer choice for stable automation. |
| [Dify](https://github.com/langgenius/dify) | You want to build AI applications with workflows, knowledge retrieval and model tooling. | Restless is the business workspace itself, rather than a platform for building one. |
| [OpenHands](https://github.com/OpenHands/OpenHands) / [Gas Town](https://github.com/gastownhall/gastown) | Your primary work is software engineering and coordinating coding agents. | Restless treats non-code business deliverables as primary work alongside software. |
| [CrewAI](https://github.com/crewAIInc/crewAI) / [LangGraph](https://github.com/langchain-ai/langgraph) / [Microsoft Agent Framework](https://github.com/microsoft/agent-framework) | You need to build your own agent application and control its execution model. | Restless provides an opinionated owner experience. Frameworks offer more freedom to build a different one; their applications are not limited to coding. |

### A closer look at Paperclip

Paperclip is the closest comparison in business scope. Its explicit company and task
model may be exactly what you want when managing a roster of agents and their work.

Restless is for the owner who wants to spend more time directing and improving outcomes
and less time managing the organisation that produces them. That is a design choice
we need to prove through the experience, not a claim that Paperclip lacks the features:
it has a [ranked attention API](https://docs.paperclip.ing/reference/api/attention/),
[native issue documents](https://docs.paperclip.ing/guides/day-to-day/issues/), and an
[experimental conversational task layout](https://docs.paperclip.ing/experimental/task-chat/).

Choose on the workflow you prefer and the results you get. We do not yet have a
controlled comparison demonstrating that Restless produces better outputs or requires
less supervision.

## Architecture

Restless separates three responsibilities:

| Layer | Responsibility |
| --- | --- |
| **Constitutional Kernel** | Authority, credentials, budgets, external effects and recovery. |
| **Organisational Intelligence (OrgIntel)** | Goals, accountable work, actors, conversations and coordination. |
| **Company Linux Runtime** | The persistent computer where agents use tools, edit files and produce results. |

The kernel bounds consequential actions. OrgIntel coordinates the work. The runtime
provides the environment to do it. These are responsibility boundaries, not a mandatory
sequence of model calls.

Read [ARCHITECTURE.md](ARCHITECTURE.md) for the design and
[coordination theory](docs/COORDINATION_THEORY.md) for the current reasoning behind team
shape. Architecture documents include intended behaviour; code and observed runs
establish what has been delivered.

## Contributing

Start with the [working agreement](CLAUDE.md), [architecture](ARCHITECTURE.md), and the
relevant [sprint](docs/sprints/README.md). The most useful contributions close a real
workflow gap and show the resulting output or behaviour.

| Path | Contents |
| --- | --- |
| `crates/restlessd/` | Coordination daemon and owner APIs |
| `crates/restless/` | Operator CLI |
| `crates/restless-orgintel/` | Recoverable company and work state |
| `crates/restless-model-gateway/` | Model routing and spend accounting |
| `infra/company-image/` | Persistent company computer |
| `services/native-documents-collaboration/` | Document collaboration service |
| `web/` | Owner workspace |
| `experiment/` | Experiment designs and recorded results |

The hosted control plane and public website live in a separate private repository.
This repository contains the local company core, architecture and experiment evidence.

## License

Restless Core is licensed under [Apache 2.0](LICENSE). The Restless name, marks,
visual identity, hosted service and private cloud control plane are not licensed by
this repository.
