# Restless

https://github.com/user-attachments/assets/9e8fa0e4-c8ad-43a1-af92-c7ef9d3602a6

**Run your business with AI. Spend your attention on the work that needs you.**

Restless is an open-source, multiplayer AI workspace for founders and teams.
Humans and AI agents share documents, conversations, ongoing work and a persistent
company computer. Set direction, shape the output together, and make the decisions
that need your judgement. Agents carry the work forward.

**Bring your people. Bring Codex, Claude, or your preferred API provider.**

Research a market. Develop a product. Prepare a sales proposal. Run operations.
Restless brings the work, the people and the tools into one place.

[Why Restless](#why-restless) · [OrgIntel](#orgintel-a-company-that-carries-the-work-forward) ·
[Features](#features) · [Product tour](#product-tour) ·
[Getting started](#getting-started) · [Alternatives](#alternatives) ·
[Architecture](#architecture) · [Contributing](#contributing)

> **Development preview:** a runnable local company workspace under active development.
> Start from a development checkout; interfaces are evolving.

## Why Restless

AI makes it easier to produce work. It can also give you another organisation to manage:
more conversations, handoffs, status reports and decisions competing for your attention.

We believe a business harness should help you **work with AI on the result**. The
measure of success is useful output and the human attention it takes to get there.

### Human multiplayer, from the start

Bring your cofounder, designer, operator or reviewer into the same company. People
participate alongside agents with their own identities and access. Discuss the work
in rooms, edit shared documents, and keep feedback beside the thing you are improving.

Human collaboration is central to Restless. A business can be run by several people,
with AI contributing around them.

### Your attention is part of the budget

A decision should arrive prepared: the context, the recommendation, the relevant
output and what happens next. Leads own their work and resolve routine coordination.
Attention brings you into the places where your judgement or participation matters.

### Collaborate on the output

Open the actual proposal, plan, page or prototype. Revise the document, leave a comment,
ask a question, or take over the company computer. The executive conversation stays
alongside the work, so you can steer without rebuilding context in another tool.

### Start with the smallest useful team

One capable worker can often own a coherent outcome. Add a specialist when it brings
expertise, independent evidence or useful parallel work. Roles establish accountability;
orchestration must justify its cost in the quality of the result.

This principle is grounded in our [coordination experiments](experiment/coordination/experiments/EXP-17/RESULTS.md).
We measure the cost of coordination and use that evidence to shape the system.

### Finish with something you can use

Completion means a reviewable result: a current document, a working page, a file or a
live application. Review targets, revisions and handoffs keep feedback attached to
that result and make the next action clear.

## OrgIntel: a company that carries the work forward

A business has continuity: its commitments, people, knowledge and unfinished work survive
an individual conversation. **OrgIntel — Organisational Intelligence — gives Restless
that continuity.** An agent can change model, restart a session or hand over an assignment
while the company retains the goal, responsibility, evidence and next step.

Its design spans four kinds of work:

| Mode | What it means for the business |
| --- | --- |
| **Explore** | Investigate an uncertain market, compare approaches and decide which evidence would justify investment. |
| **Execute** | Carry an agreed outcome through production, review and delivery with a clear owner. |
| **Repair** | Respond to failed checks, broken tools, lost sessions or changed requirements while preserving useful work. |
| **Evolve** | Turn experience into better company knowledge, examples and reusable practices. |

These modes describe how we want a company to operate. The implementation gives them
concrete foundations: durable goals and actors, scheduled and event-driven wakes,
versioned attempts, review targets, recovery records and governed company identity.

### Responsibility that outlives a model call

Exec holds the portfolio and stays available to you. Each executable outcome has one
accountable lead; a worker owns production. Add researchers, critics or specialists when
their contribution improves the result. Leads can coordinate directly around shared work.

**Accountability does not require a stream of supervisory model calls.** The bounded
single-worker path can route an exact brief and settle passing mechanical checks without
a paid lead paraphrase. Failed evidence, ambiguity, a blocked worker or your changed
direction brings the lead back in. This keeps supervision available where judgement matters.

### Quality is part of the brief

Choose **Fast, Thorough, Exceptional or Frontier** as the company default or for a
particular outcome. The standard guides how deeply agents investigate, develop and
review the result. Your authority and spending limits remain separately controlled.

A review belongs to an exact candidate. Attempts record the inputs they used, checks
run against that version, and changes requested by the reviewer. A revised dependency
can invalidate downstream work that relied on the old result. Play the game, read the
proposal, inspect the page: judge the thing your business will actually use.

### A company that knows what it stands for

Company Identity gives agents four distinct sources of direction:

| Pillar | What it carries |
| --- | --- |
| **Truth** | Approved product facts, claims, evidence and the boundaries of what the company can say. |
| **Voice** | Real writing examples, audience and channel conventions, and room for named human authors to sound like themselves. |
| **Visual language** | References, reusable primitives, accepted and rejected examples, and evidence from rendered work. |
| **Culture** | How the company handles disagreement, uncertainty, correction and customers, grounded in decisions and conduct. |

Work that uses the company identity binds to an approved release and receives a focused brief. Artifacts can
record the specific evidence they depend on. When a fact changes, Restless can identify
the affected artifacts and record your decision to retain, revise or retire them.
Learning proposals carry evidence and go through owner approval.

For example, changing an approved product claim can identify the sales page that still
uses the previous claim. You can commission a focused revision with evidence of what
changed. New work uses the approved correction; existing work keeps its release history.

The [Company Identity run](docs/dogfood/company-identity/s35-run-report.md) exercises this
across two different businesses. Read the [OrgIntel specification](docs/specs/orgintel.md)
for the wider design, and the [capability evidence guide](docs/product-capabilities.md)
for the implementation behind these features.

## Features

### Work together

| Capability | What you can do |
| --- | --- |
| **Human multiplayer** | Work with cofounders and colleagues alongside AI agents, with durable identities, explicit membership and scoped access. |
| **Collaborative documents** | Co-edit rich-text documents with live sync, named versions and exports. Built on **Tiptap, Yjs and Hocuspocus**. |
| **Comments and document review** | Attach feedback to a passage or the whole document, resolve discussions and review revisions against their base version. |
| **Rooms and conversations** | Keep direct and group discussions with an explicit audience of people and agents. |
| **Attention** | Open the decisions, outcome reviews and requests that need your judgement, with the relevant work and evidence attached. |
| **Executive conversation** | Steer the company through a persistent conversation beside the active workspace. |
| **Prepared human handoffs** | Receive a specific action, the prepared state and an observable condition for continuing the work. |
| **Decision continuity** | Follow what your decision unlocked, who now owns the work and whether the result completed or hit another blocker. |

### Run ongoing company work

| Capability | What you can do |
| --- | --- |
| **Durable goals and outcomes** | Keep the business objective, accountable owner and work history across sessions and restarts. |
| **Roles, leads and teams** | Give each outcome one accountable lead and assign independently useful contributions to workers. |
| **Work board and dependency map** | Inspect responsibilities, dependencies, attempts and requested changes in the same company. |
| **Event-driven supervision** | Bring a lead in for material changes, failed evidence or blockers; routine progress can settle without a model turn. |
| **Schedules and follow-ups** | Set one-shot or recurring weekday wakeups, inspect occurrence history and recover skipped work with an explicit missed-run policy. |
| **Focused actor context** | Give each actor relevant goals, messages, decisions, sources and a return path, with durable checkpoints for later sessions. |
| **Evidence-aware memory** | Preserve distinctions between observations, hypotheses, decisions and unknowns, along with source trust and freshness. |
| **Reusable company know-how** | Keep instructions, skills, scripts and internal tools in the company workspace and version them with the work. |

### Set the standard and improve the result

| Capability | What you can do |
| --- | --- |
| **Outcome standards** | Choose Fast, Thorough, Exceptional or Frontier for the company or an individual outcome. |
| **Native outcome review** | Review a working application, document, page or media artifact in its useful form. |
| **Exact attempts and review targets** | Tie execution, input versions, checks and immutable review artifacts to a specific candidate. |
| **Revision and dependency tracking** | Carry requested changes into another attempt and identify downstream work affected by a revised input. |
| **Independent critique** | Give a reviewer the relevant outcome and evidence through a dedicated review context. |
| **Company Identity** | Maintain approved Truth, Voice, Visual Language and Culture as versioned organisational assets. |
| **Identity drift and learning** | Find artifacts bound to superseded evidence; approve proposed learning and decide which outputs need revision. |
| **Recovery with context** | Preserve files and attempt history, classify runtime or provider failures, and supply the next session with the goal, failure evidence and repair context. |

### Give the company a real computer

| Capability | What you can do |
| --- | --- |
| **Persistent Linux workspace** | Use ordinary files, Git, toolchains, packages and applications in a company environment that survives agent sessions. |
| **Embedded desktop** | Enter the company computer, inspect the result and take control to participate directly. |
| **Persistent browser** | Research, inspect live pages and work through browser tools in the company environment. |
| **Internal tools and services** | Build scripts, dashboards, prototypes and local applications with the tools the business needs. |
| **Codex, Claude and API providers** | Connect native agent harnesses, model APIs and compatible gateways. |
| **Per-agent intelligence** | Inherit a company model or choose a connection and model for a particular agent. |
| **CLI and APIs** | Operate the company, inspect work and automate interactions from the terminal. |
| **Multiple companies** | Switch between distinct businesses with their own work, people and company environments. |

### Keep autonomy accountable

| Capability | What you can do |
| --- | --- |
| **Authority and standing grants** | Define what may proceed independently and what needs a separate decision. Deterministic code enforces the boundary. |
| **Membership, roles and permissions** | Keep workspace access, organisational responsibility and authority to cause external consequences distinct. |
| **Budgets and attributable spend** | Set model spend ceilings and inspect usage, request accounting and admission decisions. |
| **Vault — powered by Infisical** | Store credentials through an established secrets backend and keep secret references in company configuration. |
| **Consequential-action receipts** | Record governed external actions with authority, idempotency and execution evidence. Unknown outcomes require reconciliation before repetition. |
| **Resources and Doctor** | Inspect timestamped capability observations and diagnose the runtime, browser, desktop and service connections. |
| **Runtime continuity** | Preserve company work outside individual model processes; external-action history remains separate from the runtime filesystem. |
| **Company floor** | Explore colleagues and teams through an interactive spatial view of the same company. |

[Implementation, interfaces and recorded evidence →](docs/product-capabilities.md)

## Getting started

You need **Rust/Cargo, Node.js/npm, Docker with a running daemon, and curl**.
The first run builds the local daemon and company image.

```sh
git clone --branch dev https://github.com/BlueprintLabIO/restless-core.git
cd restless-core
npm --prefix web install
```

### Choose your intelligence

| Connection | Setup |
| --- | --- |
| **Codex** | Connect the native Codex harness using sign-in or an OpenAI API key. |
| **Claude** | Connect the native Claude harness using sign-in or an Anthropic API key. |
| **API providers** | Use OpenAI, Anthropic, Google Gemini, OpenRouter, Groq, Mistral, DeepSeek, xAI, Moonshot, Z.ai and other supported connections. |
| **Custom endpoints** | Configure an OpenAI-compatible gateway with your endpoint, model IDs and credentials. |

For an API-backed company, choose a model and reference a key already in your shell:

```sh
export RESTLESS_DEV_MODEL=anthropic/claude-sonnet-4-6
export RESTLESS_DEV_CREDENTIAL_REFERENCE=env:ANTHROPIC_API_KEY
./scripts/restless-dev demo_test --reconcile
```

For native sign-in, set `RESTLESS_DEV_MODEL` to your chosen `provider/model`, leave
`RESTLESS_DEV_CREDENTIAL_REFERENCE` unset, and run the same launcher. Open the setup
URL it prints and connect **Codex or Claude** in **Company → Intelligence**.
The new-company button also asks you to choose a starting model before creation.
Existing companies retain their saved configuration.

Open the workspace address printed by the launcher. In another terminal, verify setup:

```sh
./scripts/restless-dev doctor demo_test
```

Ctrl-C stops the foreground host processes; company containers and volumes persist.
See [web development](web/README.md) and [build storage](docs/BUILD_STORAGE.md) for details.

## Example: a game studio, humans and AI together

**Lantern Studio** is the example business in the walkthrough. Its brief: make a small
co-op lighthouse game, find an audience and prepare the first playtest.

| Teammate | Contribution |
| --- | --- |
| Human creative director | Sets direction, co-edits the brief and approves the playable result. |
| Human artist / playtester | Adds references, tests the game and leaves specific feedback. |
| AI production lead | Owns the outcome and brings prepared decisions to the humans. |
| AI developer | Builds and revises the prototype in the company computer. |
| AI researcher or QA specialist | Contributes audience research or independent testing when useful. |

1. **Shape the brief together.** Agree on the experience, scope and what makes it good.
2. **Make the work.** Build the prototype, prepare the audience plan and draft the invitation.
3. **Review the actual result.** Play the game and leave feedback beside the build and brief.
4. **Revise and decide.** Improve the confusing parts, review the next version and approve the playtest.

The same loop works for a consultancy preparing a client proposal, a founder researching
a new market, or an operations team improving an internal process. The business changes;
the shared workspace and the way people collaborate with AI stay familiar.

## Product tour

Real development UI, including the Lantern Studio example. Screenshots and video share
a **1920 × 1080 (16:9)** frame. [Capture details and captions](docs/media/README.md).

### Write and revise together

Native documents bring the brief, working draft, comments and version history into one
surface. Live sync keeps collaborators on the same document.

![Shared creative brief in the native document editor](docs/screenshots/restless-documents.jpg)

### Keep feedback attached to the result

Discuss a document or a specific passage, resolve comments, and carry the requested
changes into the next version. Review happens in the workspace where the work lives.

![Document review with a saved comment and reply controls](docs/screenshots/restless-document-review.jpg)

### Share a room with the people and agents involved

Direct and group rooms give ongoing discussions an explicit audience. Keep direction,
questions and feedback together while documents and the company computer hold the work.

![Studio room with participant controls and shared discussion](docs/screenshots/restless-rooms.jpg)

### Enter the company computer

Agents work in a persistent Linux environment with a browser, files and applications.
Open its desktop inside Restless, inspect the result, and **Take control** when you
want to participate directly. Here, the studio's playable prototype is open in the computer.

![Playable prototype inside the embedded company computer](docs/screenshots/restless-computer-game.jpg)

### Set authority and permissions

Choose what the company may do independently and which decisions require you.
**Authority & limits** brings autonomy, outcome standards, model spend ceilings and
standing grants into one visible surface.

![Authority boundaries, owner decisions and model spend ceiling](docs/screenshots/restless-authority.png)

### Give roles clear accountability

People connects each role to its current work and accountable lead. See who is producing,
who owns the outcome and where to take a question.

![People view showing roles, accountable leads and work](docs/screenshots/restless-roles.png)

### Choose intelligence per agent

A researcher, lead and reviewer can use different models and connections. Inherit the
company choice or assign intelligence directly beside the person's role.

![Per-agent connection and model assignment](docs/screenshots/restless-agent-models.png)

### Connect your preferred provider

Native Codex and Claude connections sit alongside API providers and compatible gateways.
API connections can store credentials in Infisical and keep only references in company settings.

![API provider setup and secure credential storage](docs/screenshots/restless-providers.png)

### Know what the company can use

**Resources & access** reports available capabilities with their observed state, source
and timestamp. Doctor checks the execution path so you can find and fix a broken boundary.

![Resource availability and timestamped evidence](docs/screenshots/restless-resources.png)

![Doctor showing runtime, browser and desktop checks](docs/screenshots/restless-doctor.png)

### Keep credentials in a real vault

**Vault is based on [Infisical](https://github.com/Infisical/infisical).** Restless uses
its secret storage and machine identity through a host-side adapter. The owner can
inspect credential inventory without displaying secret values. Restless owns the
authority checks and controls how credentials reach the relevant provider or harness.

![Infisical connection and company credential inventory](docs/screenshots/restless-vault.png)

### Follow outcomes and revisions

Use the work board for current state or the dependency map for relationships between
outcomes. Keep the result, accountable lead and revision history connected.

![Work board with outcome states](docs/screenshots/restless-work.png)

### See the company at a glance

The company floor is an interactive view of colleagues and teams, connected to the
same work available through Attention, Work, People and Company.

![Interactive company floor](docs/screenshots/restless-office.png)

## Alternatives

### Why choose Restless over Paperclip?

**Choose Restless when you want to run the business alongside AI, with your team
working directly on the output.**

[Paperclip](https://github.com/paperclipai/paperclip) organises an agent company through
roles, goals, issues, budgets and agent runtimes. It is a strong fit when you want that
agent-management model. Its [issue workflow](https://docs.paperclip.ing/guides/day-to-day/issues/)
lets a CEO agent create and delegate work, with humans reviewing progress and approvals.

Restless starts from a different question: **what useful work can we complete together,
and where does human judgement improve it?**

- **Participate in the result.** Your cofounder can revise the brief, a colleague can
  comment on the proposal, and you can enter the company computer to try the build.
- **Make attention purposeful.** Bring the relevant output and a prepared decision to
  the owner. Keep routine coordination with the accountable lead.
- **Keep orchestration proportionate.** Start with a capable worker. Add collaborators
  for a specific contribution to quality, evidence or speed.
- **Carry company identity into the work.** Approved facts, voice, visual language and
  culture become versioned context with traceable dependencies in the outputs.
- **Make quality an operating choice.** Set an outcome standard, review the exact result
  and carry feedback into the next attempt.
- **Keep business context together.** Documents, rooms, work, roles and the computer
  belong to the same ongoing company.

Paperclip also offers [human membership](https://docs.paperclip.ing/guides/org/members-and-access/),
[ranked attention](https://docs.paperclip.ing/reference/api/attention/) and
[versioned issue documents](https://docs.paperclip.ing/guides/day-to-day/issues/).
The choice is the working relationship you want: Restless puts collaborative production
and the owner's attention at the centre of its design.

### Features at a glance

**✓** Built-in capability · **◐** Related capability or a route through configuration,
workflows or integrations · **—** Not a native feature ·
**?** Equivalent capability not established in the sources.
Checks describe documented mechanisms, not comparative output quality. Availability can
vary by plan, adapter or rollout. Sources checked **22 September 2026**.

#### Human collaboration and the working surface

| Feature | Restless | Paperclip | OpenClaw | Lindy | n8n | Dify |
| --- | :---: | :---: | :---: | :---: | :---: | :---: |
| General business work beyond coding | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Shared human workspace / membership | ✓ | ✓ | ◐ | ✓ | ✓ | ✓ |
| Multiple agents or delegated work | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Human–agent group conversation | ✓ | ◐ | ✓ | ✓ | ◐ | ◐ |
| Native editable documents or files | ✓ | ✓ | ✓ | ✓ | ◐ | ◐ |
| Live collaborative rich-text documents | ✓ | ◐ | ? | ◐ | ◐ | ◐ |
| Built-in attention / decision feed | ✓ | ✓ | ◐ | ✓ | ◐ | ◐ |
| Artifact viewing or interactive output | ✓ | ✓ | ✓ | ✓ | ◐ | ✓ |
| Embedded desktop with human interaction | ✓ | ◐ | ✓ | ? | ◐ | ◐ |
| CLI or programmatic API access | ✓ | ✓ | ✓ | ◐ | ✓ | ✓ |

#### Continuing work and agent execution

| Feature | Restless | Paperclip | OpenClaw | Lindy | n8n | Dify |
| --- | :---: | :---: | :---: | :---: | :---: | :---: |
| Company goals and accountable organisational roles | ✓ | ✓ | ◐ | ◐ | ◐ | ◐ |
| Persistent agent identity or session memory | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Scheduled work and recurring routines | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Reusable instructions, skills or playbooks | ✓ | ✓ | ✓ | ✓ | ◐ | ◐ |
| Failure recovery or retry mechanisms | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Bring Codex / Claude agent runtimes | ✓ | ✓ | ✓ | ? | ◐ | ◐ |
| Multiple model providers | ✓ | ✓ | ✓ | ◐ | ✓ | ✓ |
| Configure intelligence per agent / workflow node | ✓ | ✓ | ✓ | ◐ | ✓ | ✓ |
| General visual workflow editor | — | ? | ? | ✓ | ✓ | ✓ |
| Self-hostable core | ✓ | ✓ | ✓ | ? | ✓ | ✓ |

#### Quality, company identity and authority

| Feature | Restless | Paperclip | OpenClaw | Lindy | n8n | Dify |
| --- | :---: | :---: | :---: | :---: | :---: | :---: |
| Human approval / permission controls | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Review feedback carried into further work | ✓ | ✓ | ◐ | ◐ | ✓ | ✓ |
| Model usage / cost visibility | ✓ | ✓ | ✓ | ✓ | ◐ | ✓ |
| Enforced company / agent model-spend budgets | ✓ | ✓ | ? | ◐ | ◐ | ◐ |
| Exact artifact, input and revision lineage for outcome review | ✓ | ◐ | ? | ? | ◐ | ◐ |
| Company / outcome quality standards beyond model selection | ✓ | ◐ | ◐ | ◐ | ◐ | ◐ |
| Approved truth, voice, visual and culture releases bound to work | ✓ | ◐ | ◐ | ◐ | ◐ | ◐ |
| Artifact drift traced to superseded company-identity evidence | ✓ | ? | ? | ? | ? | ? |
| Secret storage / credential management | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Permissioned external tools / integrations | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

**The distinctions behind the ticks:**

- **Collaboration:** Paperclip has versioned issue documents and artifact previews;
  OpenClaw has file, browser and capability-dependent desktop panels; Lindy has team
  files and hosted artifacts. Restless combines a native shared editor, rooms and the
  company computer around the same ongoing work.
- **Continuity:** schedules, memory and recovery are common strengths across this
  category. Restless gives them company-level responsibility, attempt history and
  prepared human handoffs. Its lead can remain accountable without a routine model turn.
- **Quality and identity:** an alternative's ◐ credits related prompts, skills, knowledge
  or workflow mechanisms. Restless supplies named outcome standards and a specific
  identity-release, evidence-binding and drift mechanism in Core.
- **Automation:** n8n, Dify and Lindy provide visual workflow building. Restless's work
  graph describes outcomes and revisions; reusable automation lives in company tools
  and scripts. It is not a general drag-and-drop workflow editor.

The [comparison notes and sources](docs/product-capabilities.md#comparison-notes-and-sources)
explain the scope of these rows. Corrections are welcome through an issue or PR.

### When another tool is a better fit

| Alternative | Choose it for |
| --- | --- |
| [Paperclip](https://github.com/paperclipai/paperclip) | Managing an agent organisation through tasks, roles, budgets and bring-your-own runtimes. |
| [OpenClaw](https://github.com/openclaw/openclaw) | A general assistant across devices, messaging channels and extensible tools. |
| [Lindy](https://docs.lindy.ai/teammate/home) | A hosted business assistant connected to email, meetings and team tools. |
| [Manus](https://manus.im/desktop) | Delegating general tasks with computer-based execution. |
| [Relevance AI](https://relevanceai.com/workforce) | Configuring specialist agents and workflows into a workforce. |
| [n8n](https://github.com/n8n-io/n8n) | Explicit, repeatable automation with visual workflows and integrations. |
| [Dify](https://github.com/langgenius/dify) | Building AI applications, retrieval pipelines and agentic workflows. |
| [OpenHands](https://github.com/OpenHands/OpenHands) / [Gas Town](https://github.com/gastownhall/gastown) | Software engineering and coordinating coding agents. |
| [CrewAI](https://github.com/crewAIInc/crewAI) / [LangGraph](https://github.com/langchain-ai/langgraph) / [Microsoft Agent Framework](https://github.com/microsoft/agent-framework) | Building your own agent application and controlling its execution model. |

## Architecture

Restless separates three responsibilities:

| Layer | Responsibility |
| --- | --- |
| **Constitutional Kernel** | Authority, credentials, budgets, external effects and recovery. |
| **Organisational Intelligence (OrgIntel)** | Goals, accountable work, actors, context, review, company identity and coordination. |
| **Company Linux Runtime** | The persistent computer where agents use tools, edit files and produce results. |

The kernel bounds consequential actions. OrgIntel coordinates the work. The runtime
provides the environment to do it.

### Built on established tools

| Foundation | Role in Restless |
| --- | --- |
| [Infisical](https://github.com/Infisical/infisical) | Vault storage and machine identity, behind Restless's credential adapter. |
| [Tiptap](https://github.com/ueberdosis/tiptap), [Yjs](https://github.com/yjs/yjs), [Hocuspocus](https://github.com/ueberdosis/hocuspocus) | Rich-text editing, shared document state and collaboration transport. |
| [PostgreSQL](https://www.postgresql.org/) | Durable company, work and collaboration state. |
| Linux and Docker / OCI | Persistent company execution environments, files, processes and applications. |
| Rust | Daemon, CLI, coordination and model gateway. |
| SvelteKit | The human workspace: Attention, Work, People and Company. |
| Codex and Claude | Native agent harnesses, alongside Restless-managed API execution. |

Read [ARCHITECTURE.md](ARCHITECTURE.md) and [coordination theory](docs/COORDINATION_THEORY.md)
for the responsibility boundaries and team design.

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
