# S08-T10 · Make Work legible as one graph and one board

**Layer:** Owner surface + OrgIntel projection.  
**Serves:** Sprint 08 criterion 17.  
**Depends on:** —  
**Observed friction:** The current Work implementation contains a SvelteFlow/Dagre dependency-map
candidate and a separate board projection, but neither has been accepted against representative
real Work. There is
no written contract proving that both lenses share the same Work, Attempt, edge, artifact and gate
truth; `requires` and `revises` remain legible under branching and return paths; current progress and
liveness are observed rather than invented; or the layout-engine branch ends with one retained
implementation.  
**Makes deletable:** duplicate map/board derivation, invented percentage progress, status-only busy
animation, the losing layout engine, custom edge-routing experiments and any fixture adapter used
only by the losing candidate.

## Outcome

The owner can open Work and answer two different questions without entering two different systems:

1. **Map:** what depends on what, what returned for revision, and where the current outcome path is.
2. **Board:** what is next, in motion, waiting and recently landed.

Both lenses are read-only projections over one OrgIntel snapshot and link each item to the same Work
detail. The graph is the primary causal view; the board is the denser scanning view. Neither owns
status, progress, ordering or liveness.

## Shared projection

Build one bounded owner-facing projection from the existing source-owned concepts:

- goal and Work identity;
- outcome/title, accountable actor and priority;
- Work revision/status;
- latest Attempt selected by source-owned ordering, not array accident;
- `requires` and `revises` edges;
- expected and available artifacts/evidence;
- gate result and blocker;
- latest meaningful update;
- optional observed current activity with source and observation time.

Map, board and Work detail consume this projection without independently reclassifying Work. The
board labels may use owner language such as `Next`, `In motion`, `Waiting` and `Recently landed`, but
those columns remain a projection over source statuses rather than new states.

## Map behaviour

1. Render a calm left-to-right dependency map for the selected goal or company path.
2. Give `requires` one strong forward direction and `revises` a materially different return treatment;
   colour alone is insufficient.
3. Keep arrowheads outside node bodies, avoid edge labels colliding with nodes and make fan-in/fan-out
   traceable without decorative motion.
4. Nodes lead with the Work outcome and show accountable actor, revision/status, latest Attempt and a
   compact evidence/gate signal. Continuous logs and raw tool calls stay out of cards.
5. Selecting a node opens the existing Work detail with outcome contract, exact Attempt inputs,
   evidence, gates and relationships. Preserve selected goal/lens context on return.
6. Fit the useful graph initially while retaining pan/zoom only where the graph exceeds the calm
   surface. Do not add editable handles, graph mutation or persisted free-form positioning.

## Board behaviour

1. Render `Next`, `In motion`, `Waiting` and `Recently landed` from the same visible Work rows used by
   the map. Completed history is progressively disclosed.
2. A board card shows the same revision/status, actor, Attempt and evidence meaning as its graph node.
3. Card selection opens the same Work detail route. There is no drag-to-change status or board-owned
   transition.
4. Preserve goal filtering and truthful unassigned Work. Empty columns say `Clear`; an unavailable
   source does not masquerade as an empty column.

## Progress and liveness

- Progress is categorical and inspectable: revision, Attempt state, prerequisite release, gate
  result and evidence availability. Do not calculate completion percentages from arbitrary row
  counts or animate a progress bar without a source-owned measure.
- Work status `active` means in motion, not necessarily executing at this instant.
- `live/current step` requires an observed session/process signal associated with the Work or latest
  Attempt and includes `observed_at` plus source availability.
- Observed, stale, unknown and unavailable are distinct. Unknown does not become idle; stale does not
  keep animating; source failure does not become zero active Work.
- Both lenses show the same bounded semantic activity. Raw tool names and continuous event streams
  remain behind detail/debug surfaces and never compete with outcomes.
- Respect reduced motion and stop ambient animation while the document is hidden.

If Sprint 08 has no trustworthy session/process projection, ship honest Work/Attempt progress and
label it `in motion`; record live activity as a pending evidence gap rather than synthesising it in
the browser.

## Layout-engine branch and purge

Run the current SvelteFlow/Dagre candidate and the smallest credible alternative against the same
fixtures and one live Sprint 08 graph. The alternative may be a lighter deterministic SVG/HTML rail
or another mature layout engine only if it requires less machinery for the observed topology.

Compare:

- chain, branch, fan-in, revision return, blocked/disconnected and completed-history legibility;
- arrow direction, crossings, node overlap and useful initial viewport;
- keyboard/pointer interaction and reduced-motion behaviour;
- production bundle delta and client-only loading cost;
- code and maintenance surface;
- whether the candidate introduces editing/game/workflow concepts Restless does not need.

Record the result in the Sprint 08 run report. Retain one layout path and remove the losing engine,
adapter, CSS and lockfile dependency. Do not keep both behind a switch.

## Verification

- Seed one `_test` graph containing a linear handover, a branch and fan-in, a `revises` return, a
  blocker, disconnected Work and evidence-backed completion. Assert both lenses contain the same
  Work ids, owner/status/Attempt/evidence meaning and detail URLs.
- Assert every visible edge has the correct endpoints/kind and that the semantic direction does not
  depend on colour. Verify node/card selection and back navigation preserve goal and lens.
- Seed observed, stale, unknown and unavailable liveness. Assert only observed activity receives the
  live treatment and that stale/unknown/unavailable text remains explicit in both lenses.
- Exercise source loading and failure: the page does not claim an empty graph/column before a source
  answers and does not discard the last truthful status without labelling degradation.
- Run the frontend type/check suite and a headless browser pass at the supported owner viewport.
- Perform one visual review of the representative graph and the live Sprint 08 sourcing/payment
  graph. A single linear fixture or green snapshot test cannot choose the engine.
- Measure the production bundle before and after purging the losing candidate; record the retained
  dependency and observed delta rather than quoting registry package size.

## Risks

- **The board becomes a second writer — invariant:** no drag mutation, separate columns or local
  transitions exist; it is a projection over OrgIntel.
- **Beautiful motion becomes fake progress — invariant:** only observed live activity animates;
  Work/Attempt state remains visible without pretending it is a process heartbeat.
- **The graph becomes an agent-surveillance surface — guarded:** cards lead with outcomes, handovers,
  evidence and blockers; continuous activity and raw tools stay out.
- **Layout comparison becomes permanent abstraction — invariant:** one candidate is deleted before
  the sprint closes.
- **The lightest engine produces the least readable graph — accepted:** choose by the smallest
  implementation that passes representative topology and owner comprehension, not package size
  alone.
- **A dense graph is unreadable on narrow screens — accepted for V0:** preserve selection and detail
  access, offer the board as the narrow scanning lens and record real owner friction before adding
  collapsed groups, minimaps or persisted layouts.
