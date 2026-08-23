# Matched Cosmon mission — Loop 4 gate to Prism Caverns

Starting from exact seed `514b7b3d0a65e093af608b08ca142344412181f4`, deliver one coherent playable
Loop 4 milestone:

> Choose a starter, reach the north Sunleaf Basin gate, confront a visible corrupted Prism guardian,
> resolve that encounter through the existing combat/bond systems, unlock the gate, enter a compact
> authored Prism Caverns arrival chamber, and return to the basin.

The milestone must preserve the existing exploration, battle, bonding, roster and evolution loops.
Reuse their real implementations; do not create a parallel combat or capture system.

## Observable success contract

1. While locked, the north gate is visible and denies cavern entry with clear feedback.
2. A corrupted guardian is visible at the gate and presents an unambiguous contextual interaction.
3. The interaction enters the existing Battle system with the existing controls and HUD.
4. Resolving the guardian encounter through the implemented victory or successful calm/bond path sets
   one shared unlock state, removes or settles the guardian, and communicates the outcome.
5. The unlocked gate moves the player into a distinct cavern coordinate space; the player, active
   companion and camera all arrive coherently on the cavern floor.
6. A readable return route restores the basin beside the gate without breaking the render loop.
7. The arrival is visibly authored rather than a featureless dark room: it has a readable focal
   landmark, composed crystal/rock forms, depth or height variation, a clear entrance/return route and
   the tone “mysterious, beautiful, quiet, slightly dangerous.”
8. Existing native browser proofs and new milestone-specific proofs pass with zero browser errors.
9. The final candidate is a clean advanced Git commit and includes prepared screenshots of the gate
   confrontation and cavern arrival for independent review.

Prefer the smallest implementation that genuinely satisfies this whole experience. Do not add broad
trainer systems, quests, a large empty biome, publishing, multiplayer infrastructure or unrelated
roadmap work. There is no mid-run owner help.
