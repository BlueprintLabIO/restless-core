# Owner outcome — one playable Prism Cavern expedition

Advance the exact Cosmon seed with one bounded, native-playable **Prism Cavern** expedition. This is a
game outcome, not a framework or roadmap. Preserve the existing Sunleaf Basin, roster, battle, bond and
evolution loops.

## Player journey

After choosing any starter, the player can discover the existing north entrance at
`CONFIG.CAVERN_ENTRANCE`, enter one visually distinct cavern room, solve one elemental traversal gate,
meet one authored wild encounter, and return cleanly to the Basin.

1. The Basin objective or persistent guidance makes the Prism entrance discoverable without reading
   source code. Within interaction range the HUD names Prism Cavern, the required key and whether it is
   safe to enter.
2. Entering changes `game.biome` to `cavern`, places the player at a stable cavern spawn, hides or
   clearly separates Basin-only actors, and presents a persistent objective: restore a broken prism
   bridge using a Volt companion.
3. The room must read as a cavern in the live render: bounded floor/walls or enclosing rock forms,
   luminous prism/crystal structures, a portal/entrance, a broken bridge or equivalent blocked crossing,
   and at least three deliberately differentiated material/colour roles. A renamed Basin clearing is
   not enough.
4. At the traversal console, the ordinary interaction key is `R`. A non-Volt active companion cannot
   power the bridge; persistent text explains the exact missing element. A Volt active companion can
   power it. The powered state is visible through geometry/material change and remains available in
   equivalent persistent text rather than colour or motion alone.
5. Powering the bridge opens a traversable route and advances the objective to the authored encounter.
   The encounter is a deterministic, explicitly identifiable wild `nullix` placed beyond the gate—not a
   lucky random spawn. It participates in the existing battle path: pressing `F` in range enters battle
   against Nullix. Fleeing or resolving battle returns to the cavern with the powered traversal state
   intact.
6. A clearly named return portal uses `R` and restores `game.biome === 'basin'`, Basin actors, a useful
   survey objective and a safe position near the north entrance. Re-entering retains bridge completion
   for the current page session. Reload begins from the unchanged intro and a fresh expedition state.

## Native review contract

The live browser is the primary review target. Extend the existing read-only `window.__cosmon` test
surface with a `prismSnapshot()` function that returns serialisable current truth only:

- biome;
- entrance/room/bridge/console/return-portal presence and visibility;
- bridge powered and traversable state;
- authored encounter species ID and authored marker;
- interaction positions for the entrance, console, encounter and return portal;
- persistent objective and status text; and
- Basin actor visibility.

Do not add a test-only mutation API. The evaluator will choose a starter, position the existing player,
switch ordinary team state, and dispatch the documented `R`/`F` keys through the live page. Keep the
snapshot derived from the same runtime objects and DOM the player sees.

## Acceptance

- The exact journey above works in prepared headless Chromium at 1440×900 with zero page, console or
  failed-request errors.
- A fresh 390×844 render keeps persistent objective/status text and interaction prompt inside the
  viewport without horizontal document overflow.
- The room contains no external asset/network dependency and uses the existing vendored Three.js.
- Existing battle, roster/evolution and starter behavior remain intact.
- Add the smallest focused executable proof useful to the implementation; do not replace live review
  with source-string, object-count or proxy-only tests.
- Leave one clean meaningful candidate commit. Do not modify screenshots or add generated evidence to
  the product tree.

## Product judgement

Keep scope to one room, one Volt traversal, one authored encounter and one return. Prefer a cohesive,
legible expedition over more content. The final candidate should be reviewable by playing it, with
commits/tests as supporting evidence.
