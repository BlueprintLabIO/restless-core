# Owner outcome — Lumaara release evidence decision

Produce a founder-ready research pack that recommends one primary next-release direction for the
Lumaara vertical slice and a bounded supporting sequence. Use only the frozen experimental source
corpus below. The corpus is synthetic evaluation data, not a claim about real players or a live
market; label that limitation prominently and do not browse or import outside facts.

The pack contains five independently reviewable evidence artifacts plus the final synthesis:

- `research/evidence/play-telemetry.md` for P01–P08;
- `research/evidence/player-interviews.md` for Q01–Q08;
- `research/evidence/accessibility-usability.md` for A01–A08;
- `research/evidence/production-support.md` for O01–O08;
- `research/evidence/source-ledger.csv`, with one row per source ID; and
- `research/lumaara-release-decision.md`, the lead-owned founder decision memo.

Each regional dossier must be useful without the final memo: explain the region's strongest claims,
quality/limitations, contradictions, implications for all four choices, and questions the region
cannot answer. Cite every record in its region. The ledger maps each exact ID to region, supported
claim, counterevidence/limitation, affected release choices and evidence strength. Do not merely copy
the source card text into either artifact.

This is a source-synthesis outcome, not a coding task. Do not change the game. Leave one clean commit
containing the complete research pack.

## Decision choices

- **Prism Expedition** — a new cavern habitat, traversal objective and authored guardian encounter.
- **Battle Mastery** — deeper enemy AI, status interactions and readable combat feedback.
- **Field Research** — discovery, comparison and squad-planning tools around the existing roster.
- **Accessible Journey** — input, readability, onboarding and low-friction play improvements across
  the existing loop.

Choose exactly one as the primary release direction. Supporting work may borrow from another choice,
but the memo must preserve a clear sequence rather than merging everything into an unfalsifiable
roadmap.

## Required memo

1. Start with an executive decision naming the primary direction, target owner/player outcome and why
   it outranks the three alternatives.
2. Synthesize the four regional dossiers rather than repeating them. The final memo must show how play
   telemetry, player interviews, accessibility/usability audits, and production/support constraints
   jointly change the decision.
3. Include a decision matrix with all four choices, explicit criteria, evidence citations and visible
   trade-offs. Explain the weighting rather than hiding judgement in one score.
4. Cite every source record at least once using its exact bracket ID, for example `[P01]`. Do not cite
   an ID absent from this corpus.
5. Reconcile at least four concrete cross-source tensions. State which evidence wins, why, and what
   remains uncertain.
6. Propose a bounded release sequence with success signals, stop conditions, dependencies and the
   smallest useful first slice.
7. Include risks, counterevidence, limitations and a falsifier that would reverse the recommendation.
8. End with an evidence appendix mapping every source ID to the claim it informed, and link the four
   dossiers plus source ledger so coverage is inspectable rather than implied.

## Frozen experimental source corpus

### Region P — play telemetry summaries

- **P01.** Across 2,400 synthetic first sessions, 61% chose a starter, 44% entered one wild encounter,
  27% completed a bond and 11% reached a second encounter. The largest single funnel loss occurred
  between starter choice and recognising the nearest useful world action.
- **P02.** In 680 synthetic battle sessions, players used basic attack in 92%, an ability in 58%, dodge
  in 31%, guard in 9% and creature switching in 6%. Battles won after at least one deliberate status
  interaction were 38% longer but had 24% higher replay intent in the post-session prompt.
- **P03.** In a roster-recognition test, only 36% of participants could correctly recall their active
  creature's role after play, while 72% recalled its silhouette and element. Evolution requirements
  were correctly described by 18% without opening implementation notes.
- **P04.** A prototype static field guide was opened by 54% of exposed participants and raised correct
  evolution-answer accuracy from 18% to 57%, but it did not materially increase completion of the
  existing first bond during the same session.
- **P05.** A prototype objective beacon reduced median time from starter choice to first wild encounter
  from 164 seconds to 93 seconds. The improvement held on keyboard and controller fixtures; phone
  touch input was not represented.
- **P06.** Prism Cavern concept screenshots received the highest stated curiosity of the four release
  choices, but participants saw no playable cavern and could not assess traversal clarity, encounter
  quality or performance.
- **P07.** After three synthetic sessions, roster-planning participants constructed more elementally
  diverse squads than the control, yet 41% still could not explain why one status combination worked.
- **P08.** Accessibility-fixture users with reduced motion or 200% zoom abandoned the current intro at
  2.3 times the reference-fixture rate. This comparison is directional because fixture groups were not
  randomly assigned.

### Region Q — moderated player-interview summaries

- **Q01.** Nine of twelve interviewees described finding and understanding creatures as the game's
  distinctive promise; seven wanted a reason to care about a species before optimising a battle team.
- **Q02.** Eight interviewees asked for a new place to explore before more menus. Five of those eight
  also failed to notice the existing bond prompt until a moderator pointed it out.
- **Q03.** Skilled action-game players liked dodge timing and status combinations but called enemy
  intentions inconsistent. Less experienced players could not separate damage feedback from status
  feedback during crowded effects.
- **Q04.** Participants valued evolution being tied to behaviour rather than level alone. Ten of twelve
  wanted requirements discoverable; four explicitly rejected a complete spoiler list before meeting a
  creature.
- **Q05.** Squad-planning language varied. Some participants reasoned in roles, others in favourite
  creatures or status pairs. A single numeric team score was distrusted by seven interviewees.
- **Q06.** Controller users wanted remapping and persistent prompts; keyboard users mainly wanted the
  first objective made obvious. Neither group asked for additional combat commands in the first hour.
- **Q07.** The Prism Cavern pitch was memorable when framed around freezing streams, powering ancient
  doors and creature habitats. It became generic when described only as a larger map with more fights.
- **Q08.** Interviewees tolerated rough visuals more readily than losing progress, unclear save state
  or controls that changed between exploration and battle without explanation.

### Region A — accessibility and usability audit summaries

- **A01.** At 390×844 the intro remains operable, but three persistent HUD regions compete with the
  starter choice and two tap targets fall below the 44-pixel audit reference.
- **A02.** At 200% zoom, battle command labels wrap into the active-creature status region. No action is
  technically unreachable, but the relationship between label and control becomes ambiguous.
- **A03.** Keyboard focus reaches the starter cards and primary buttons, but the current visible focus
  treatment is lost against two bright glass surfaces.
- **A04.** Several battle and status meanings depend on colour plus transient motion. Reduced-motion
  mode suppresses useful emphasis without providing an equivalent persistent textual cue.
- **A05.** The first exploration objective is expressed through world placement, a short prompt and a
  temporary cue. Screen-reader order does not expose one durable objective statement after the cue
  disappears.
- **A06.** The bond minigame has a keyboard path, but its timing window and failure recovery are not
  described before the first attempt. Audit participants interpreted the first miss as a hard lockout.
- **A07.** Species names, elements and roles are available in roster surfaces, but evolution progress
  and status-combination explanations are dispersed across different states of play.
- **A08.** A no-new-content accessibility pass could repair all critical audit blockers, but the audit
  cannot predict whether that pass alone improves longer-term replay or perceived product novelty.

### Region O — production and support summaries

- **O01.** The current slice has one world biome and 18 species forms. Existing deterministic suites
  cover battle, combat-extra and roster/evolution, totalling 48 checks before any new release work.
- **O02.** A bounded Field Research prototype can reuse current roster and element modules. Its main
  risk is creating copied domain data or a second evolution model that silently drifts.
- **O03.** Prism Expedition requires new world geometry, traversal state, encounter scripting and
  performance proof. Its seams are parallelisable, but integration touches the world loop, creature
  placement and objective feedback together.
- **O04.** Battle Mastery changes the most regression-sensitive module. Existing checks cover core
  paths, but visual telegraph clarity and emergent status combinations still require native review.
- **O05.** Accessible Journey spans presentation modules but can be delivered as independently
  reviewable fixes. The highest-risk item is preserving equivalent meaning under reduced motion rather
  than merely disabling animation.
- **O06.** Support simulations attribute 46% of first-contact questions to controls/objective clarity,
  29% to evolution/status understanding, 15% to save/progress confidence and 10% to crashes or loading.
  These are synthetic classifications over 180 scripted contacts.
- **O07.** The smallest independently shippable Prism slice is one cavern room, one elemental traversal
  interaction and one authored encounter. Shipping the full habitat first would at least triple the
  estimated verification surface.
- **O08.** The release window can support one primary bet plus two enabling fixes. It cannot fairly
  absorb all four choices. A delayed release has no external contractual penalty in this experiment,
  so quality and learning value may outweigh date optics.

## Prepared evidence

Run the external evaluator plus any memo-quality checks you create. The exact evaluator is held outside
actor workspaces. Producer narration is not evidence; the final memo and its traceable citations are.
