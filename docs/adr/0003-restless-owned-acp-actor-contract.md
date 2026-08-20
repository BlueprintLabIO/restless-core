# ADR 0003: Restless owns the actor contract behind ACP

Status: accepted

## Decision

Keep ACP as the replaceable session transport for Exec and Staff. Do not treat an ACP harness's default agent persona as the company actor.

For every actor launch, the Runtime Bridge supplies:

- a Restless-authored system prompt containing the actor identity, standing operating rules and trusted focused context;
- an explicit native-tool allowlist;
- controlled Restless, company and project skill roots;
- only the MCP servers already selected and authorised for that session;
- the immediate owner message, Work feedback or wake instruction as the ACP user turn.

The concrete harness runs in an isolated profile. Ambient developer configuration, third-party instruction providers, extensions, rules, private subagents and project-discovered MCP servers are disabled unless Restless deliberately adopts them into the launch contract.

## Why

ACP defines session and tool transport but no portable system-prompt field or native tool/skill policy. Previously, Restless sent its entire Exec or Staff briefing as a user message while the harness retained a coding-agent system prompt. The resulting agent correctly followed the higher-priority harness instruction and behaved like a developer doing the work itself, rather than the durable company actor OrgIntel intended.

Owning the launch contract fixes that hierarchy while preserving ACP's useful interoperability. A future harness adapter must prove it can express the same contract; otherwise it is not a compatible actor runtime.

## Consequences

- Exec and every Staff actor use one controlled launch path.
- OrgIntel remains the owner of identity, Work and organisational memory.
- Runtime files remain the owner of project instructions and skills.
- Authority remains the owner of credentials and consequential effects.
- Harness replacement requires a live probe of system instructions, tools, skills and MCP attachment—not merely an ACP handshake.
- A custom Restless agent protocol is unnecessary unless a required control cannot be expressed through a concrete ACP harness adapter.
