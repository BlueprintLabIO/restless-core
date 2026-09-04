# T0 — Freeze the Certified Harness Contract and Admit the Claude Route

**Serves:** Sprint 39 — Work Through Attention With Verified Agent Harnesses  
**Layer:** Runtime Bridge + Authority + Release  
**Depends on:** Sprint 11, Sprint 17, current OMP ACP and native Codex probes

## Outcome

The team has one executable acceptance corpus and an evidence-backed go/no-go decision for Claude Agent before shared runtime code is generalized around guesses.

## Work

- Capture the current Restless Managed and Codex launch manifests, readiness signals, events, usage, cancellation, session storage, and cleanup behavior as the baseline.
- Freeze the semantic harness contract from the sprint spec: launch, scope, policy, readiness, capabilities, input, events, permissions, usage, terminal outcome, recovery, and cleanup.
- Pin the candidate `claude-agent-acp` package and integrity, then record the Claude Code/Agent binary, Node runtime, ACP version, license, and distribution chain actually executed.
- Prove the adapter can receive Restless-authored system context, exact cwd, model, effort, tools, and MCP configuration without ambient augmentation.
- Prove an API-authenticated provider path in which the Runtime Plane does not receive an Anthropic root key or reusable subscription credential. Determine whether the current gateway suffices or requires a narrow Anthropic-compatible scoped relay.
- Confirm the permitted owner-facing product name and record the release copy as **Claude Agent**.
- Turn the proof cases into a checked-in harness acceptance corpus reusable by every later ticket.
- Record explicit admission decisions for required and optional capabilities. Do not infer capability support from executable presence alone.

## Acceptance

- A checked-in matrix names every required semantic outcome and its evidence source for Restless Managed, Codex, and Claude Agent.
- Exact adapter and bundled agent build identities are pinned and reproducible.
- A real Claude turn completes through the proposed credential route with company/session attribution and without a reusable provider credential in Runtime environment, argv, config, logs, or process inspection.
- Ambient user settings, plugins, hooks, MCP servers, skills/subagents, and approval caches are tested explicitly.
- The team records `admit`, `admit with guarded optional gaps`, or `reject` for Claude Agent.
- A rejection stops Claude implementation rather than introducing a raw-key or permissive fallback.

## Makes Deletable

- Ad hoc manual harness checks.
- Unpinned `latest` installations.
- Prototype raw-key launch paths.
- Assumptions that ACP handshake success equals Restless support.

## Stop

Stop and return to founder alignment if the adapter requires a product-policy fork, credentials inside Runtime, or distribution terms incompatible with the intended integration.
