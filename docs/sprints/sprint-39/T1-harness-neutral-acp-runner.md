# T1 — Extract the Harness-Neutral ACP Session Runner

**Serves:** Sprint 39 — Work Through Attention With Verified Agent Harnesses  
**Layer:** Runtime Bridge + Control Plane  
**Depends on:** T0

## Outcome

The existing ACP session, event, input, usage, and cancellation machinery can launch either Restless Managed or Claude Agent from a certified static profile without OMP-specific assumptions leaking through the shared path.

## Work

- Separate the current OMP executable, arguments, config root, storage namespace, readiness probe, and capability expectations from shared ACP transport/session logic.
- Introduce the smallest closed harness identifier and launch-profile boundary needed for `restless-managed` and `claude-agent`; do not accept arbitrary commands or registry packages.
- Keep transport concerns separate from Restless launch policy: the caller supplies system instructions, actor identity, cwd, model/effort, tool/MCP policy, and scope.
- Normalize initialize/auth/new/load/prompt/update/cancel behavior against the ACP v1 baseline while preserving observed optional capabilities.
- Namespace runtime config and session storage by company and harness in addition to the existing actor/responsibility/conversation or attempt scope.
- Persist resolved harness identifier, pinned build identity, transport, and observed capability digest on Session and productive Attempt records.
- Add canonical `coordination_harness` and `worker_harness` configuration parsing. Preserve `worker_runtime = "omp" | "codex"` as a read-compatible migration input with exact mapping and no changed default.
- Refuse unknown identifiers and incompatible model/harness combinations before queueing Work or starting a process.
- Move existing OMP tests to the shared corpus and prove no launch-policy or event regression.

## Acceptance

- Shared ACP code contains no OMP-specific executable name, argument grammar, config path, or assumed optional capability.
- Only release-certified identifiers resolve to an executable profile.
- Existing configuration produces byte-for-byte equivalent effective Restless Managed launch policy where relevant.
- Legacy `worker_runtime` input maps exactly and new serialized/configured state uses canonical harness fields.
- Session locators cannot cross company, harness, actor, responsibility, conversation, work, or attempt scope.
- Capability absence produces a structured readiness or feature-gating result rather than a late protocol hang.
- Restless Managed passes the full pre-T1 regression corpus.

## Makes Deletable

- `WorkerRuntime::Omp | Codex` as the canonical configuration type.
- `AGENT_CONFIG_DIR` and OMP launch constants in shared ACP code.
- Per-harness copies of ACP framing, update dispatch, and terminal normalization.
- Tests that equate a process spawn with readiness.
