# T4 — Extend Harness Policy to Coordination Sessions

**Serves:** Sprint 39 — Work Through Attention With Verified Agent Harnesses  
**Layer:** Control Plane + Organization Intelligence + Runtime Bridge + Settings  
**Depends on:** T1

## Outcome

Exec and producing-lead conversations can use the company's certified coordination harness, while productive Staff Attempts continue to use the independently configured worker harness.

## Work

- Resolve the canonical `coordination_harness` for Exec and producing-lead conversations and `worker_harness` for productive Staff Attempts.
- Keep both defaults at `restless-managed`; do not infer a default change from installation or benchmark results.
- Record the exact resolved harness on every launched session/attempt so later configuration edits cannot rewrite history.
- Validate the selected model against the resolved harness before work is queued. Show the incompatibility and required owner action without automatic model substitution.
- Route coordination conversation launch, input, resume/reconstruction, usage, cancellation, and failure through the normalized harness boundary.
- Preserve actor identity and conversation ownership independently from harness session identity.
- Make a configuration change apply only to a new explicit session. The current session retains its recorded harness and remains resumable under that identity.
- Add compact expert settings/status for the two defaults, pinned build, readiness, auth requirement, and Restless-relevant capability gaps.
- Refuse arbitrary executable paths, package names, and unknown ACP registry identifiers in configuration/API inputs.

## Acceptance

- Coordination and worker harness settings resolve independently and default to Restless Managed for existing and new companies.
- Exec/lead conversations can launch through Restless Managed or Claude Agent with identical Restless message/work scoping.
- A session records the harness identity used at creation and does not change when company configuration changes.
- Model incompatibility and not-ready states fail before session creation/provider traffic with an actionable error.
- Settings never expose provider credentials, raw launch commands, a registry browser, or a per-task override.
- An unsupported identifier is rejected at every write boundary, including direct API input.
- Changing a default and explicitly starting a new session reconstructs from canonical Restless context without importing the old harness's private state.

## Makes Deletable

- The assumption that only productive workers have a selectable runtime.
- Conversation launch branches tied directly to OMP.
- Any temporary per-item harness picker.
- Hidden automatic model substitution.
