# T3 — Complete Claude Session, Usage, and Recovery Parity

**Serves:** Sprint 39 — Work Through Attention With Verified Agent Harnesses  
**Layer:** Runtime Bridge + Persistence + Observability  
**Depends on:** T1, T2

## Outcome

Claude Agent is a durable Restless session implementation rather than a one-turn demo: conversations and productive attempts are scoped, observable, chargeable, cancellable, recoverable, and clean.

## Work

- Implement new and load/resume behavior using harness-native session identifiers only after full Restless scope and harness identity validation.
- Normalize assistant text, tool calls/results, plans or TODOs, permission requests, nested-agent notices, errors, and terminal outcomes without inventing events unsupported by the adapter.
- Tag history emitted during load/reconstruction so it cannot become a new canonical message, tool effect, or usage charge.
- Map provider-observed charged usage and harness-reported token usage into separate source/confidence fields. Preserve unknown values.
- Map permission requests to Restless events and ensure external effects remain behind approved MCP/broker authority.
- Implement input delivery with an acknowledged `applied-to-current-turn` result where supported and an explicit next-turn queue otherwise.
- Implement bounded cancellation, process-group termination, adapter/provider failure classification, runtime restart reconstruction, and orphan cleanup.
- Verify that a failed or cancelled Claude session preserves canonical conversation/work evidence and can start a fresh scoped session without importing private state from another actor or company.
- Add concurrency and isolation cases for two companies, two actors, conversation plus productive attempt, and identical upstream Claude session strings.

## Acceptance

- New, resumed, failed, cancelled, lost, and reconstructed Claude sessions each produce one truthful terminal lifecycle.
- Resume cannot cross company, harness, actor, responsibility, conversation, work, or attempt scope.
- Replayed history creates no new canonical side effect or duplicate charge.
- Permission refusal and unsupported permission modes fail structurally and do not hang.
- Usage records distinguish provider-observed, harness-reported, estimated, and unknown values.
- Cancellation stops owned descendants inside the deadline and leaves no orphan adapter or Claude process.
- Runtime restart evidence shows either valid reconstruction or an honest lost session followed by canonical-context restart.
- Parallel isolation tests show no transcript, config, MCP, permission, or storage leakage.

## Makes Deletable

- Claude-specific transcript persistence outside the normal Session records.
- Sentinel token counting or zero-filled unknown usage.
- Unbounded cancellation waits.
- Manual process cleanup after tests.
