# EXP-16 embodied NPC architecture specification

**Authority:** subordinate to [`EXP16_PROTOCOL.md`](EXP16_PROTOCOL.md) and the
[experiment sprint](../../../exp-sprints/exp-sprint-16-embodied-npc-playtesting.md)

**Purpose:** freeze the smallest shared architecture that can power both player-equivalent automated
play and real Swift Arrival NPC roles. It is a product/game contract, not a general autonomous-agent
framework.

## 1. Design constraints

1. **One body, several policies.** Evaluator, driver, robber and vampire profiles share observations,
   blackboard conventions, goal selection, action adapters, receipts and failure reporting.
2. **Semantic decisions, physical execution.** A policy can request “drive toward waypoint” or “pick
   up visible cargo”; the adapter must accomplish it through ordinary controls, collisions,
   interactions and host authority.
3. **No model in the motor loop.** Mechanics advance at engine rate. A language/vision model may judge
   a bounded failure packet or decisive rendered moment, never individual steering frames.
4. **Deterministic where determinism is useful.** Fixed scenario, world and policy seeds reproduce the
   same choices. Network scheduling and rendering may vary, so evidence compares semantic receipts,
   not pixel-perfect frame numbers.
5. **Failures become observations.** Stalls, oscillations and rejected actions emit typed evidence and
   stop or recover within fixed bounds. They do not spin until a time limit happens to fire.
6. **Host authority remains real.** The server owns vehicles, cargo, damage, mission state and combat
   resolution. NPC policy cannot award itself an outcome.

## 2. Layered runtime

```text
player-visible / role-legal perception
  → observation frame
  → bounded blackboard
  → goal and utility decision
  → profile behaviour tree
  → semantic action intent
  → ordinary action adapter
  → host-authoritative game systems
  → action + scenario receipts
```

The layers are logical ownership boundaries. The implementation may use ordinary Godot resources,
nodes and scripts; it does not require a new framework, service or dependency.

### 2.1 Observation frame

One immutable frame describes what an actor could currently know:

| Field | Meaning |
| --- | --- |
| `run_id`, `actor_id`, `profile_id` | Exact run and policy identity |
| `physics_tick`, `observed_at` | Monotonic game time and capture time |
| `scenario_seed`, `policy_seed` | Reproduction coordinates |
| `self` | Pose, velocity, stance, held item, vehicle seat, health and legal actions |
| `contacts` | Current collision/contact normals and blocking actor identities |
| `visible_entities` | Role-legal entities with relative pose, class and confidence |
| `interactables` | Player-visible interaction affordances and their feedback state |
| `vehicle` | Player-visible controls, speed, damage, occupancy and route cues |
| `cargo` | Carried/strapped cargo observable to the actor |
| `audio_events` | Role-legal cues with origin/confidence, not hidden emitter state |
| `recent_feedback` | Bounded action acceptance/rejection and UI feedback |

The evaluator profile constructs this only from player-equivalent information. It may use exact local
geometry that a competent player could perceive, but not future route nodes, hidden mission flags,
unseen actor transforms or evaluator-only labels. Production profiles may add server-owned sight,
hearing and role knowledge that the fiction permits. Every additional field is declared per profile.

### 2.2 Blackboard

The blackboard is bounded operational memory, not a second world model. It contains:

- current goal and subgoal;
- last progress tick and last materially different observation;
- remembered visible targets with expiry and confidence decay;
- current route or pursuit plan derived from legal observations;
- recent action failures and recovery attempts;
- role state such as robber threat level or vampire agitation; and
- scenario-local counters required by the stop policy.

It has a versioned schema and deterministic update order. Facts that expire are removed. Hidden truth
cannot enter through debug singletons, direct mission references or test fixture labels.

### 2.3 Goal selection

Each profile exposes a small named goal set. A deterministic utility function chooses the highest legal
goal using the observation and blackboard. Ties resolve through the recorded policy seed.

The v0 goal sets are:

- **delivery evaluator:** orient, obtain job, load, board, traverse, recover route, unload, confirm;
- **recovery evaluator:** diagnose, reverse/reposition, reacquire, retry interaction, return to route,
  fail boundedly;
- **robber:** conceal/approach, warn/ambush, pursue, attack, board, acquire cargo, retreat;
- **vampire:** cooperate, complain/signal, seek safety, interfere, escape, recover cooperation.

No goal can name “set mission complete”, “teleport to target”, “steal cargo flag” or an equivalent
outcome mutation.

### 2.4 Behaviour policy

Each goal is implemented by a small behaviour tree or bounded state graph. The same action nodes are
reused across profiles. Every node has:

- explicit entry and terminal conditions;
- a maximum attempt count or progress window;
- observable success/failure evidence;
- an interruption rule; and
- a deterministic recovery edge or typed terminal failure.

A learned planner, open-ended code generation or model-authored behaviour at runtime is out of scope.
Policies may be tuned from evidence, but counted runs execute frozen policy bytes.

### 2.5 Semantic action intents

The shared action vocabulary is deliberately small:

| Family | Intents |
| --- | --- |
| locomotion | face, walk/run, stop, crouch if supported, avoid, follow |
| vehicle | approach, enter/exit, steer, throttle, brake/reverse, park |
| interaction | target, interact, confirm/cancel, wait for feedback |
| cargo | pick up, carry, drop, place, strap/unstrap |
| combat | aim, fire, cease fire, take cover, retreat |
| social/role | board, signal, speak a state line, comply, interfere |

An intent includes target identity or observable position, bounded tolerances, start tick, deadline in
ticks, and policy reason. It cannot include an internal mission result.

### 2.6 Action adapters

Adapters translate intents into the same input and authority paths used by ordinary play:

- character movement reaches `CharacterBody3D` or the canonical player locomotion contract;
- vehicle commands reach the existing network-owned vehicle control path;
- interactions use targeting, range, line-of-sight and feedback checks;
- cargo obeys pickup, carry, placement and strap validation;
- combat uses aim, cooldown, ammunition, hit and damage resolution; and
- passenger behaviour uses ordinary boarding, seat and exit contracts.

Test-only methods may observe receipts or inject a frozen scenario. They may not bypass these adapters
in a counted run. Network tests record controlling peer and authoritative peer for every material
action.

## 3. Receipt contract

### 3.1 Action receipt

Every terminal intent records:

- run, actor, profile, policy and action identity;
- start/end ticks and bounded parameters;
- source observation digest;
- authority path and controlling/authoritative peer;
- terminal state: `succeeded`, `rejected`, `blocked`, `timed_out`, `interrupted` or `invalid`;
- observed state delta and player-visible feedback;
- collision/contact summary; and
- failure class and recovery edge when applicable.

The receipt proves what was attempted and observed. It does not claim success solely because a method
returned or a timer expired.

### 3.2 Scenario receipt

One scenario receipt links candidate commit/tree, game build, scenario/policy/world seeds, profile
versions, action receipt digests, native gate results, objective result, elapsed ticks, manual
interventions, process cleanup and retained evidence. The result vocabulary is defined in
[`EVIDENCE_SPEC.md`](EVIDENCE_SPEC.md).

### 3.3 Failure packet

A bounded failure packet is emitted when progress stops or evidence becomes semantically ambiguous. It
contains the last materially different observations, recent action receipts, current goal, attempted
recoveries, exact candidate and seed, a compact event trace, and at most the protocol's allowed decisive
captures. It never includes an unbounded transcript or full frame stream.

## 4. Progress, stall and recovery

Each goal declares a progress function such as reduced route distance, changed obstruction state,
accepted interaction, changed cargo custody or changed combat geometry. The actor is stalled when the
function does not materially improve within the goal's frozen progress window.

Oscillation is detected from repeated state/action signatures rather than elapsed wall time. Examples
include alternating steering directions without route gain, enter/exit loops, repeated invalid pickup
targets and pursuit around an unchanged obstacle.

The shared recovery ladder is:

1. retry once when feedback identifies a transient rejection;
2. stop, reorient and reacquire from a fresh legal observation;
3. reposition or reverse using the same physical controls;
4. select a declared alternate local tactic;
5. abandon the current subgoal when the profile permits; or
6. emit a terminal failure packet.

The ladder is bounded per scenario. It cannot reset world state, teleport, reload a successful snapshot
or silently skip an objective.

## 5. Production profiles

### 5.1 Player-equivalent evaluator

- Has the same meaningful actions and equivalent local information as a player.
- Runs headlessly for mechanics and can request rendered evidence only at declared events.
- May classify a scenario mechanically, but cannot claim fun, legibility or human acceptance.
- Uses two modes over the same body: golden-path completion and adverse-state recovery.

### 5.2 Robber

- Perception must make approach, threat and attack legible before decisive harm.
- Pursuit and shooting obey vehicle, movement, weapon, damage and line-of-sight rules.
- Cargo changes custody only through ordinary board/pickup/carry/drop interactions.
- Both the driver's fair success path and robber's adverse success path must be possible under frozen
  scenario parameters.

### 5.3 Vampire passenger

- Role state begins from scenario parameters and changes only from observable journey events.
- Calm, agitated/hungry and escape/interference states change movement, compliance and signalling.
- State is legible through behaviour plus sparse dialogue/feedback; dialogue alone does not satisfy the
  role.
- Adverse behaviour remains recoverable in at least one declared player path.

## 6. Anti-cheat and authority gates

Counted execution must fail if any profile:

- writes actor/vehicle transforms outside the canonical movement/physics path;
- writes mission completion, delivery, cargo custody, damage or combat result directly;
- reads hidden mission/evaluator fixture state not declared by its profile;
- invokes a test-only shortcut after scenario setup;
- acts without a source observation and receipt;
- continues beyond its recovery/action bound; or
- produces a material result on a non-authoritative peer without server confirmation.

Static dependency checks supplement but do not replace runtime probes. Fault-injection scenarios must
demonstrate that the gates turn red when a forbidden shortcut is introduced.

## 7. Performance envelope

- Policy evaluation may run below physics rate but must not block a physics frame.
- Headless counted runs record simulation speed, p95 policy step time and peak NPC count tested.
- Action/event traces are bounded and stream to compact JSONL; rendered frames are event-triggered.
- A fixed-seed replay compares semantic action and outcome receipts. Pixel or wall-time equality is not
  required.
- Any background process, capture session or server is owned by the exact Attempt resource lease and
  cleaned at terminal completion or restart.

## 8. Completion boundary

This contract is implemented only when the scenario matrix proves the shared body, evaluator, robber
and vampire profiles through ordinary authority. Creating interfaces, debug scenes or a successful
scripted route is not completion. A more general planner, crowds, police, traffic ecology, squad combat,
procedural dialogue and production balancing remain future product work.
