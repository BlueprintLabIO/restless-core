# Owner outcome — make the cavern gate a real first boss

Turn the unused cavern-gate hook at `CONFIG.CAVERN_ENTRANCE` into one coherent, playable
**Prism Warden** encounter. This is a progression slice, not a decorative landmark or a generic boss
framework.

## Observable success contract

1. After the player has at least two living team members, the north cavern gate has an unmistakable,
   animated in-world Prism Warden presence and a nearby prompt that names it and offers battle with
   **F**. Before that gate, the objective or prompt clearly says another bond is needed.
2. Starting the gate encounter enters the existing native battle without selecting a nearby wild
   creature. The battle visibly identifies **Prism Warden** and **Phase 1** while retaining normal
   movement, attacks, abilities, switching, dodge, guard and exit controls.
3. This authored boss cannot enter the wild-creature weakened/bond flow. Pressing **B** must not end
   or replace the encounter, and visible feedback explains that the Warden cannot be bonded.
4. Crossing 50% health transitions exactly once to visible **Phase 2: Prismatic Surge**. The transition
   telegraphs a named **Prism Pulse** for at least 650 ms before it lands, then damages a non-dodging,
   non-invulnerable active creature exactly once. The normal dodge/invulnerability damage rules still
   apply; the pulse may reuse the existing combat/VFX primitives.
5. Defeating the Warden returns cleanly to the basin, sets the session progression flag
   `game.flags.prismUnlocked === true`, grants the existing victory XP/bond growth exactly once, makes
   the gate encounter non-repeatable for that session, and visibly announces that the cavern signal
   is unlocked.
6. Ordinary wild battle and bond eligibility still work after the boss is gone. Existing battle,
   combat and roster/evolution behaviour remains green.
7. Leave one clean candidate commit with no new dependency or management-document substitute.

The native review target is the running basin → gate → boss battle → unlocked-gate loop. The frozen
external evaluator may arrange player/team position and combat health through the existing
`window.__game` review surface, but judges owner-visible prompts, battle state and progression rather
than requiring a particular class, file split or scene-graph origin.
