# EXP-11 playtest referee contract

**Original model under test:** `zai/glm-5.3-flash`
**Owner-authorised replacement:** `litellm/gpt-5.6-sol` (GPT-5.6 Sol via OMP's OpenAI-compatible adapter)
**Role:** Independent native playtester and referee
**Authority:** Observe and report only

The exact requested selector is infrastructure-invalid in this run: it is absent from the governed
direct catalogue, its OpenRouter route failed the live credential-capacity probe, and GLM-5V Turbo is
not included in the current subscription. The founder then explicitly replaced it with exact
`litellm/gpt-5.6-sol` through OMP's generic adapter and the locally provisioned OpenAI-compatible route. Fresh candidate reviews
therefore use GPT-5.6 Sol and must report that actual selector. They can support only the experiment
disposition `supported-with-evaluator-limit`; they cannot be counted as GLM-5.3 Flash admission or
silently renamed as Flash evidence. All other referee constraints remain frozen.

For this exact vision-capable selector, image ingestion means reading the fresh PNG with OMP's
ordinary `read` tool and receiving the image in the active Sol turn. OMP intentionally hides its
separate delegated `inspect_image` tool in automatic mode when the active model accepts image input.
Forcing or requiring that delegated tool would introduce a second, potentially different model route
and weaken exact-selector evidence. The transcript must show the image read and the same Staff turn's
pixel-grounded description; file existence, metadata or a successful capture command still does not
count as observation.

## Purpose

The referee tests whether an unfamiliar capable agent can understand and operate the real Swift
Arrival client. It supplies experience evidence that deterministic probes cannot. It is deliberately
separate from the production team so it cannot merely repeat the team's theory of the artifact.

## Admission test

The exact selector is admitted only when one recorded session proves:

1. successful first-party routed inference with non-zero usage;
2. ingestion and truthful discrimination of a known screenshot pair;
3. bounded control of the real Godot client using ordinary player inputs;
4. a fresh capture and visual interpretation after its own input;
5. a concise durable result containing evidence locators; and
6. no fallback model, source inspection or human-authored visual description.

Admission has two explicit levels:

- **visual critic:** can inspect native images but cannot act and recapture;
- **native playtester:** can inspect, act, recapture and reason over resulting state.

EXP-11 requires `native playtester`. Reaching only `visual critic` stops the run as
`infrastructure-invalid`; it must not be relabeled as successful playtesting.

## Context supplied to each playtest

Each candidate receives a fresh responsibility-scoped session containing only:

- the concise delivery objective;
- public player controls;
- how to start or focus the prepared native client;
- a bounded action/time/spend envelope;
- how to capture evidence and return the report; and
- the previous public player-visible state only when continuity is itself being tested.

The session must not receive source, diffs, architecture, producer messages, Work records, test
scripts, known defect names, previous private referee reasoning or the desired verdict.

## Allowed actions

The referee may:

- use ordinary keyboard and pointer input;
- inspect and recapture the rendered native window;
- bind input and evidence to the enumerated numeric client window ID. In the current X11 Runtime,
  raise that exact window, read its geometry, and capture the resulting screen region; an
  active-window capture alone is not proof that the client was observed;
- repair one bounded launch, focus or capture-command mistake while preserving its failed command in
  the action trace; a mistyped window selector is not itself evidence that native input is absent;
- try the documented happy path and ordinary mistakes;
- retry an action when uncertainty is recorded;
- observe host/client behavior through the prepared player surfaces; and
- stop early on a reproducible blocker.

The referee may not:

- edit files, source, configuration, state or tests;
- use developer consoles or internal logs as a substitute for player-visible evidence;
- prescribe implementation or assign work;
- contact the producer for hints;
- alter the frozen playability contract; or
- declare founder acceptance or product promotion.

## Required exploration

Within the bounded session, attempt enough of the following to evaluate the candidate honestly:

1. calibrate camera-relative `W`, `A`, `S` and `D` movement;
2. approach walls, doorway edges, floor boundaries and relevant objects;
3. use mouse look and recover orientation;
4. identify and execute pickup/drop interaction;
5. enter, operate and exit the driver interaction;
6. attempt the delivery loop with both an intended and an imperfect action;
7. inspect whether host and client expose coherent shared state; and
8. test recovery after at least one ordinary mistake when the client remains usable.

This is an outcome surface, not a fixed click script. The referee chooses its actions and records what
it could not reach.

## Durable report schema

Every report must contain:

```yaml
model_selector: <exact model that actually ran>
candidate_commit: <exact commit>
session_id: <fresh session identifier>
started_at: <timestamp>
ended_at: <timestamp>
verdict: pass | revise | blocked | invalid
goal_outcome: completed | not_completed | indeterminate
```

Follow with:

1. **Action trace** - ordered player inputs or intent, observed response and evidence locator.
2. **Material observations** - severity, reproduction, expected player model, actual behavior and
   confidence.
3. **Recovery behavior** - what happened after ordinary mistakes.
4. **Uncertainty** - anything not directly observed or not reproducible.
5. **Player verdict** - the smallest accurate conclusion about current playability, without source or
   implementation advice.

The report must distinguish direct visual observation, inference and absence of evidence.

## Evidence rules

- At least one before/after capture must show state around a material action.
- Each native capture must be preceded by an exact ID-to-title check and must visibly identify the
  client surface. A successful focus call followed by host pixels is a routing failure, not client
  evidence.
- For a natively vision-capable referee, the active-model transcript must show `read` ingesting each
  cited capture before the associated pixel observation. A separate delegated image model is not a
  substitute for exact-selector evidence.
- A mechanics claim needs either repeatable native behavior or an exact deterministic corroboration
  supplied later by the lead.
- Failure to see feedback is an experience observation; it is not automatically proof that internal
  state failed.
- Raw text copied from producer tests does not become native evidence.
- Reusing the same report, capture set or session does not count as an independent second playtest.
- `invalid` requires failure of the capability after one bounded setup repair, not merely the first
  command returning non-zero. The repair may change only launch/focus/capture mechanics and cannot
  inspect source, logs or prior evidence.
- The lead may reject a report item only with a recorded reproduction or contradictory native
  evidence. Rejections remain in the experiment record.

## Verdict meanings

- `pass`: no material blocker was observed and the attempted loop was independently understandable
  and operable.
- `revise`: the client remained testable, but one or more material experience defects warrant another
  product decision.
- `blocked`: a reproducible product behavior prevented meaningful continuation.
- `invalid`: model route, image ingestion, input, capture or evidence integrity failed, so no product
  conclusion is valid.

Two `pass` verdicts are necessary but not sufficient for experiment completion. Deterministic gates
must also pass, the lead must accept the exact candidate, and the founder retains the final judgement.
