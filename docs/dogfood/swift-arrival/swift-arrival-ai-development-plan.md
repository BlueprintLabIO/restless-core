# Swift Arrival — AI-First Development Plan

Status: proposed implementation strategy

Project model: one human owner with AI performing most implementation and content production

Primary milestone: private 1–6 player Steam alpha

## Strategy

Build an **always-networked, scenario-driven walking skeleton from the first week**.

The game should never spend months as a large offline prototype that later needs multiplayer retrofitted. Begin with the smallest complete online delivery, then improve its physics, content and presentation while keeping it playable.

The main sequence is:

> Agent development harness → ugly networked game → hard physics → content grammar → scenario production → Steam playtest

## Principles

1. **Always keep an end-to-end playable build.**
2. **Treat multiplayer and the moving truck as foundation work, not late integrations.**
3. **Optimise the repository for bounded agent context rather than minimum file count.**
4. **Represent content as data assembled from reusable behaviours.**
5. **Give every mechanic a reproducible scenario.**
6. **Automate objective verification; reserve human attention for feel and creative judgment.**
7. **Keep work-in-progress low and make changes easy to reverse.**

## Locked product decisions

- First-person perspective.
- Two visible, world-space physics-driven arms.
- Stable locomotion and camera with a floppy upper body.
- Other players see a complete modular low-poly employee.
- Host-authoritative grip constraints; arm bones are reconstructed locally.
- Hybrid structure: roguelite workdays inside a light story campaign.
- Persistent company, fleet, regions and customer story chains.
- Temporary per-workday cargo combinations, damage, routes and upgrades.
- Trucks can change mid-workday through a shared data-driven vehicle interface.

## Phase 0 — Build the development machine

Before substantial gameplay, establish:

- One-command host plus two-client launch.
- Headless tests and automated Windows exports.
- Automatic logs, screenshots and short recordings.
- A debug menu for spawning cargo, enemies, events and route conditions.
- Network latency, jitter, loss and reordering simulation.
- Fixed seeds for repeatable scenarios.
- Player-input record and replay.
- Structured crash reports and assertion failures.
- A small set of smoke tests that prove the project launches, hosts and joins.

This harness is what allows AI agents to work with less supervision. An agent should be able to change code, run the relevant scenario and return evidence without requiring manual use of the Godot editor.

### Exit condition

An agent can implement a small change and independently produce:

- A passing test result.
- A launched multiplayer session.
- Useful logs.
- A screenshot or recording of the changed behaviour.

## Phase 1 — The ugliest complete game

Within the first implementation week, create an online walking skeleton:

1. Two players join.
2. Both walk through a stationary truck.
3. Both see their physics-driven arms.
4. A player physically picks up one crate.
5. One player enters the driver seat and grips the wheel.
6. The truck moves down a straight road.
7. The players unload the crate.
8. The mission completes.

Use boxes, capsules and flat colours. There is no need for proper animation, a depot, progression or varied cargo yet.

### Exit condition

Two networked players can complete one minimal delivery from beginning to end.

## Phase 2 — Solve the difficult physics

Implement and stabilise:

- Truck-local interior simulation.
- Exterior truck movement and road collisions.
- Acceleration, braking, turning and impact forces inside the truck.
- Remote driving.
- Host-authoritative cargo.
- Player prediction and reconciliation.
- Replicated hand targets and local arm IK.
- Host-authoritative grip constraints.
- Two-handed and multi-player grabbing.
- Cargo snapshot interpolation.
- Entering and exiting through doors, windows and the roof hatch.
- Conversion between truck-local and exterior coordinates.
- Falling out and recovery.
- Correction after major desynchronisation.

Test this phase under artificial latency before expanding content.

### Exit condition

Two remote players can drive, move between cab and cargo area, throw a crate and fall out without persistent desynchronisation at approximately 100–150 ms latency.

## Phase 3 — Build the content grammar

Cargo should be assembled from reusable behaviours rather than implemented as unrelated subclasses.

Example components:

```text
Grabbable
Breakable
Leaking
TemperatureSensitive
Explosive
Alive
Suspicious
Valuable
Magnetic
RequiresOrientation
OccasionallyMoves
```

A dinosaur egg becomes:

```text
Grabbable + Breakable + Alive + TemperatureSensitive
```

`NOT A DEAD BODY` becomes:

```text
Grabbable + Suspicious + Alive + OccasionallyMoves
```

Use the same compositional approach for:

- Cargo.
- Customers and destinations.
- Road obstacles.
- Robber encounters.
- Police inspections.
- Vehicle faults.
- Truck specifications and interaction sockets.
- Contract conditions.
- Mission modifiers.

Agents should normally add a new cargo variant through a resource file, labels and configuration. New code is justified only when the cargo introduces a genuinely new reusable behaviour.

### Exit condition

An agent can add and validate a new cargo type without modifying core networking, truck or mission code.

## Phase 4 — Scenario production

Treat scenarios as the primary unit of feature development and regression testing.

Example:

```text
Scenario: Hard Braking Eggs
Players: 3
Cargo: 4 dinosaur eggs
Road: steep descent
Event: brakes partially fail
Network: 120 ms latency
Success: at least 2 eggs arrive intact
```

Every scenario should provide:

- Initial world and truck state.
- Player count and spawn locations.
- Cargo manifest.
- Route and event sequence.
- Network conditions.
- Success and failure assertions.
- Screenshot or recording checkpoints.
- Human playtest notes when available.

Agents can then implement bounded scenarios rather than vaguely being asked to “make the game more fun.” Synthetic players and input scripts can test regressions, but they do not replace humans for judging humour, frustration or control feel.

## Phase 5 — Steam and content expansion

Once the complete delivery scenario is stable:

- Add the Steam lobby and transport adapter.
- Test all six player slots.
- Establish the procedural low-poly visual identity.
- Add cargo interactions and contradictory contracts.
- Add robbers and police inspections.
- Add basic progression and truck upgrades.
- Add the persistent company/story layer around roguelite workdays.
- Validate the shared `TruckSpec` with one mid-workday vehicle transfer.
- Automate Steam depot uploads.
- Create a hidden Steam Playtest and distribute private keys.

The browser/WebRTC version should remain a later experiment rather than a constraint on the first Steam alpha.

## Recommended repository structure

```text
game/
  player/
  truck/
  cargo/
  runs/
  missions/
  enemies/
  world/
network/
  transports/
  authority/
  snapshots/
  interpolation/
content/
  cargo/
  trucks/
  missions/
  scenarios/
tools/
tests/
docs/
```

Prefer composition over deep inheritance. Avoid giant manager scripts and global event systems whose effects are difficult to trace.

An agent should usually need one subsystem’s documentation and approximately 2,000–5,000 lines of nearby code rather than the entire repository.

## Agent-facing documentation

Maintain a small set of authoritative files:

| File | Purpose |
| --- | --- |
| `AGENTS.md` | Commands, coding conventions and verification requirements |
| `ARCHITECTURE.md` | Module boundaries, ownership and network authority |
| `CODEMAP.md` | Where each responsibility is implemented |
| `DECISIONS.md` | Decisions that should not be casually reversed |
| `CURRENT_MILESTONE.md` | Current goal, acceptance criteria and explicit non-goals |
| `PLAYTESTS.md` | Observations, repeated problems and decisions |
| `ASSET_PROVENANCE.md` | Source, licence and AI provenance for shipped assets |

Keep these concise. Stale documentation is worse for an agent than missing documentation.

## Task format

Each implementation task should include:

```text
Player outcome:
Acceptance test:
Scenario to run:
Expected evidence:
Files or subsystem involved:
Explicitly out of scope:
```

A task should normally fit within one subsystem and take an agent less than a day. Split tasks that require broad repository context or cannot be independently verified.

## Project management

Use lightweight Kanban with strict work-in-progress limits:

- **Now:** at most two implementation tasks.
- **Next:** likely work for the current milestone.
- **Later:** validated ideas that are not yet required.
- **Icebox:** exciting, uncommitted ideas.
- **Bugs:** reproducible problems with evidence.

Avoid story points and ceremonial Scrum. Use one-week playable milestones, short-lived branches and frequent integration into the main branch.

### Weekly cadence

- **Monday:** choose the single question the week must answer.
- **Tuesday–Thursday:** implement small, verified changes.
- **Friday:** produce a stable multiplayer build.
- **Weekend:** conduct a 30–60 minute human playtest.
- **After playtest:** record the three largest problems and choose the next question.

## Suggested milestone sequence

| Week | Player-visible result | Question answered |
| --- | --- | --- |
| 0 | Automated project and test harness | Can agents work independently? |
| 1 | Ugly two-player delivery with visible arms | Does the full loop exist online? |
| 2–3 | Physics hands, moving interior, remote driving and one crate | Is the central technical idea viable? |
| 4 | Reliable two-hand grab, throw, fall-out and recovery | Does physical interaction survive latency? |
| 5 | Data-driven cargo behaviours | Can content scale without bespoke code? |
| 6 | Depot, branching route, three stops and complete workday | Is the hybrid run structure satisfying? |
| 7 | Cargo interactions and escalating failures | Does chaos create stories? |
| 8 | Robbers and police inspection | Do threats improve teamwork? |
| 9 | Visual, audio and UI identity | Does it resemble a marketable game? |
| 10 | Six-player stress test | Is the target player count viable? |
| 11 | Steam lobby and Playtest build | Can friends install and join normally? |
| 12 | Private playtest and focused fixes | Should production expand? |

## Timeline expectations

| Milestone | Full-time AI-led | Around 10–15 human hours/week |
| --- | ---: | ---: |
| Networked walking skeleton | 1–2 weeks | 3–5 weeks |
| Stable two-player physics prototype | 4–5 weeks | 2–3 months |
| Complete delivery vertical slice | 6–8 weeks | 3–4 months |
| Private 1–6 player Steam alpha | 10–12 weeks | 4–6 months |
| Strong public demo | 4–6 months | 8–12 months |

AI should substantially accelerate systems, data entry, testing tools and repetitive art production. Multiplayer debugging, control feel, humour and human playtest scheduling remain the likely pacing constraints.

## Expected codebase size

Approximate handwritten GDScript and shader code:

| Stage | Expected code |
| --- | ---: |
| Networked walking skeleton | 4,000–8,000 lines |
| Complete vertical slice | 12,000–25,000 lines |
| Private Steam alpha | 25,000–45,000 lines |
| Polished commercial release | 45,000–90,000 lines |

Possible release breakdown:

| Subsystem | Expected lines |
| --- | ---: |
| Networking and sessions | 5,000–10,000 |
| Truck and physics | 5,000–9,000 |
| Player interactions | 3,000–6,000 |
| Cargo and hazards | 5,000–10,000 |
| Missions, world and enemies | 6,000–12,000 |
| UI, progression, audio and saving | 5,000–10,000 |
| Tools and tests | 8,000–15,000 |

Godot scene and resource files may add tens of thousands of textual lines, but most represent configuration rather than logic.

A reasonable target is roughly **60,000 lines of meaningful handwritten code**. Exceeding 100,000 lines for the intended scope may indicate excessive abstraction, duplicated systems or too many bespoke cargo mechanics.

## Repository and build size

- Handwritten source code: generally only a few megabytes.
- Full repository including source art and audio: approximately 1–5 GB.
- Compressed release build: approximately 300 MB–2 GB.

Assets, particularly audio and textures, will dominate storage. Procedural meshes, shared materials and compressed audio help keep the project compact.

## Decision gates

### Gate 1 — Networked walking skeleton

If an end-to-end two-player delivery cannot be produced cleanly, simplify the architecture before adding content.

### Gate 2 — Moving truck physics

If players and cargo cannot remain stable under realistic latency, simplify the interior/exterior model rather than trying to mask the problem with more smoothing.

### Gate 2B — Physical hands without frustration

If grabbing feels unreliable, retain physical-looking arms but make the control target, socket selection and grip much more forgiving. The appearance can be floppy; the player’s intention must remain legible.

### Gate 3 — Human fun test

If the physical toy is not funny without progression or content volume, revise driving, grabbing and failure escalation before proceeding.

### Gate 4 — Six-player viability

If six players create excessive bandwidth, visual confusion or idle roles, adjust cargo density, role surfaces and update prioritisation.

### Gate 5 — Private Steam alpha

Use playtest evidence to decide whether the project should become a public demo, remain a small friends game or change direction.

## Scope control

Explicitly defer:

- Dedicated servers.
- Host migration.
- Steam/browser cross-play.
- Large procedural open worlds.
- Realistic vehicle simulation.
- Bots intended to replace human teammates.
- Live AI-generated content.
- Large progression trees.
- More than one production-quality truck before the base vehicle works.
- Hundreds of simultaneously awake rigid bodies.

The highest-priority milestone is the ugly, networked truck. If that experience is funny, the remaining work is mainly reliable content production and polish.
