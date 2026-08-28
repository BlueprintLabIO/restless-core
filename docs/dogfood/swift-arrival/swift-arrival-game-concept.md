# Swift Arrival — Game Concept

Status: early concept

Target: 1–6 player online co-op

Primary platform: Steam; browser demo later

Perspective: first-person with visible physics-driven arms

Structure: roguelite workdays inside a light story campaign

## Premise

Players operate **Swift Arrival**, an underqualified delivery company trusted with wildly inappropriate cargo. They load a large walk-through truck, plan a questionable route, drive through obstacle-course roads, protect the cargo from itself and from robbers, and somehow complete the delivery.

The truck is not just transport. It is the shared moving level.

## Design pillars

1. **Shared moving playroom** — two or three players can work in the cab while two or three manage the cargo hold, and everyone can move between them.
2. **Cargo changes the rules** — every package introduces a physical constraint, behaviour or hazard.
3. **Failure escalates** — mistakes create new problems instead of immediately ending the mission.
4. **Useful incompetence** — controls are physical and funny, but consistent enough for teams to improve.
5. **Greed creates difficulty** — players choose whether to stack extra, incompatible contracts for more money.
6. **No fixed classes** — driver, navigator, mechanic, cargo wrangler and defender emerge naturally.
7. **Hands make the comedy** — players physically grab, pull, brace, drive and interfere using two visible, spring-driven arms.
8. **Challenge is chosen** — dangerous routes, bonus contracts and unstable cargo raise rewards without forcing every group into the same difficulty.
9. **Contrast makes chaos funny** — genuinely peaceful driving and recovery stretches separate the major disasters.
10. **The world acknowledges actions** — important player choices leave marks, change state, provoke specific reactions or create later consequences.

## Core loop

1. Choose contracts at the depot.
2. Inspect, arrange and secure cargo.
3. Choose a route using an unreliable physical map.
4. Drive through alternating scenic, recovery and high-pressure stretches while managing cargo and attacks.
5. Unload the correct item and satisfy the customer’s strange delivery condition.
6. Potentially transfer everything into a different truck mid-shift.
7. Finish the workday with profit, damage, reputation and story consequences.
8. Return, repair the fleet and accept a worse collection of jobs tomorrow.

## First-person physical interaction

Players see their own delivery gloves and long, slightly rubbery arms. The locomotion capsule and camera remain responsive and stable while the upper body supplies the physical comedy.

- Left and right hands can grab independently.
- Hands follow spring-driven targets instead of snapping perfectly to the cursor.
- Heavy cargo stretches and drags the arms.
- Sudden braking makes held objects and hands lag behind.
- Grip can fail under excessive force, but only with strong visual warning.
- Multiple players can hold the same object.
- Players can grab straps, doors, controls, the truck and each other.
- Driving physically connects hands to the wheel, gear lever and handbrake.
- Severe impacts can trigger a brief tumble without turning the camera into a nausea machine.

Other players see the complete low-poly employee body and its reconstructed floppy pose. The arms are a primary game mechanic and visual identity, not merely a first-person view model.

### Exterior climbing

Players can climb around the moving truck using assisted controls presented as physical climbing. A hand raycast selects a nearby handle, rail, roof edge, mirror, strap or tagged surface; holding grab anchors that hand while movement pulls the stable locomotion capsule around it.

- Generate forgiving grip points across climbable truck surfaces.
- Simulate the player relative to the truck while attached, then convert back to world space on release or a fall.
- Use spring-driven arm IK, body lean, searching feet and wind response to sell the struggle.
- Provide automatic ledge catches, short coyote time and recovery handles around likely fall points.
- Let players grab a dangling teammate’s hand.
- Keep the camera readable even when the visible body is being thrown around.

The rule remains: the presentation may look barely controlled, but deliberate climbing inputs should work reliably.

## Crew fantasy inside the truck

The driver controls where the disaster goes. The navigator sees what is coming and chooses the route. The remaining crew physically transforms the cargo, truck and situation before it arrives.

Non-driver play is not routine maintenance. Cargo can temporarily turn the hold into a laboratory, animal enclosure, kitchen, ritual chamber, moving construction site, counterfeit office or portal interior. Players might incubate an egg, keep a giant heart pumping, redirect a miniature sun with mirrors, feed a black hole, track an invisible animal, conduct an exorcism, assemble the delivery, fabricate legal labels or climb inside a crate larger on the inside.

These activities reuse common physical verbs—grab, carry, attach, pour, rotate, restrain, throw and climb—rather than becoming disconnected button-based minigames.

## The truck

The truck contains one continuous playable interior:

- A 2–3 seat cab.
- Walk-through passage between cab and cargo hold.
- Large cargo area for another 2–3 players.
- Side doors, rear roller door, tail lift and roof hatch.
- Large side windows, rear windows and roof skylights in the cargo area.
- Physical controls for mirrors, radio, GPS, map, wipers, doors, straps, winch and emergency systems.

Windows are strategically important. Rear players should see cliffs, police, robbers, incoming obstacles and cargo flying outside. The visual treatment can omit perfectly realistic transparent glass: frames, grime, highlights and faint reflections can imply glass while keeping the outside view clear.

## Depot and defence

The depot begins as a shabby garage and company home rather than a military base. It supports contract selection, route forecasts, truck choice, tools, repairs, progression and social downtime. As the company accumulates valuable cargo and enemies, players can gradually turn it into an absurd improvised bunker with shutters, spotlights, barricades, vaults, false walls and defensive equipment.

Depot defence is a rare, announced chapter event rather than a repeating wave mode. Robbers, repo agents, escaped cargo, rival couriers, police complications or collateral from a distant giant battle may threaten the facility. Failure damages equipment, loses stored cargo or creates a recovery contract; it does not erase campaign progress.

The depot must remain a comforting home between disasters. Players can depart quickly without completing repetitive maintenance chores.

## Changing trucks

The crew can be forced or tempted to change vehicles during a workday. The transfer itself is physical gameplay: every surviving package must be unloaded, carried and repacked while time, weather, inspections or attackers continue.

Possible vehicles include:

- Standard box truck.
- Armoured bank truck: secure, heavy and cramped.
- Refrigerated truck: slippery, freezing and prone to stuck doors.
- Flatbed: fast loading but no walls.
- Tiny emergency van: reliable but painfully small.
- Oversized removal truck: spacious but unable to use many routes.
- Highly questionable tanker.

Truck swaps can occur at ferries, transfer depots, borders, impound yards, wreck sites or customer facilities. Every vehicle implements a common `TruckSpec` with seats, cargo volume, doors, windows, strap anchors, roof access and attachment points.

## Cargo families

### Dangerous

- Explosives armed by impacts.
- Poison gas cylinders that must stay upright.
- Cryogenic containers that freeze nearby objects.
- Dinosaur eggs that hatch when too warm.
- Giant magnets that attract street furniture and other cargo.

### Alive or suspiciously alive

- `NOT A DEAD BODY`, which occasionally changes position.
- Three raccoons wearing one employee uniform.
- An unlicensed wizard.
- A vampire that must stay out of sunlight.
- An invisible animal located only by noise and collisions.

### High-value

- Loose banknotes that escape through open windows.
- Coin bags that change the truck’s weight distribution.
- A giant gold bar requiring the whole team to move.
- A vault whose combination changes after crashes.
- Cursed treasure that briefly makes players possessive.

### Fragile or absurd

- A full glass of water.
- A wedding cake with unstable frosting.
- A house of cards.
- A celebrity who repeatedly changes the destination.
- A crate of counterfeit rubber ducks.
- A suspicious refrigerator that knocks from inside.

The most interesting contracts combine incompatible cargo, such as explosives that must remain cold beside an egg that must remain warm.

## Roads and navigation

- Low bridges and narrow parking garages.
- Drawbridges, ferries and railway crossings.
- Roads present on the map but absent from reality.
- One-way streets that periodically reverse.
- Deliveries requiring a reverse-driving obstacle course.
- A destination mounted on another moving vehicle.
- A physical map so large it blocks the windscreen.
- A GPS that becomes passive-aggressive when ignored.

The world should feel like a truck-scale obstacle course rather than a realistic open-world driving simulation.

## Background world events

The world contains conflicts and spectacles unrelated to Swift Arrival. An original web-slinging vigilante might fight a giant lizard across the metropolitan skyline while the crew simply tries to deliver a refrigerator before closing time. These characters and creatures should be original parodies rather than recognisable copyrighted properties.

Background events operate at three levels:

1. **Spectacle** — distant animation, sound and dialogue with no gameplay effect.
2. **Collateral** — debris, frightened traffic, blocked roads or weather reaches the delivery route.
3. **Convergence** — rarely, the unrelated event collides with the contract and becomes its finale.

Most events remain spectacle. Route forecasts can hint at them with reports such as `Significant vigilante activity expected downtown`, but players are neither required nor expected to save the city.

## Pacing and urgency

A workday should breathe rather than sustain maximum chaos. A useful rhythm is depot chaos, peaceful departure, warning signs, a major incident, recovery driving, final escalation and a physical drop-off.

After major incidents, an intensity director creates a protected calm period with no new attacks, fewer hazards, softer music and scenic roads. These sections let friends talk, change seats, repair the truck, reorganise cargo and enjoy coastal roads, mountains, forests, sunsets, villages and ferry crossings.

Urgency usually comes from pursuit pressure rather than a universal hard timer. Fast progress builds distance from danger; slow driving, crashes and stops allow it to approach. Getting caught escalates through warnings, contact, damage and cargo danger before causing outright failure. Threat themes can include an avalanche, wildfire, toxic fog, police reinforcements, robbers, a pursuing creature, nightfall or a departing ferry.

Only some contracts should demand continuous movement. Other jobs create pressure through cargo instability, customer patience, police suspicion or optional bonuses.

## Enemies and inspections

- Motorbike thieves boarding through the rear doors.
- Robbers climbing across the roof.
- Helicopters attempting to lift unsecured cargo.
- Fake customers trying to accept deliveries.
- Rival couriers attaching their trailer to the truck.
- Police checkpoints where players hide cargo, change labels and attempt to look normal.

Enemies interact with the physics sandbox. Players can throw them out, trap them in crates or accidentally deliver them.

NPC behaviour uses a traditional layered game-AI model: limited perception and short memory feed utility scores, which select a goal executed by a small state machine. Navigation handles ordinary movement, while grabbing, doors, cargo, hazards and impacts use the same physical interaction systems as players.

Robbers can vary through courage, greed, competence, aggression, loyalty and panic thresholds. They should be readable, fallible and capable of abandoning one plan when a more urgent danger appears. Apparent intelligence comes from understandable intent and physical reactions rather than unrestricted reasoning.

## Arrivals and drop-offs

Arrival is a short physical finale, not a glowing completion zone. The crew must unload the correct cargo and satisfy an acceptance condition while destination geometry, a receiver personality and one possible complication reshape the scene.

A reusable encounter recipe is:

`Destination + Receiver + Acceptance Rule + Complication`

For example, a clifftop mansion, a paranoid billionaire, a perfect-condition requirement and a collapsing driveway. Some arrivals should remain completely peaceful so that disastrous endings stay unpredictable.

The receiver must acknowledge the cargo’s actual history rather than only checking a generic damage percentage. If a dinosaur hatched, bonded with one player, ate three packages and was later killed with the customer’s own ramp, the inspection, dialogue, payment and follow-up should reflect those facts.

Story-worthy actions receive acknowledgement at four levels:

1. Immediate physical and NPC response.
2. Consequences during the current workday.
3. Specific inspection and outcome at drop-off.
4. Persistent radio reports, complaints, nicknames, trophies and follow-up contracts.

## Art direction

Use a coherent, code-friendly first-person low-poly style inspired by the broad principles of *How to Fish*: chunky geometry, flat colours, readable silhouettes, sparse detail and procedural physical motion. It should not reproduce that game’s exact models or palette.

- Visible delivery gloves and elongated springy arms.
- Characters assembled from simple modular mesh pieces for how teammates appear.
- Exaggerated proportions, world-space hands and procedural wobble.
- Limited shared material palette.
- Simple shapes with expressive decals and labels.
- Procedural animation, IK and ragdolls instead of animation-heavy realism.
- AI-generated imagery mainly for packaging, fictional brands, posters, icons and UI.
- Headless Blender scripts only for meshes that are awkward to generate directly in Godot.

All shipped assets should have recorded source, licence and AI provenance.

## Technical direction

- Godot 4 stable, pinned to one version.
- Typed GDScript.
- Jolt 3D physics.
- Compatibility renderer so a browser build remains possible.
- Host-authoritative multiplayer.
- Stable first-person camera with host-authoritative grip constraints and hand targets.
- Data-driven `TruckSpec` so vehicle changes do not fork gameplay systems.
- Seeded world-event director with spectacle, collateral and convergence levels.
- Steam release first; browser demo after the native vertical slice works.

Gameplay content should be data-driven. New cargo, missions and destinations should mostly be new resource files rather than bespoke code.

## First vertical slice

Build only enough to prove the central experience:

- One truck with connected cab and glass-sided cargo area.
- First-person locomotion with two visible physics-driven arms.
- One depot, one short obstacle-course route and three destinations.
- Six network player slots, initially tested with 2–4 people.
- Six cargo types with at least one incompatible combination.
- Loading, straps, grabbing, throwing and delivery validation.
- One robber encounter and one police inspection.
- One full workday lasting roughly 15–20 minutes.

Do not initially build a large open world, progression tree, dedicated servers, host migration, procedural cities or full browser cross-play.

## Publishing position

Steam is the primary commercial target because native execution better suits multiplayer physics and Steam provides friend/lobby infrastructure. A browser build is valuable for frictionless demos and playtests, but browser networking and performance should not constrain the initial native prototype.

Steam permits AI-assisted shipped content when it is honestly disclosed and legally usable. Pre-generated AI content should be reviewed before shipping; live-generated content would require additional safeguards and is unnecessary for this concept.

## Progression structure

The game is a **roguelite workday inside a light story campaign**.

Each 30–45 minute workday contains branching routes, random cargo combinations, optional contracts, temporary damage, hazards and possible truck swaps. The persistent company layer retains its fleet, regions, licences, repeat customers, cosmetics, depot improvements and authored contract chains.

Failure costs profit, cargo and reputation but does not erase the campaign. Story is delivered through contracts, radio calls, customers and recurring company disasters rather than long cutscenes.

## Open creative decisions

- Whether leaving the truck on foot is common or an exceptional event.
- How punishing cargo loss should be.
- Which upgrades belong to the persistent fleet versus one workday.
- How much illegality is implied versus explicitly cartoonish.
- Whether a solo player uses bots, simplified jobs or character switching.
- How often truck changes occur before they stop feeling special.

## Reference links

- [Godot web export](https://docs.godotengine.org/en/latest/tutorials/export/exporting_for_web.html)
- [Godot high-level multiplayer](https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html)
- [Godot Jolt Physics](https://docs.godotengine.org/en/stable/tutorials/physics/using_jolt_physics.html)
- [Steam Content Survey and generative AI disclosure](https://partner.steamgames.com/doc/gettingstarted/contentsurvey)
- [Steam Direct fee](https://partner.steamgames.com/doc/gettingstarted/appfee)
