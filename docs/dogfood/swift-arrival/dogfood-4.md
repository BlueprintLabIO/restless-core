# Swift Arrival Dogfood 4 — Networked delivery walking skeleton

**Status:** EXP-15 vertical-slice campaign running from the verified v0.4 mechanics baseline
**Version:** 0.5
**Type:** Standard build dogfood / first product-feasibility run
**Company:** `swift_arrival_test`
**Operating phase:** Exploration → build
**After-action:** [`dogfood-4-after-action.md`](./dogfood-4-after-action.md)

Dogfood 4 is a persistent Swift Arrival charter. Later slices revise this document's version and add
run-specific after-actions; they do not become Dogfood 5 merely because the game grows.

## Proposed Version 0.6 amendment — embodied NPC development loop

After the v0.5 campaign freezes an exact candidate and Sprint 26 proves the execution substrate,
[EXP-16](../../../experiment/exp-sprints/exp-sprint-16-embodied-npc-playtesting.md) proposes one shared
embodied-agent architecture for cheap delivery/recovery playtesting and real driver, robber and vampire
behaviour. The evaluator must use ordinary player actions and physics; production roles may use
host-authoritative perception but cannot directly mutate mission, cargo, delivery or combat outcomes.
Its executable architecture, scenario and evidence contracts live beside the
[EXP-16 controller record](../../../experiment/coordination/experiments/EXP-16/README.md).

This is a draft experimental amendment, not execution authority or a promoted game version. Dogfood 4
remains the charter and receives the keep/revise/purge decision after source-blind current-candidate
review.

## Version 0.5 amendment — tuning-first full vertical-slice campaign

Version 0.5 expands the bounded mechanics skeleton into the first complete vertical slice already
defined by the game concept: one depot, one walk-through truck, one obstacle route, three destinations,
six cargo types with an incompatible pair, a robber encounter, a police inspection and one coherent
15–20 minute workday. It does not expand into Steam, a campaign, progression or a large open world.

The product decision is now explicit: ordinary fun and feel feedback searches validated data, named
presets and fixed scenario seeds before changing stable multiplayer authority, physics ownership or
mission-state code. The target is at least 80% of ordinary findings closing through tuning, content or
presentation. A core refactor remains available when native evidence shows the tuning surface itself
cannot express reliable intention, cooperation or the central physical toy.

The controlled 30–60-loop campaign, objective gates, model envelope and terminal evidence are frozen
in [EXP-15](../../../experiment/exp-sprints/exp-sprint-15-swift-arrival-vertical-slice-campaign.md).
The founder is asked to judge fun only after independent players and current-candidate native evidence
establish the objective slice. Until then, the honest label is `vertical-slice candidate`, not accepted
game.

## Version 0.4 amendment — native first-person review and local playtest lane

Founder review exposed three defects that the earlier technical evidence did not catch: the playable
camera was not first-person, keyboard input in the remote desktop was not a reliable player contract,
and Company-computer control imposed needless repeated claim/return actions. Version 0.4 corrects
those defects before any further product expansion.

The review target is now an actual first-person client view: two local delivery gloves remain visible
in the foreground, `WASD` moves and drives, and `E` interacts. Carrying the crate into the cab now
keeps it held while entering the driver seat; leaving the seat, walking to the cargo rear, and
interacting unloads it at the host-judged destination. This is a product correction, not a new
feature claim.

The cockpit claims keyboard/pointer control automatically when the Company computer is free. It never
displaces an active actor or another live owner tab, renews only after observed desktop input, and
returns control after 60 seconds without input. The owner can still explicitly return control.

The Runtime now has a deliberately local, inspectable OS-input lane (`xdotool` plus `scrot`). Its
`playtest.sh` launches rendered host/client windows, focuses the real client window, sends physical
keyboard events, captures checkpoints, and requires host and client delivery evidence. It is a
mechanical regression replay, not a substitute for visual or founder judgement. A model-led visual
playtester is admitted only after a real model-gateway probe in an isolated `_test` company; it must
be able to decline an unplayable target and preserve screenshots, trace, logs, and a concise verdict.

### Version 0.4 acceptance additions

9. The native review target presents a first-person player view with two visible arms/hands, and the
documented `WASD`/`E` interaction path can complete the loop.
10. The local OS-input replay proves its actions reached the rendered client and that the host and
client both observed authoritative completion. A replayed success is mechanics evidence only.
11. Company-computer control is frictionless when free, safe when held, and automatically expires
after one minute of no actual desktop activity.

### Version 0.4 verified evidence

On 2026-08-28, the local Runtime at `/company/projects/swift-arrival` passed
`DISPLAY=:1 ./playtest.sh evidence/v04-os-input-pass` from game commit `84ff174`. The retained
`input-trace.tsv`, host/client logs, and five screenshots show a rendered client receiving OS-level
input, host-resolved pickup and seat entry, bounded route completion, unload, and delivery
completion. The test initially failed on a real product constraint—cargo carrying prevented driver
seat entry—and the committed interaction correction `1f2cf6e` fixed it before the passing replay.

The isolated `swift_arrival_flash_probe_test` reached its model-gateway turn with
`zai/glm-5.3-flash`, but a fresh-runtime invocation was blocked before inference because the local
OMP profile had no selected model. A follow-up with that local profile supplied then closed during ACP
initialization; both attempts spent $0. This is recorded as Flash **unadmitted for this run**, not as
a claimed visual-playtest capability. The local replay remains the canonical lane until a clean,
separately budgeted Flash visual probe produces a real result.

## Version 0.3 amendment - continuous product development

Version 0.2 established the technically runnable local loop at commit
`3dc502ae9938dafccd26286e0d28ee8f50dc60c1`. Version 0.3 keeps that exact result as its baseline and
tests whether one standing Game Product lead can improve it across material playtest and executable
signals without repeated owner decomposition.

Each improvement remains bounded Staff Work with exact Git lineage, runnable evidence and lead
judgement. Routine signals and terminal results wake the lead directly; Exec remains available. One
explicit scheduled product review is included to test time-driven continuity. The schedule is a review
opportunity, not evidence that more features are valuable, and may correctly close with no work.

The controlled sequence, recovery injection, metrics and stop conditions are frozen in
[`EXP-10`](../../../experiment/coordination/experiments/EXP-10/README.md). This amendment does not turn
the technically passing baseline into founder taste acceptance. The founder remains the final evaluator
of whether the resulting native game deserves continued investment.

### Version 0.3 result

EXP-10 completed in an isolated `_test` company at exact candidate commit
`f9f5e61ed733d8479cf2ae3078779c73db457317`. One Staff-owned cycle reduced debug dominance and
overlapping labels and added explicit objective/route cues while preserving the positive two-player
delivery. A second cycle recovered a deliberately killed worker, removed an injected parse regression
and added a passing outside-zone negative release probe. An exact duplicate created no Work; one
direct scheduled review found no new evidence, created no production and was followed by an 83-second
zero-delta interval.

The source `swift_arrival_test` company was not silently advanced: failure injection and accepted
experimental changes remain isolated until founder judgement. Review the exact before/after target in
[`FOUNDER_REVIEW.md`](../../../experiment/coordination/experiments/EXP-10/FOUNDER_REVIEW.md) and the
full architectural result in [`RESULTS.md`](../../../experiment/coordination/experiments/EXP-10/RESULTS.md).

## Why this run exists

Swift Arrival is an ambitious multiplayer physics game. Its idea vault is deliberately broad, but its first uncertainty is narrow:

> Can a small Restless-run studio produce a genuinely runnable, two-player online delivery whose moving truck, shared cargo, and visible hands already feel like one game?

This run proves neither market demand nor a Steam-ready alpha. It tests whether Restless can turn the four source documents in this directory into one bounded playable outcome without asking the owner to become a project manager, Godot operator, or integration engineer.

The executable artifact is the evidence. Plans, task graphs, generated assets, screenshots, and agent reports support inspection but cannot substitute for a host-and-client delivery that a human can run.

## Starting mission shown to Exec

> Build a private, original Swift Arrival walking skeleton. From a reproducible local command, one host and one joining client must complete a short delivery: both enter the same walk-through truck, one player can take the driver position and move it along a simple route, at least one player can pick up and unload a crate using visible world-space hands, and the host authoritatively completes the delivery. Use placeholder geometry and original labels. Keep the game runnable and bring me the prepared build, play instructions, exact evidence, and only the product judgement you genuinely need. Do not publish, buy, enrol in Steam, or contact anyone externally.

The mission is intentionally outcome-specific but does not prescribe the engine layout, worker count, implementation sequence, or exact art treatment.

## Success contract

### Desired outcome

A private local build demonstrates the first Swift Arrival loop as a coherent shared experience: two people connect, inhabit one truck, move a crate, drive a short route, unload it, and receive an authoritative delivery result. The interaction need not be polished, but it must make the proposed game legible rather than merely showing disconnected networking and physics demos.

### Accountable acceptance

The founder is the named final evaluator. An independent play reviewer may contribute an observation, but does not replace the founder's judgment. The accountable Product lead prepares the exact runnable review target and accepts or rejects Staff output; the lead does no planned production or silent repair.

### Acceptance criteria

The founder can follow a concise prepared path and observe all of the following:

1. A documented local command launches a host and one joinable client from the same pinned project.
2. The remote client joins the host's session over the selected local transport; connection evidence identifies the host and peer rather than inferring success from two windows being open.
3. Both players appear in the same continuous, walk-through truck space and can move between the cab and cargo area.
4. Each player has two visible world-space hand representations. At least one player can use a hand interaction to pick up, carry, and release the crate; the host rejects or resolves the resulting cargo state.
5. A player can occupy the driver position and move the truck across a short, bounded route while the other player and crate remain meaningfully present in the shared session.
6. The crate can be unloaded at the destination and host-owned mission state visibly records one completed delivery for both players.
7. A focused repeatable probe covers host start, client join, crate interaction, route completion, and clean shutdown; its output, engine version, commands, and limitations are retained.
8. The final review target is live-probed immediately before review and is accompanied by a meaningful Git checkpoint, run instructions, and known limitations.

### Explicit exclusions

- Steamworks, lobbies, invites, browser/WebRTC, cross-play, dedicated servers, host migration, and public distribution.
- Six-player scale, reconnect support, reliable performance under adverse network conditions, full rollback, or exact mid-physics persistence.
- A second truck, truck transfer, climbing, NPCs, robbers, police, progression, depot, campaign, procedural roads, world events, voice chat, monetisation, or a content-production system.
- Production art, non-original third-party assets, paid asset packs, generated asset procurement, or claims about fun, retention, or commercial demand beyond the recorded play observation.
- Building Restless product mechanisms that the run has not demonstrated a need for.

The idea vault remains a source of creative direction only. Adding an item from it is not success and does not expand this contract.

## Starting state, tools, and source authority

The dedicated `_test` company begins from a fresh, isolated Git repository/worktree named for Swift Arrival. It contains these four source documents:

- [`swift-arrival-game-concept.md`](./swift-arrival-game-concept.md) — product intent and constraints;
- [`swift-arrival-ai-development-plan.md`](./swift-arrival-ai-development-plan.md) — milestone order and agent-operability guidance;
- [`swift-arrival-multiplayer-architecture.md`](./swift-arrival-multiplayer-architecture.md) — the proposed host-authoritative, ENet-first technical direction; and
- [`swift-arrival-idea-vault.md`](./swift-arrival-idea-vault.md) — non-committed creative inventory.

Before implementation, the company live-probes the actual Runtime for the game engine, export path, headless capability, local networking, screen/video capture, Git, disk space, and the command needed to start two local processes. A tool that is configured, documented, or installed but has not been run is `unverified`, not available. If a required capability is missing, the company brings the exact bounded blocker and viable recovery choices to the founder; it must not simulate a successful build or connection.

The team pins the observed engine and plugin versions in the project only after probing them. The default first transport is local ENet through Godot's high-level multiplayer API. Replacing that choice requires evidence that it cannot produce the acceptance outcome, not anticipation of Steam or browser work.

## Organisation constraints

- Begin with the singleton Exec and any standing actors the product truthfully retains.
- Exec appoints one accountable Game Product lead for the complete playable outcome. The lead is a non-producing supervisor and final integrator.
- The default is one end-to-end gameplay Staff worker. The lead may add a Staff member only for a stable, independently useful seam—such as a runnable build/probe harness or a bounded visual-interaction contribution—where the added evidence or parallelism repays the briefing and integration cost.
- At least one Staff worker owns all planned production. No actor may silently repair another worker's planned artifact outside its responsibility.
- The evaluator does not prescribe a department chart, task graph, or worker count. Material facts, candidate artifacts, failures, and decisions move directly to the affected lead; priority or budget conflicts go to Exec.

This tests natural accountable leadership and evidence-led team shape, not a fixed game-studio org chart. It is expected that one worker may be the right production shape at this size.

## Authority, cost, and owner-attention envelope

**Allowed without another founder decision:** create and edit ordinary Runtime files in the isolated worktree; use locally installed tools; run local hosts and clients; create original placeholder assets; make local Git commits; inspect local logs, recordings, screenshots, and builds; and use the existing model envelope.

**Requires exact founder approval:** any paid tool, asset, service, plugin, or API; Steam account, Steamworks enrolment, key, upload, or store configuration; publication, external hosting, external messages, external playtester contact, deployment, or use of non-original/licensed assets whose right to ship is not already documented.

**Prohibited:** public releases; real customer/player outreach; unapproved spend; unlicensed asset use; provider-root credentials or privileged browser sessions in Runtime; destructive work outside the isolated company state; and presenting a mocked host, client, transport, or build result as a real one.

**Resource envelope:** use the company’s existing model ceiling and local compute allocation; record actual cost and time. The run has no semantic deadline. A proposed increase in model or compute budget is an explicit founder decision with the exact expected discriminating value.

**Expected founder attention:**

1. the opening mandate;
2. at most one prepared product/taste judgment if the team reaches a genuine ambiguity that the concept documents cannot resolve; and
3. final native review and acceptance, rejection, or revision.

Choosing tasks, locating tools, resolving ordinary Godot issues, relaying credentials, repairing harness failures, or chasing actor status are rescue interventions and count against the run.

## Evidence and review target

The Product lead chooses and live-probes the native review target. The default is a pre-positioned local host/client session with the route ready to run, plus a short captured recording as fallback evidence—not a source diff or an agent narrative.

When the run starts, the evaluator creates an evidence bundle under `docs/dogfood/swift-arrival/evidence/`. It contains references, not duplicate source truth:

- `run-manifest.md` — scenario version, exact source commit, company, model/tool versions, live-probe results, envelopes, and declared unknowns;
- `capability-probes.md` — commands, observed outputs, dates, and limits for engine, export, local transport, recording, and two-process launch;
- `artifact.md` — project path, meaningful Git checkpoints, launch command, and ReviewTarget;
- `play-observation.md` — founder/independent reviewer observations against every criterion;
- `verification.md` — focused executable probe commands and results, including failures;
- `owner-attention.md` — active minutes and intervention taxonomy;
- `cost-and-usage.md` — model/compute cost and unknown fields, without private chain-of-thought;
- `timeline.jsonl` — material responsibility, artifact, decision, failure, recovery, and review observations only; and
- `final-report.md` — terminal classification, accepted criteria, limitations, and next decision.

Evidence records include a locator, observation time, responsible actor/session, the criterion they support, and material limitations. A screenshot of a game window is inspected-artifact evidence, not proof that networking or delivery completion worked.

## Controlled challenge and recovery

This is a standard dogfood, not a chaos exercise. The evaluator may make the following injection only after the base local two-player loop is runnable and before final acceptance:

> The delivery crate must now be accepted only after it is unloaded in the marked destination zone; carrying it through the route alone is not a completed delivery.

The injection tests whether the affected delivery-completion path is revised without restarting the entire project, discarding useful work, or turning unrelated planning into founder work. The exact delivery is recorded as an owner requirement change; it is not hidden future knowledge for actors.

If the product already implements the stricter condition, the evaluator instead asks for one visible and testable correction to the delivery feedback that does not change the mission. The evaluator records which version occurred. No fake external provider, fake market result, or simulated player feedback enters the company’s evidence.

One process replacement may be injected after a useful committed artifact exists, using the ordinary supervision boundary. It tests recovery of responsibility and local work, not data destruction. Do not combine it with a deleted repository, wiped session backing, or a fabricated networking failure.

## Manual review

The founder reviews the prepared native target in this order:

1. start or join the supplied local session and confirm the host/client identity evidence;
2. move both players through the truck, observing the cab and cargo space as one location;
3. use the hand interaction to handle the crate and observe the host-resolved result;
4. drive the route while the second player and cargo remain part of the shared session;
5. unload the crate in the destination zone and observe delivery completion on both clients;
6. inspect the focused probe output, source checkpoint, recording/fallback artifact, known limits, owner-attention record, and spend record; and
7. record `accept`, `reject`, or `revise` with one concise reason.

Do not begin with the task list, source diff, or agent report. Use them only to explain a behaviour that has already been run or inspected.

## Risk dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| A real external effect or spend occurs while proving a local game loop. | **Invariant** | No external capability is in the envelope; all work stays local and private. |
| The team claims a build, transport, or headless harness works without running it. | **Guarded** | Live-probe and retain command/output before relying on each capability. |
| Multiplayer physics proves too unstable for a meaningful local delivery. | **Accepted for this run** | Record the observed failure and choose simplification, a narrow feasibility experiment, or stop; do not hide it with more content. |
| One local two-player success is mistaken for six-player or Internet viability. | **Accepted for this run** | The contract makes no such claim; later scenarios must test those properties directly. |
| The unfiltered vault expands the milestone beyond an inspectable outcome. | **Guarded** | Explicit exclusions and lead review; additions need evidence that they are required for this loop. |
| A failed worker/process loses recoverable useful work. | **Guarded** | Ordinary Git checkpoints, artifact references, and one bounded process-replacement observation. |
| The first playable loop is not amusing or legible enough to justify more investment. | **Accepted** | Founder judgment may reject it; a negative result is valid evidence. |

## Termination and next decision

**Immediate safety stop:** any possible external publication/spend, credential or tenant crossover, unlicensed asset use, data corruption outside the dedicated company, a material budget breach, or a founder stop.

**Natural terminal state:** the founder is presented the runnable review target; the accountable lead explicitly stops after evidence shows the loop is not viable; or the company reaches an honest blocker and prepares the precise founder decision needed to continue.

Classify the run as:

- `accepted` — every acceptance criterion passes and the founder accepts the playable artifact;
- `rejected` — runnable evidence shows the loop is not acceptable or not worth continuing;
- `inconclusive` — the available evidence cannot support an honest product judgment;
- `product-invalid` — Restless prevented a valid company run through coordination, Runtime, or authority behaviour; or
- `evaluation-infrastructure-invalid` — the product evidence exists but the evaluator cannot make the required decision.

The after-action answers what playable outcome exists, which criteria passed, whether any owner attention was legitimate or rescue, the smallest observed Restless friction, and one next move:

- **continue** to a narrow latency/physics feasibility run only if the accepted loop is coherent;
- **branch** to a second controlled implementation only if concrete evidence leaves a technical choice unresolved;
- **pivot** the interaction/truck model if the prototype works but fails the founder’s core judgment; or
- **stop** Swift Arrival work if the smallest loop does not earn another investment.

Do not turn a successful first run into a claim that OrgIntel outperforms a strong single agent. Once the environment can be provisioned repeatably, a separately versioned follow-up should run matched `single_agent`, `minimal_team`, and `orgintel` modes from the same fresh source snapshot, tool access, budget, and acceptance contract.

## Version 0.5 experimental result — autonomous playability frontier

EXP-11 controlled an isolated v0.5 successor from baseline `84ff1745`. The production organisation
retained the intended architecture: Exec delegated to one non-producing Game Product lead, the lead
supervised one end-to-end gameplay worker, and independent native players remained outside production
context. The exact GLM-5.3 Flash route was unavailable, so the founder authorised exact GPT-5.6 Sol
through the local OpenAI-compatible route; the substitution is retained as an evaluator limitation.

The team produced experimental candidate `41f4fa53a2cd05ab17aea473f3d1be28979b2dcf` with real
player/truck/world collision, camera-relative control, recoverable parcel and seat paths, clearer
interaction feedback, host-owned journey gating and five passing final deterministic gates. The
candidate remained isolated and was not promoted.

The frozen independent-playability contract failed. Strict R19 directly proved immediate route-zero
shortcut rejection, then reached the visible route end through ordinary native play after a deliberate
drop and recovery. The player exited, re-entered, moved and exited again, but could not obtain visible
delivery completion from the destination. This is a reproducible experience blocker even though
scripted mechanics pass. The required two consecutive fresh completions were not achieved and founder
acceptance review was withheld.

The experimental disposition is `product-judgement-failure`; the prepared product decision is
`revise`. Full evidence and costs are in
[`EXP-11`](../../../experiment/coordination/experiments/EXP-11/RESULTS.md). Any successor must remain a
single bounded route-end exit/unload repair with one aggregate budget and strict fresh replication.
