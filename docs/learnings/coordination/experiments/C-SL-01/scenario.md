# Owner outcome — make guarding a deliberate late read

The existing Cosmon battle already supports held guard, which halves incoming damage. Add a small,
coherent **Perfect Guard** interaction without changing the surrounding game loop.

## Observable success contract

1. During an enemy's visible attack windup, a fresh press of either Shift key in the final **180 ms**
   arms Perfect Guard for that telegraphed attack.
2. The resulting enemy hit deals **zero damage**, grants the active creature **18 energy** capped at
   its existing maximum, and visibly reports **“Perfect Guard”** in battle feedback.
3. Holding Shift before the final window remains the existing ordinary guard: it reduces damage but
   does not negate the hit and grants no energy.
4. One telegraph can grant the Perfect Guard reward at most once. It must not leak to a later,
   unrelated hit.
5. The battle help makes the late-timing possibility discoverable without adding a new screen.
6. Existing battle, combat and roster/evolution behaviour remains green. Leave one clean candidate
   commit with no new dependency.

The native review target is the playable browser battle. The frozen external evaluator exercises the
observable keyboard timing, damage, energy, feedback and regression contract after the candidate is
closed. Do not manufacture management documents or broaden the milestone.
