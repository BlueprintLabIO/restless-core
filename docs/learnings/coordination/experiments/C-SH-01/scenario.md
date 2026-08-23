# Owner outcome — make the research cairn easy to navigate to

The existing Sunleaf Basin already contains a research cairn at world position `(18, 6)`, exposed as
`game.world.props.cairn`. Make it an unmistakable exploration destination through two complementary,
independently useful affordances. Do not add a new screen or dependency.

## Observable success contract

1. Add a clearly visible, softly animated in-world beacon centred above the existing cairn. Keep the
   cairn itself intact. Expose the rendered review handle as `game.world.props.cairnBeacon` so the
   native target can be inspected without guessing scene internals.
2. Add a compact exploration-HUD marker exposed as `#cairn-nav`. Its accessible text must identify
   **Research Cairn** and continuously show rounded distance plus one of eight bearings:
   `N`, `NE`, `E`, `SE`, `S`, `SW`, `W`, `NW`.
3. Use the basin convention **north = negative Z, east = positive X**. The marker must update when the
   player moves; it may show `ARRIVED` instead of a bearing within 3 metres.
4. Keep the marker hidden before a starter is chosen and outside ordinary exploration. It must fit the
   existing HUD rather than obscuring the objective, team or interaction prompt.
5. Existing exploration, battle, combat and roster/evolution behaviour remains green. Leave one clean
   candidate commit.

The two seams are both part of the owner outcome but are independently acceptable: the world beacon is
a useful diegetic landmark without the HUD, and the live HUD bearing is useful without the beacon. The
native review target is the running basin viewed through the browser. Do not create management
documents or broaden the milestone.
