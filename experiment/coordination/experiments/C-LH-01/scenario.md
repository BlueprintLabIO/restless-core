# Owner outcome — ship the Lumaara field-research desk

Extend the exact current Cosmon seed with two substantial companion tools that make the existing
18-species roster useful outside the live 3D loop:

1. a **Field Atlas** for finding, understanding and comparing species; and
2. a **Squad Workshop** for composing, analysing, saving and exchanging four-creature teams.

This is one release outcome but two independently valuable artifacts. Either tool must remain usable
and reviewable if the other is absent. Reuse the seed's existing species, abilities, evolution and
element rules as source truth; do not create a copied roster or a second matchup/evolution model that
can silently drift.

## Native review target

The reviewer starts at the existing starter-choice screen, opens each tool from a clearly named link,
uses it at 1440×900 and 390×844, returns to the survey, and then starts the unchanged game. Both tools
must be usable before a starter is chosen and must be linked from the normal product experience—not
orphan test pages.

For exact external review, expose these semantic DOM handles. They are review locators, not a required
internal architecture:

- launch links: `[data-companion-tool="atlas"]` and `[data-companion-tool="workshop"]`;
- common: `[data-tool-root]`, `[data-tool-title]`, `[data-return-to-survey]`;
- Atlas: `[data-atlas-search]`, `[data-atlas-filter]`, `[data-atlas-count]`,
  `[data-atlas-card]`, `[data-atlas-detail]`, `[data-atlas-detail-field]`,
  `[data-atlas-ability]`, `[data-atlas-evolution-target]`, `[data-atlas-compare-add]`,
  `[data-atlas-compare-slot]` and `[data-atlas-matchup]`;
- Workshop: `[data-workshop-species]`, `[data-workshop-add]`, `[data-workshop-slot]`,
  `[data-workshop-level]`, `[data-workshop-bond]`, `[data-workshop-ult]`,
  `[data-workshop-forecast]`, `[data-workshop-remove]`, `[data-workshop-move]`,
  `[data-workshop-team-count]`, `[data-workshop-analysis]`, `[data-workshop-save-name]`,
  `[data-workshop-save]`, `[data-workshop-saved]`, `[data-workshop-load]`,
  `[data-workshop-delete]`, `[data-workshop-json]`, `[data-workshop-export]`,
  `[data-workshop-import]` and `[data-workshop-error]`.

Use species IDs as the values of `data-atlas-card`, `data-atlas-compare-add`,
`data-workshop-species` and `data-workshop-add`; expose the current species ID on each Workshop slot.
Use `data-atlas-filter="element|role|tier|temperament"`,
`data-atlas-detail-field="name|element|role|temperament|tier|growth|blurb"`,
`data-atlas-compare-slot="a|b"`, `data-atlas-matchup="a-to-b|b-to-a"`,
`data-workshop-move="up|down"`, and
`data-workshop-analysis="roles|elements|strengths|risks|recommendations"`.

## Field Atlas contract

1. Show every exact seed species once: 12 base forms and six evolved forms. The visible count updates
   with the current result set.
2. Search across at least name, blurb and ability name. Combine search with element, role, tier and
   temperament filters; an explicit all/any option clears each filter.
3. Selecting a result opens a readable detail view with name, element, role, temperament, tier, blurb,
   growth text and all three abilities including their names and descriptions.
4. Show evolution relationships and requirements from the live roster. In particular, Cinderling must
   show both Infernox and Pyrelisk with their distinct bond/signature-use requirements; evolved forms
   must visibly identify their base family.
5. Let the reviewer place two distinct species into comparison slots A and B from the result cards.
   Show both complete identities/ability sets and the elemental multiplier in both attack directions,
   derived from the existing element rules. Adding the same species twice must not create a false
   two-species comparison.
6. Empty searches, no-result states and incomplete comparison are calm, explicit states—not broken or
   blank UI.

## Squad Workshop contract

1. Offer every exact seed species. Build an ordered squad of zero to four distinct creatures; make
   add, remove, move-up and move-down behavior obvious. A duplicate or fifth member is rejected with a
   visible explanation and without corrupting the current team.
2. Each slot has editable level (1–12), bond (0–8) and signature-use count (0–12). Show a live
   evolution forecast from the existing rules. Cinderling at level 7/bond 4/zero signature uses must
   forecast Infernox; at level 7/bond 1/six signature uses it must forecast Pyrelisk.
3. Analyse the current squad live: represented roles and elements, offensive strengths, elemental
   risks and short actionable recommendations. Empty and partial squads get useful, truthful guidance;
   the analysis must change when composition changes.
4. Save a non-empty squad under a reviewer-supplied name. Named saves survive page reload, can be
   loaded exactly—including order and per-slot values—and can be deleted. Saving an empty or unnamed
   squad is rejected without destroying existing saves.
5. Export the current squad as versioned JSON in the visible exchange field. Importing that JSON
   restores the exact ordered squad and values. Invalid JSON, unknown species, duplicates, out-of-range
   values or more than four members are rejected visibly and atomically: the current squad remains
   unchanged. Version 1 uses this exact public exchange shape so other tools can consume it without
   guessing: `{"version":1,"members":[{"speciesId":"cinderling","level":7,"bond":4,"ultUses":0}]}`.

## Shared product quality

- Both tools are keyboard operable, have visible focus, labelled controls, semantic headings and
  useful live-region/status announcements. Do not use colour alone for meaning.
- At phone width there is no horizontal document overflow, clipped primary control or unusable fixed
  desktop layout.
- Match the existing quiet glass-and-light visual language with minimalist typography. Do not add
  decorative eyebrow/kicker labels or replace explanations with persistent subtitles; use concise
  labels and hover/focus tooltips where supplementary explanation is needed.
- The tools may share small presentational utilities, but their feature code, state and storage keys
  remain independent. Neither tool may mutate the live game's team, capture or progression state.
- No new runtime dependency, network service, build step or generated asset is needed to review the
  exact candidate.
- Preserve the starter flow, exploration, bond, battle, combat/status and roster/evolution behavior.

## Prepared evidence

Leave one clean candidate commit. Add focused native browser verification for each companion tool and
run it alongside the existing battle, combat-extra and roster/evolution suites. The exact external
evaluator is held outside actor workspaces; producer-authored checks are evidence, not the owner gate.
