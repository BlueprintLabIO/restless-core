# EXP-16 S1 NPC contract

## Candidate and scope

- Candidate commit: `0e02451bf99c7d53981ebb14afbc4949b789eac9`
- Candidate tree: `47efc95514b40975a84f5f554800f71a1df8f199`
- Frozen base: `29f2fe75e9f891f7533faa0a93d8c127e73909c5` / tree `fb3092d7c422e941bb21d565efcbfddaabd40f51`
- Shared policy/schema: `s1-body-v1` / schema 1
- Stage: S1 only. No delivery, recovery-journey, robber, vampire, repair-loop, blind-review, or human-playtest outcome was implemented or claimed.

## Implemented shared body

`npc/npc_body.gd` is one bounded body used by the player-equivalent evaluator profile and available to the declared driver, robber, and vampire profile identifiers. It provides:

1. role-legal observation ingestion with required local/player-visible fields, immutable deep-copy snapshots, content digests, and explicit rejection of hidden destination, fixture flag, internal mission state, and unseen-transform fields;
2. a schema-versioned blackboard bounded to eight observation frames, sixteen terminal receipts, five recovery steps, and a four-signature progress window;
3. deterministic utility selection with stable policy/scenario-seed tie-breaking;
4. a closed semantic intent vocabulary spanning locomotion, vehicle, interaction, cargo, combat, and social/passenger actions;
5. `dispatch`, which sends an intent to an ordinary-system callback and rejects an acknowledgement whose authority path does not match the intent;
6. terminal action receipts carrying observation lineage, controlling and authoritative peers, terminal state, observed delta, feedback, contacts, host confirmation, and digest;
7. typed stall and alternating-signature oscillation detection; and
8. the bounded recovery ladder `retry_once`, `stop_reorient_reacquire`, `reposition_reverse`, `alternate_local_tactic`, `abandon_subgoal`, then a terminal failure packet.

There is no language or vision model in the motor loop. The implementation adds no general agent framework or dependency.

## Perception and production extensions

The tested evaluator frame contains only local self state, contacts, visible entities, interactable feedback, player-visible vehicle/cargo/audio cues, and recent feedback. No production-only observation extension is implemented in S1. Future driver, robber, or vampire role knowledge must be declared before it enters this observation contract.

## Action and authority boundary

The closed adapter map is:

- character input: face, walk, stop, avoid, follow, approach vehicle, carry, aim/cease, cover, retreat, comply;
- vehicle input: steer, throttle, brake/reverse, park;
- host interaction: enter/exit, interact, board, signal, speak, interfere;
- host cargo: pickup, drop, place, strap/unstrap;
- host combat: fire; and
- local perception only: target and wait for feedback.

Clean conformance executes these through `dispatch`; it does not permit a policy to write transforms or outcomes. The corrected ENet fixture sent a player-equivalent observation from controlling peer `1857691696`, then authoritative peer 1 dispatched and returned six content-digested receipts across movement, vehicle, interaction, cargo, and combat families. The client recorded only the host receipt digests.

Before execution, `npc/scenario/scenario.json` was read and `restless-scenario doctor npc/scenario` observed Godot `4.7.2.stable.official.ed1daf0bf` and jq 1.6 available. The exact committed package was then run:

```text
S1_NETWORK_PORT=26383 restless-scenario run npc/scenario --output /company/outputs/exp16/S1_SCENARIO --seed 16001
```

Observed uncounted package result: `mechanical_status: verified`; acceptance remains blocked pending replacement Runtime gates with a leased `RESTLESS_GATE_PORT` resource.

The clean fixture recorded BODY-01 through BODY-10 passed, 22 bounded local receipts, bounded recovery from a temporary block, oscillation detection, and a terminal failure packet for permanent obstruction. The ENet fixture recorded six authoritative receipts and matching client-side digests. Compact evidence:

- `S1_SCENARIO/run-manifest.json` SHA-256 `14c88d4ea409e9d672188671245bc11480dcbc4c50c62e56295ece4e686c4b4b`
- `S1_SCENARIO/s1-summary.json` SHA-256 `3de6bb0692b7a53ca0e72b44ae22a650b2e5efc2f37d13b64887bc4fff2819e4`
- `S1_SCENARIO/clean-actions.jsonl` SHA-256 `f690a5bcdeb347a8753ddcad7d5b479f6d74ef99c899a46c6d4edb2604e2157f`

`verify-npc.sh` now requires `RESTLESS_GATE_PORT` in both `body` and `anti-cheat` modes, validates its numeric range, and forwards that exact value to every ENet host/client launch. Missing-variable probes for both modes exited 1 before Godot execution. Local checks with explicitly supplied ports 26381 and 26382 passed, but those were not Runtime-resource injections and are not counted gate acceptance. The currently registered `resources:[]` gate definitions are invalid; exact validation awaits lead-owned replacement gates.

## Explicit limits and deviations

- S1 uses small deterministic mechanics/authority fixtures. It does not complete a production delivery or role journey; those remain S2-S5.
- The baseline game has no production robber/vampire combat journey in this candidate. BODY-08 proves the bounded host-combat adapter and receipt boundary in the fixture, not balance, fairness, visual hit credibility, or a production encounter.
- Pixel/render evidence was not generated because S1 is headless mechanical conformance. No vision judgement was performed.
- Fixed seed 16001 and policy seed 991 were producer-visible S1 fixture coordinates, not held-out robustness evidence.
- Bot fixture success does not establish playability, player-facing legibility, human acceptance, or fun. The frozen public-surface false-assurance evidence remains unresolved and authoritative under Amendment 003.
