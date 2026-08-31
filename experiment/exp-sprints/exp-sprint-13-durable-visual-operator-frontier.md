# Experiment Sprint 13 - Durable visual operator frontier

**Status:** Complete — `model-or-policy-limited`

**Decision owner:** Founder

**Date:** 29 August 2026

**Depends on:** EXP-11's native-use protocol and friction record; an enforceable aggregate experiment
budget; exact multimodal model admission through a locally provisioned provider route.

## Decision this sprint must produce

Determine whether existing multimodal models become materially more capable, reliable and economical
at native computer work when Restless gives them a durable visual session and a very small set of
closed-loop computer-use primitives.

The decision is not whether Restless should train a realtime foundation model. It is which of these
smallest implementation shapes deserves one production implementation sprint:

1. **Playbook only:** current Linux input and capture tools, operated through a stronger visual-work
   skill.
2. **Thin visual operator:** the playbook plus durable attach, observe, act and export primitives.
3. **Temporal extension:** the thin operator plus a short recent-frame and event history, activated
   only if the thin operator still loses material state across time.
4. **Defer:** existing models remain the dominant limit, so further Runtime machinery is not yet
   justified.

The experiment must purge losing scratch implementations. It must not leave several permanent ways to
control the desktop.

## Terminal outcome

The requested `glm-5.3-flash` route was probed through `ZAI_BASE_URL` and returned provider rate-limit
code `1310`, so the owner-approved exact `litellm/gpt-5.6-terra` vision fallback was frozen for every
arm. Repaired B2 materially beat B1 on the game journey, halving decisions and eliminating observed
target/focus/invalid-interaction failures. It then failed before pixels on the website because its
public attachment contract could not uniquely identify a fresh browser target.

That repeated the target-discovery seam already repaired twice for the game. The predeclared stop rule
fired, so the spreadsheet, temporal extension and product-repair wave did not run. The scratch operator
was purged. A future experiment requires a deterministic launch-to-target identity contract that works
unchanged across targets. See [EXP-13 results](../coordination/experiments/EXP-13/RESULTS.md).

## Why this test is now necessary

EXP-11 established both the value and the weakness of vision-driven native review.

- An independent GPT-5.6 Sol player found a real route-end usability failure after deterministic
  mechanics passed.
- The final full journey required 36 minutes and USD 16.19 for one Attempt.
- Native navigation, focus recovery and mouse look consumed substantial reasoning.
- Long GUI processes were coupled to foreground tool deadlines.
- Transient feedback required hand-built atomic action and capture.
- Evidence linking, exact window selection and freshness checks were mostly manual.
- Nineteen referee iterations were needed before product evidence and harness evidence were reliably
  separated.

The product lesson was not simply "use a stronger vision model." The model lacked a stable body:
persistent target ownership, calibrated input, fresh observation, temporal continuity, recovery and a
compact evidence trail. These are Runtime-tool concerns, not new organisational entities.

See [EXP-11 results](../coordination/experiments/EXP-11/RESULTS.md) and
[EXP-11 frictions](../coordination/experiments/EXP-11/FRICTIONS.md).

## Central hypothesis

> An exact admitted multimodal visual actor using four thin Runtime primitives can complete and critique native
> game, browser and desktop workflows much closer to a diligent junior human tester than the same
> model using ad hoc screenshots and shell input. It will preserve application state across model
> turns, reduce invalid or stale interactions, produce independently reproducible feedback and close
> one repair loop without requiring realtime model inference or a new Restless control plane.

The contrary result is that a better playbook performs equally well, the thin operator merely hides
the same brittleness, temporal understanding rather than session mechanics is the dominant limit, or
the model cannot reliably operate the tested interfaces even after control friction is removed.

## Architectural boundary under test

```text
accountable lead
  -> independent visual Staff referee
       -> existing multimodal model: goals, exploration, judgement and language
       -> visual operator: bounded perception and action only
            -> mature desktop/browser/process infrastructure
       -> evidence-backed repair brief
  -> producing Staff repairs the accepted issue
  -> fresh visual Staff referee reruns the native outcome
```

The visual referee is Staff because it exercises project judgement and authors a meaningful
contribution. The visual operator is a hand or ordinary tool because it owns no goals, interpretation,
backlog or acceptance decision.

The operator is an ordinary supervised Runtime service or process. Its handles and evidence files are
Runtime state. It is not an Actor, Work type, constitutional capability, workflow engine, universal UI
ontology or second source of session truth. OrgIntel sees only the referee's real Work, Attempt and
artifact references.

No Kernel change is expected beyond enforcing the parent model-spend envelope. The experiment contains
no consequential external effect.

## The four primitive hypothesis

These names describe behavior for the experiment, not a frozen production API.

### 1. Attach

Start or attach to the exact target application; identify its process and visible surface; calibrate
coordinates and input; expose health, focus and generation; refocus or reconnect after an actor or
Bridge interruption; stop only on an explicit request.

The target application must outlive an individual model turn. A model-provider pause or ACP restart
must not silently destroy useful native state.

### 2. Observe

Return a fresh capture of the exact target surface with timestamp, dimensions, focus, process health
and the last acknowledged action. Reject a stale capture or wrong surface rather than presenting it as
current evidence.

The base treatment uses pixels and native window/process facts. DOM, accessibility trees and
project-specific telemetry remain off so the first comparison isolates session and control quality.
They may become a later sensor-fusion experiment if a measured bottleneck remains.

### 3. Act

Perform a bounded batch of keyboard, mouse or controller actions against the calibrated target and
return an action receipt plus an atomic resulting capture. Prefer application or screen-change events
to blind sleeping; a safety deadline may bound a broken wait but may not define task completion.

The primitive does not choose the action. It accurately performs the model's requested operation.

### 4. Export

Write one compact, immutable evidence bundle containing the ordered action trace, selected before and
after captures, target identity, process events, uncertainty and named critical files. Link it to the
current Attempt atomically as one evidence manifest rather than many manual artifact calls.

Raw video and repetitive frames remain bounded transient diagnostics. They do not become permanent
OrgIntel history.

## Explicit exclusions

This sprint does not build or claim:

- a new vision, video-action or world foundation model;
- reinforcement learning, imitation training or online learning;
- general realtime robotics or safety-critical control;
- continuous high-frame-rate inference;
- a universal semantic model of every application;
- a remote RPC wrapper around every desktop action;
- a new workflow/session database or durable-workflow engine;
- per-action Work nodes, messages or constitutional receipts;
- unsupervised access to live financial, identity or provider-root sessions;
- fun, expert design taste or human-equivalent embodied judgement; or
- production promotion of experimental product changes.

The experiment should accumulate useful action traces, but collecting a training corpus is not a
success criterion.

## Frozen implementation candidates

All candidates use the same exact target build, visual model, player goal, public controls, clean-room
context, action/resource envelope and acceptance method.

### B0 - Current native loop

The strict EXP-11 method: ordinary shell/native input, manually selected screenshots and manually
assembled evidence. Run B0 only on Swift Arrival; EXP-11 remains historical evidence but does not
substitute for this matched baseline.

### B1 - Playbook-only operator

The same existing tools plus a concise visual-operation skill covering target identity, calibrated
input, act-observe discipline, recovery, reporting and stop behavior. It adds no persistent operator
service or new Restless primitive.

B1 is the simplicity control. If it matches B2, keep the skill and reject the Runtime abstraction.

### B2 - Thin durable operator

B1 plus the four scratch primitives above. Use mature process, window, capture and input facilities.
Implementation stays under the isolated experiment until the result supports promotion.

### B3 - Short temporal context, conditional

B2 plus a bounded recent-frame and application-event ring buffer exposed through `observe`. Activate
B3 only when B2 produces a specific failure that a still image plus last-action receipt cannot explain,
and the missing fact is present in the retained temporal window.

B3 is not a streaming model. The deliberative model remains turn-based and receives selected temporal
evidence only when useful.

## Scenario portfolio

Every target is frozen at an exact commit, build or file hash before the first run. Runs use fresh
model processes and resettable native starting state.

### G1 - Swift Arrival native journey

Use the exact EXP-11 experimental candidate as an unpromoted test artifact. From a clean launch, the
operator must learn and execute the delivery journey, deliberately exercise one ordinary recovery,
and report any blocker with native evidence. The route-end transition is not disclosed.

This is the highest-coupling, spatial and temporally extended case. It compares B0, B1 and B2 first.
If B2 does not materially beat B1 here, stop before generalisation unless the evidence isolates a
non-session model limitation.

### W1 - Real website comprehension and navigation

Freeze one local build of the Restless public site. Give the operator an ordinary visitor outcome
covering home-page comprehension, navigation to a named research result, return navigation and a
mobile-width pass. It must complete the journey and report material usability failures without source,
DOM or producer context.

This tests information architecture, responsive UI, scrolling, links and visual judgement. It uses B1
and B2 only after G1 passes its continuation gate.

### D1 - Native spreadsheet correction

Use a labelled `_test` workbook opened in the installed desktop spreadsheet application. The operator
must locate a small set of visually described business-data errors, correct them through the native UI,
save a new copy, reopen it and confirm the intended result. A structural workbook comparison supplies
ground truth after the visual run.

This is controlled mechanism evidence, not evidence about a live business. It tests precise clicking,
keyboard entry, application state, file dialogs and verifiable economic desktop work. It uses B1 and
B2 only after G1 passes.

If the required native browser, spreadsheet application, input, capture or exact model route cannot be
admitted without changing the scenario, record `infrastructure-invalid` for that domain.

## Human reference and the 80 percent question

One human reference performs each frozen journey from the same starting state and writes the same form
of concise feedback without reading producer reasoning. Human observations remain withheld until all
model arms for that scenario finish.

The experiment does not publish one opaque "percent human" score. It may say the MVP reached the
provisional 80 percent profile only if all of these independently reported dimensions pass:

1. the operator completes at least 80 percent of the native checkpoints completed by the human;
2. it finds at least 80 percent of the human's critical or major usability failures, after blind
   adjudication of the union of findings;
3. at least 80 percent of its accepted material reports can be reproduced by a fresh actor from the
   text and attached evidence;
4. no false success claim survives native adjudication; and
5. its repair brief is accepted as correctly prioritised and actionable by the accountable lead.

Small sample size and domain coverage remain explicit. Passing means "approximately junior functional
tester capability on these bounded workflows," not general human computer use or expert game design.

## Staged execution

### Wave 0 - Admission and freeze

1. Enforce one aggregate EXP-13 budget across every company, replacement session and model route.
2. Probe exact `glm-5.3-flash` through the locally provisioned Z.ai route; if unavailable under the
   owner-approved rule, admit exact `litellm/gpt-5.6-terra` with real image ingestion and non-zero
   usage. Never expose environment-backed credentials.
3. Admit target application launch, capture, native input and exact surface identity.
4. Freeze scenario artifacts, human instructions, success checkpoints, reset procedures and evidence
   locations.
5. Confirm that no arm receives source, expected defects, another arm's report or human findings.

No product run begins if the aggregate envelope or clean-room boundary is absent.

### Wave 1 - Smallest discriminating game run

Run G1 once under B0, B1 and B2 in randomised order with fresh model and native processes. A fresh
blind adjudicator compares native completion, material findings and evidence without seeing arm
identity.

Continue only if B2 materially improves successful native progress, evidence quality, recovery or
cost over B1. If B1 matches it, choose `playbook-sufficient` and purge B2. If neither improves B0,
classify the observed limit before adding anything.

### Wave 2 - Cross-domain generalisation

Run W1 and D1 under B1 and B2. Do not tune the primitive implementation separately for each target;
target-specific launch commands and calibration are configuration, while interaction judgement remains
with the model.

Activate B3 for one bounded replay only if Wave 1 or 2 directly demonstrates missing temporal evidence
and the retained ring buffer contains the discriminating fact.

### Wave 3 - Closed repair loop

Using the selected winning shape, let one clean-room visual Staff referee produce an evidence-backed
material report from G1 or W1. The accountable lead may accept or reject it after native corroboration.
If accepted, one producing Staff actor repairs the issue in an isolated workspace. A fresh visual
referee then reruns the exact outcome and regression path.

This wave tests the promised business loop:

```text
use naturally -> notice -> reproduce -> explain -> repair -> independently verify
```

Only one material repair is required. The experiment is about operator leverage, not another open-ended
Swift Arrival build sprint.

### Wave 4 - Decide and purge

Record one disposition, delete losing scratch paths, retain evidence and draft the smallest production
implementation sprint only if supported.

## Controlled variables and fairness

- Exact frozen model selector and reasoning configuration for every admitted arm.
- Player/task goal and public controls.
- Starting application build, data and viewport.
- Fresh model process and fresh native process per independent run.
- Maximum model decisions, native actions and spend per journey.
- No source, logs, expected defect list or producer explanation.
- Same screenshot resolution and target machine resources.
- Same referee report contract and blind adjudicator.
- No findings carried between arms.
- Arm order randomised within each scenario.

Tool documentation necessarily differs between B1 and B2. That difference is the treatment.

## Resource envelope

- **Aggregate model ceiling:** USD 120 across admission, every arm, adjudication, replacement and
  closed-loop verification.
- **Wave 0:** maximum USD 5.
- **Wave 1:** maximum USD 45.
- **Wave 2:** maximum USD 35.
- **Wave 3 and one permitted conditional B3 replay:** maximum USD 35.
- **Concurrency:** one visual controller per shared desktop profile; unrelated non-visual departments
  remain outside the experiment.
- **Owner attention:** initial approval, the three short human-reference journeys and final founder
  decision. The owner does not operate failed model runs or assemble their evidence.

Each journey receives a bounded decision and action allowance rather than treating an elapsed timeout
as task semantics. Target processes remain durable while the model is thinking or temporarily paused.
Broken waits retain a safety deadline and return an explicit unknown result.

Any wave that exhausts its allocation stops. Unused allocation may remain unspent but may not silently
expand a later wave or replacement company.

## Measurements

### Native outcome

- completed checkpoints and final outcome;
- first material progress and completion time;
- recovery from ordinary mistakes;
- false completion or unsupported success;
- final artifact or application state against executable ground truth where available.

### Perception and control

- model decisions, observations and native input actions;
- stale capture, wrong-window, lost-focus and coordinate failures;
- action-to-observable-result rate;
- unnecessary repeated captures and actions;
- recovery count and successful recovery;
- state preserved across one injected actor-process interruption;
- temporal failures for which B3 would have supplied a missing fact.

### Feedback quality

- accepted critical/major findings against the blind adjudicated union;
- false-positive and duplicate findings;
- independent reproduction from text plus evidence;
- severity and priority agreement with the accountable lead;
- time and model cost from first observation to accepted repair brief;
- closed-loop regression result after the selected repair.

### Economics and harness quality

- model cost per completed checkpoint and accepted finding;
- wall time and active model/native-operation time;
- human setup, rescue and evidence-assembly actions;
- harness-caused invalid attempts and restarts;
- evidence bundle completeness and size;
- cost and latency difference between B1 and B2.

No aggregate score may hide a domain failure or exchange false success for speed.

## Promotion decision

Promote the thin visual operator into a separately approved implementation sprint only when:

1. B2 beats B1 on native outcome or feedback quality in at least two of the three domains;
2. no domain suffers a catastrophic regression or false accepted success;
3. B2 materially reduces harness-caused invalid interaction, manual rescue or model cost;
4. the human-reference profile is reported honestly, whether or not it reaches every 80 percent bar;
5. one report is independently reproduced and closes the Wave 3 repair/regression loop;
6. the target application survives the injected actor-process interruption;
7. the four primitives remain ordinary Runtime tooling with no new OrgIntel ontology; and
8. the accountable lead and founder judge the result useful enough to replace the current method.

Use one terminal disposition:

- `thin-operator-supported`
- `playbook-sufficient`
- `temporal-extension-supported`
- `model-or-policy-limited`
- `infrastructure-invalid`
- `human-profile-not-supported`
- `inconclusive`

`human-profile-not-supported` may coexist with a useful B2 win. It forbids the 80 percent claim but
does not erase a narrower product improvement.

## Failure and stop rules

Stop when any of the following occurs:

- the exact model, modality, target identity or native input path cannot be admitted;
- the aggregate or current-wave budget is exhausted;
- clean-room context is contaminated and cannot be restored with a fresh run;
- B1 matches or beats B2 in Wave 1 without a specific untested mechanism explaining the result;
- B2 fails to preserve useful target state through the injected actor interruption;
- two consecutive attempts repeat the same harness failure without new evidence;
- B3 is proposed without an observed temporal-information failure;
- the human reference cannot complete the scenario, making it an invalid operator comparison;
- a run requires live credentials, outreach, purchase, public publication or another unapproved effect;
- one accepted repair and fresh regression already answer the decision; or
- no new material signal remains and quiet is the correct state.

Do not add primitives during a run merely to rescue the hypothesis. Version a materially changed
treatment and seek a new sprint decision.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| A scratch operator grows into another control plane | **Guarded** | Runtime-only prototype, no Core entities, explicit purge before promotion |
| A better prompt explains the gain | **Guarded** | B1 is the playbook-only simplicity control |
| Human comparison becomes one flattering percentage | **Guarded** | Report five dimensions separately and retain failures |
| Source or expected defects leak to the referee | **Invariant for validity** | Fresh processes, withheld artifacts and ordering-only dependency evidence |
| Captures expose live credentials or private data | **Invariant for this run** | Local builds and labelled `_test` data only; inspect evidence before retention |
| Model deliberation is mistaken for realtime control | **Accepted** | MVP is turn-based; bounded native actions provide fast execution |
| Mouse/controller behavior remains application-specific | **Accepted** | Generalise only attach/observe/act/export; keep calibration as target configuration |
| Optional application semantics could outperform pixels | **Pending test** | Reopen sensor fusion only after this experiment isolates a perception bottleneck |
| Physical robotics remains unsupported | **Accepted** | It is outside Restless's present product and safety envelope |
| The experiment spends across replacement companies | **Invariant** | Parent aggregate envelope gates every admitted model request |
| A false success reaches product promotion | **Invariant** | Native ground truth, blind adjudication, fresh regression and founder decision |

## Required evidence bundle

1. Frozen scenario commits/builds/files, checksums, controls and reset procedures.
2. Exact model and image/input admission receipts with secrets excluded.
3. Parent and per-wave spend records covering every experimental company and replacement.
4. B0, B1, B2 and any activated B3 action/evidence manifests.
5. Human reference reports retained blind until model arms finish.
6. Native checkpoint results and executable final-state checks where available.
7. Blind finding adjudication, reproduction results and lead usefulness judgement.
8. Injected interruption evidence showing target state preservation or loss.
9. One closed repair and fresh regression record from Wave 3.
10. Cost, time, action, observation, invalid-attempt and owner-intervention comparisons.
11. Exact scratch implementation retained for the winner and deletion record for losing paths.
12. Terminal report stating the selected disposition, limitations and next implementation decision.

## Stop boundary

This document is an experiment plan and authorises nothing until the founder approves execution. An
approved run may create isolated `_test` companies, scratch Runtime tools, local native application
sessions and unpromoted candidate repairs within the stated aggregate model budget.

It does not authorise live-company effects, outreach, purchases, public deployment, access to owner-only
authentication sessions, model training, robotics work, promotion into Restless Core or promotion of
the repaired Swift Arrival/website candidate.
