# T6 — Close Codex Coordination and Connected-Tool Parity

**Serves:** Sprint 39 — Work Through Attention With Verified Agent Harnesses  
**Layer:** Native Codex Runtime + Authority + Runtime Bridge  
**Depends on:** T1, T4

## Outcome

Codex remains a first-party native integration while satisfying the same Restless coordination, connected-tool, usage, cancellation, and recovery outcomes required of the ACP harnesses.

## Work

- Remove the current blanket restriction that confines Codex to productive Staff Attempts once the equivalent coordination contract is proven.
- Launch Exec and producing-lead conversations through the existing Codex App Server path with exact Restless system/actor/work context.
- Pass only Restless-approved MCP servers and external-effect authority; do not let native Codex configuration or remembered approvals augment the launch.
- Normalize conversation text, tool activity, permissions, plan updates, usage, errors, and terminal outcome at the common semantic boundary.
- Use native in-flight steering only when the active App Server session acknowledges it. Preserve the shared next-turn queue fallback.
- Prove scoped session continuation/reconstruction, model validation, cancellation, process cleanup, and runtime restart behavior.
- Keep Codex-specific capabilities as capability-gated enhancements; do not force the ACP implementations to mimic App Server events.
- Remove obsolete restrictions and duplicated event/session handling made unnecessary by the shared boundary.

## Acceptance

- Codex can serve an Exec/lead work-linked conversation and a productive Staff Attempt through the configured role defaults.
- Approved MCP tools work without exposing unapproved endpoints or bypassing Restless Authority.
- Native steering reports applied only after App Server acknowledgement; all other input is queued truthfully.
- Usage, permissions, terminal outcomes, cancellation, restart, and cleanup pass the shared semantic corpus.
- Native Codex ambient configuration, skills, MCP, and approvals cannot augment Restless policy.
- No ACP compatibility layer is added around Codex merely for symmetry.
- Removing the previous conversation/MCP refusal paths does not weaken readiness or fail-closed behavior.

## Makes Deletable

- The “Codex worker runtime is restricted to productive Staff Attempts” branch.
- The blanket “cannot preserve connected-tool parity” refusal once exact parity is proven.
- Codex-only conversation projections that duplicate the normalized session boundary.
- UI assumptions that Codex is worker-only.
