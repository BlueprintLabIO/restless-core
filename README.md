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

[Why Restless](#why-restless) · [Features](#features) · [Product tour](#product-tour) ·
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

## Features

| Capability | What you can do |
| --- | --- |
| **Human multiplayer** | Work with cofounders and colleagues alongside AI agents, with explicit membership and scoped access. |
| **Collaborative documents** | Edit shared rich-text documents with live sync, versions, anchored comments and review. Built on **Tiptap, Yjs and Hocuspocus**. |
| **Rooms and conversations** | Keep direct and group discussions with the people and agents involved in the work. |
| **Attention** | Review prepared decisions, collaborate on live work and handle requests that need your judgement. |
| **Executive conversation** | Steer the company with a persistent conversation beside the active workspace. |
| **Outcome and revision tracking** | Follow accountable work, dependencies, attempts and requested changes through a board or dependency map. |
| **Roles and leads** | See who owns the outcome, who contributes and where responsibility sits. |
| **Per-agent intelligence** | Use a company model or assign different connections and models to individual agents. |
| **Codex, Claude and API providers** | Connect native agent harnesses, model APIs and compatible gateways. |
| **Company computer** | Run tools, edit files and use applications in a persistent Linux environment. Enter the embedded desktop and take control. |
| **Browser** | Research, inspect live pages and work through browser-based tools inside the company environment. |
| **Authority and permissions** | Set autonomy boundaries, standing grants, owner decisions and outcome standards. |
| **Budgets and spend** | Set model spend ceilings and inspect model usage and admission decisions. |
| **Vault — powered by Infisical** | Store API credentials through an established secrets backend; company settings keep references to secrets. |
| **Schedules** | Give ongoing work exact or recurring wakeups, with accountable ownership and occurrence history. |
| **External-action records** | Inspect consequential effects and their receipts separately from ordinary internal work. |
| **Resources and Doctor** | Inspect timestamped resource availability and diagnose the runtime, browser, desktop and service connections. |
| **CLI and APIs** | Operate the company, inspect work and automate routine interactions from the terminal. |
| **Multiple companies** | Keep distinct businesses in separate company environments and switch between them. |
| **Company floor** | Explore colleagues and teams through an interactive spatial view of the same company. |

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
- **Keep business context together.** Documents, rooms, work, roles and the computer
  belong to the same ongoing company.

Paperclip also offers [human membership](https://docs.paperclip.ing/guides/org/members-and-access/),
[ranked attention](https://docs.paperclip.ing/reference/api/attention/) and
[versioned issue documents](https://docs.paperclip.ing/guides/day-to-day/issues/).
The choice is the working relationship you want: Restless puts collaborative production
and the owner's attention at the centre of its design.

### Features at a glance

**✓** Built in · **◐** Related capability, integration or configuration required ·
**?** Equivalent capability not established in the linked documentation.
Team and enterprise features may depend on the plan. Sources checked September 2026.

| Feature | Restless | Paperclip | OpenClaw | Lindy | n8n | Dify |
| --- | :---: | :---: | :---: | :---: | :---: | :---: |
| General business work beyond coding | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Shared human workspace / membership | ✓ | ✓ | ◐ | ✓ | ✓ | ✓ |
| Multiple agents or delegated work | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Native editable documents or files | ✓ | ✓ | ✓ | ✓ | ◐ | ◐ |
| Live collaborative rich-text documents | ✓ | ◐ | ? | ◐ | ◐ | ◐ |
| Built-in attention / decision feed | ✓ | ✓ | ◐ | ✓ | ◐ | ◐ |
| Embedded desktop with human interaction | ✓ | ◐ | ✓ | ? | ◐ | ◐ |
| Bring Codex / Claude agent runtimes | ✓ | ✓ | ✓ | ? | ◐ | ◐ |
| Multiple model providers | ✓ | ✓ | ✓ | ◐ | ✓ | ✓ |
| Human approval / permission controls | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Visual workflow builder | ◐ | ◐ | ◐ | ✓ | ✓ | ✓ |
| Self-hostable core | ✓ | ✓ | ✓ | ? | ✓ | ✓ |

**How to read the differences:** Paperclip's documents live with issues; Lindy's team
files are versioned and editable by owners/admins; OpenClaw has file, browser and
capability-dependent desktop panels. n8n and Dify centre collaboration on workflows
and apps. Restless centres it on the business work itself. Its work graph tracks
outcomes and revisions rather than serving as a general visual automation builder.

Sources: [Paperclip features](https://github.com/paperclipai/paperclip),
[Paperclip issues and documents](https://docs.paperclip.ing/guides/day-to-day/issues/),
[Paperclip attention](https://docs.paperclip.ing/reference/api/attention/),
[OpenClaw](https://github.com/openclaw/openclaw) and [panels](https://docs.openclaw.ai/web/control-ui/panels),
[Lindy home](https://docs.lindy.ai/teammate/home), [files](https://docs.lindy.ai/teammate/files)
and [workflow approvals](https://www.lindy.ai/blog/human-in-the-loop-automation),
[n8n](https://github.com/n8n-io/n8n), [Dify](https://github.com/langgenius/dify).
Corrections are welcome through an issue or PR.

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
| **Organisational Intelligence (OrgIntel)** | Goals, accountable work, actors, conversations and coordination. |
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
