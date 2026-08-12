# T10 · CLI — owner surface *and* the company's coordination interface

**Layer:** Owner surface + OrgIntel — see below. This ticket is more load-bearing than its position in the dependency graph suggests.
**Serves:** Two consumers at once. The owner cannot otherwise issue a directive, watch a run, answer a judgement request, or take over (§7.4). And **the agents have no other way to reach layer 2.**
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T1, T5.

## This is how layers 2 and 3 connect

There is **no gateway between OrgIntel and the runtime, deliberately.** §1 says the layer diagram is "a responsibility and trust map, not a mandatory call chain." §4.7 forbids OrgIntel from gating filesystem writes or requiring work to pass through its API. §16.6: OrgIntel provides *the easiest path, not the only path.* Building a mediating proxy here would recreate the legacy per-turn fence.

Agents reach layer 3 for free — their native file, shell, Git and browser tools execute inside the company container by simple consequence of process placement, not by any routing we build. Layer 2 is reached differently:

```text
 company container (layer 3)
 ├── ACP agent processes (Exec, staff)
 │     ├── native file / shell / git / browser tools ──► act directly on /company
 │     ├── bash tool ──► `restless` CLI ──┐
 │     └── HTTP (base-URL override) ──┐   │
 └────────────────────────────────────│───│──────────────────
                                      │   │  unix socket  ◄── TRUST BOUNDARY
 host                                 │   │
 └── restlessd                        │   │
       ├── model gateway ◄────────────┘   │
       ├── OrgIntel (Postgres) ◄──────────┤   coordination
       └── kernel effect broker ◄─────────┘   external effects
```

Exactly three channels cross the container boundary: **ACP stdio** (control in), **unix socket via this CLI** (coordination and effect requests out), and **HTTP to the gateway** (model). The trust boundary is the socket, not the binary — the CLI is a dumb client and `restlessd` authenticates which company a request came from (§6.1).

**Context comes in via the prompt (T7); coordination goes out via this CLI.** Work itself never touches layer 2 at all.

## Why a CLI rather than an MCP server

The candidates were MCP, CLI over the agent's bash tool, Postgres state projected into files, or parsing agent output. The CLI wins on cost: this ticket already exists, so the marginal work is putting the binary in the image (T1). No new protocol, no new component, no bidirectional sync problem. And **the Exec and the human owner then speak an identical interface to the company** — one surface, two consumers, every improvement helping both.

MCP is the obvious upgrade if this proves clumsy in practice — agent gets syntax wrong, cannot discover commands. Per LLM_CURE.md frame 3, **do not build it until the CLI demonstrably fails**, and then it will be clear what belongs in it.

## Build

Unix socket to `restlessd`. Same binary serves both consumers.

| Command | Does | Used by |
|---|---|---|
| `up` / `down` / `status` | Company environment lifecycle | Owner |
| `tell "<directive>"` | Issue a directive; also how a blocked judgement request is answered | Owner |
| `watch` | Stream the operational event stream | Owner |
| `attach` | Drop into the company shell or browser session | Owner |
| `goals` / `commitments` / `inbox` | Inspect coordination state | Both |
| `commitment complete\|block <id>` | Report coordination state | Agents |
| `message send` | Inbox traffic between actors | Agents |
| `effect request --capability <cap>` | Request an external effect (T8) | Agents |

**Answering a blocked judgement request goes through `tell`**, not a separate command.

## The consequence worth accepting deliberately

Because nothing mediates, **OrgIntel is not authoritative about what actually happened.** It learns via agent reports through this CLI, the operational event stream, and reconciliation against files, Git and processes (§4.8). Its state will sometimes be stale, wrong, or blind — and per §2.5 that is not an incident. A completed artifact does not become invalid because its commitment record disagrees (§4.7).

The soft spot: nothing *forces* an agent to report. Prompt instruction plus the fact that reporting is how it gets more work is all we have, and it will fail sometimes. **Those failures are among the most valuable observations available in this sprint** — record them for T15. The escalation ladder is prompt → playbook → tooling (§4.10). Do not harden this into mandatory mediation; that rebuilds what we deleted.

## Posture (§4.2, CLAUDE.md → product soul)

Outcomes, decisions, risk and next actions first. Roles, prompts, permissions, spend detail and logs only on request. **This is a calm work surface, not an agent administration dashboard** — even in a terminal.

## Acceptance

- A full Cosmon run is driven start to finish from the CLI by the **owner**: directive in, work observable, judgement point answered, artifact located.
- An **agent** completes a commitment, sends a message, and requests an effect through the same binary via its bash tool, with the results visible in OrgIntel.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
