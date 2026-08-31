# EXP-17 Codex worker parity and harness specification

**Authority:** subordinate to [`EXP17_PROTOCOL.md`](EXP17_PROTOCOL.md) and the
[experiment sprint](../../../exp-sprints/exp-sprint-17-worker-architecture-benchmark.md)

**Decision:** `C`, `R1` and `RP` use the same pinned first-party Codex session runtime. Restless
supervision is the treatment; a different producing harness is not.

## 1. Why this is a hard activation gate

Comparing solo Codex with an OMP/ACP worker that happens to call the same model would confound:

- prompt and context assembly;
- tool semantics and approvals;
- session persistence, caching and compaction;
- reasoning-effort interpretation;
- cancellation, steering and recovery;
- event and usage telemetry; and
- model-provider wire behaviour.

The experiment therefore does not start until Restless can launch and supervise a Codex worker through
the same documented first-party Codex application/session protocol used by the solo controller. A
descriptive run with another harness may inform engineering but cannot enter a matched pair.

## 2. Shared Codex runner

One pinned runner implementation is used by all arms. It owns only process/session mechanics:

- start and health-check the pinned Codex application server or supported SDK runtime;
- start or resume a thread from explicit configuration;
- submit a turn, deliver an event/steer input and request cancellation;
- stream assistant messages, tool events, approvals, usage and terminal status;
- retain an opaque session locator outside the product repository;
- reconcile a live, dead or terminal process after controller restart; and
- terminate the exact process group and clean its temporary home.

The neutral solo controller and the Restless worker adapter call this same runner library/binary. The
solo controller does not plan, review, repair or give semantic guidance; it only injects the frozen task
and event script, enforces safety and records evidence.

The pinned protocol/version and runner digest enter `PARITY_MANIFEST.json`. A compatibility shim is
allowed only if both arms use it byte-for-byte.

## 3. Exact model and effort

The producing default is:

```text
model: gpt-5.6-sol
reasoning effort: high
provider route: same admitted OpenAI-compatible route in every arm
```

Activation may amend the exact route identifier needed by the pinned runtime, but may not substitute a
different model or effort in one arm. A real no-artifact admission request proves the route, model,
effort and required tools before tasks freeze.

Record requested and observed model identity, requested effort, provider request identifiers, usage and
cached-input accounting when available. A mismatch or unobservable exact identity invalidates the pair.

Hidden chain-of-thought is never requested, stored or scored. “Effort” means the configured reasoning
level and observable resource use. Comparable process evidence consists of messages, tool calls,
checkpoints, receipts, token/cost telemetry and artifacts.

## 4. Provider and secret boundary

- Both arms receive equivalent short-lived model capability from the Restless account/provider
  boundary or an equally scoped neutral benchmark boundary.
- The local provider base URL and API key are read from environment-backed configuration and never
  copied into a task, transcript, committed file or diagnostic output.
- Provider failover is disabled during a counted pair. An unavailable route produces
  `provider/capacity-failure` for both-arm replay under the protocol; it does not silently select a
  cheaper or different model.
- Provider request limits and spend are aggregate-task controls, not per-agent rescue allowances.

## 5. Task-tool parity

Freeze one task-tool manifest per family. It names exact binaries/versions, filesystem roots, network
policy, browser/game handles, native gates and effect permissions.

`C` and the producing Codex worker in `R1/RP` receive identical task-solving capabilities:

- same starting files and writable paths;
- same shell/build/test/browser/game tools;
- same network and package policy;
- same native gates and review targets; and
- same approval defaults for task-local safe actions.

For the frozen first-pair portfolio the task network is unavailable: only the scoped host model relay
is exempted. The runner disables ambient multi-agent, plugin/skill discovery, app, browser,
computer-use and image-generation startup capabilities, and standard child network clients fail
closed. Native shell and file editing remain available. The runner reports this as
`host-model-relay-only-v1`; an unrequested background fetch, model-visible network result or
unrecorded capability invalidates the affected pair.

Restless organisational capabilities—Work/Attempt identity, messages to the lead, checkpoint feedback,
supervisory recovery and promotion—are intentionally present only in `R1/RP`; they are the treatment.
They cannot expose hidden fixtures, evaluator notes, sibling artifacts or additional product tools.

The solo actor may use ordinary Git commits and a frozen task-local `WORKLOG.md` convention because the
same convention is available to the worker in Restless arms. Neither arm gets private human rescue.

## 6. Starting-context parity

Every pair shares a content-addressed launch capsule containing:

- exact task and acceptance bytes;
- exact source/data/corpus tree;
- known constraints and allowed references;
- task-tool manifest;
- budget/safety envelope; and
- neutral persistence rules.

The Codex worker's initial user/task message is byte-identical across `C` and `R1` except for an opaque
arm-local run identifier and the minimal Restless coordination affordance appendix. The appendix may
explain how to report to a lead; it may not add task knowledge or strategy.

Exec and lead prompts, budgets and turns are recorded separately and count toward the Restless arm's
aggregate cost. Their context may contain organisational state but cannot contain hidden task evidence.

## 7. Session, cache and persistence controls

- Each arm starts with a fresh, isolated Codex home and no cross-arm thread, transcript or provider
  cache namespace deliberately shared.
- Within one run, the actor remains hot through its pinned thread/session. Restless may not repeatedly
  cold-start the worker to create artificial handoff cost.
- Longitudinal follow-ups resume the same actor thread when it survived. After the frozen process kill,
  both arms use the same runner's resume semantics over their allowed persistence.
- `C` retains product files, Git, task-local notes and its opaque Codex thread locator. `R1` retains the
  same plus Restless Work/Attempt/supervisory state—the treatment being measured.
- Cached input, compaction, context reconstruction and orientation reads are reported when observable.
  The experiment does not infer cache state from latency alone.

## 8. Steering, feedback and cancellation

The event controller delivers the same frozen semantic event at equivalent observable checkpoints:

- `C`: directly as the next user/steer input through the shared runner;
- `R1`: to the accountable lead/Work; Sprint 26 checkpoint delivery then reaches the worker;
- `RP`: as above, addressed only to affected independent units.

Ordinary feedback does not terminate productive execution. An explicit frozen urgent-interrupt event
uses the same process/session cancellation primitive and authority receipt in all arms. Controller
messages contain the event only, not advice about how to handle it.

## 9. Terminal and crash truth

The runner distinguishes:

- process alive with active turn;
- session alive and idle;
- clean terminal result;
- model/provider error;
- tool/gate failure;
- explicit cancellation;
- process death with resumable session; and
- unknown/disconnected state.

Silence and wall-clock expiry are never success. On controller or scheduler restart, observable process
and session state is reconciled before any new actor turn. Duplicate terminal events are idempotent.

## 10. Parity preflight

Before task freeze, run a no-count probe in `C` and `R1` that:

1. starts the same pinned runtime and exact model/effort;
2. reads an identical small repository fixture;
3. invokes the same shell and native gate tools;
4. writes the same content-addressed trivial artifact;
5. resumes after process replacement;
6. receives one duplicate event and records one semantic delivery;
7. cancels one long safe tool process and proves exact cleanup; and
8. emits matching launch, usage, tool and terminal receipts.

The preflight passes only when all parity-manifest fields below match or are declared treatment fields.

## 11. Parity manifest

`PARITY_MANIFEST.json` records per arm:

```text
codex_version, runner_digest, protocol_version
model_requested, model_observed, reasoning_effort
provider_route_class, runtime_image, environment_profile
task_tool_manifest_digest, network_policy_digest, gate_set_digest
starting_capsule_digest, initial_message_digest
approval_policy, sandbox_policy, session_persistence_policy
budget_ceiling, safety_envelope
organisational_capabilities
```

Allowed differences are only:

- Restless Exec/lead/Work/Attempt and communication state in `R1/RP`;
- multiple identical worker sessions in predeclared `RP` units; and
- opaque arm/run identifiers needed for evidence custody.

Any other difference blocks counted execution until removed or the experiment is amended to ask a
different question.

## 12. Completion and deletion

This specification is complete when the shared runner and parity preflight work end to end in both
arms. Delete benchmark-only duplicate launchers, one-arm prompt shims and any OMP-to-Codex parity
pretence. Retain OMP as a supported Restless worker backend only on its own merits; it is not a counted
Codex arm without exact parity.
