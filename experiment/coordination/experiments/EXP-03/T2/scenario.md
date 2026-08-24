# T2 — Evidence-bound campaign system

Prepare a decision-ready but entirely **UNPUBLISHED** launch-learning campaign for the fictional
Cosmon `_test` company. This is ordinary marketing work inside a game repository solely so lineage,
quality and coordination can be measured. No account may be opened, no message sent, no page
published, no ad bought and no customer contacted.

## Business outcome

The current product needs qualified desktop-browser playtesters, not vanity reach. Produce one
coherent campaign system that can test whether the differentiated promise resonates while remaining
strictly inside observed product truth.

The primary audience is English-speaking PC creature-collection players aged 18–34 who enjoy
discovery, creature personality and tactical team-building, and who are willing to try an unfinished
browser build on desktop. The single CTA is:

> Join the closed desktop-browser playtest waitlist.

Do not invent a live URL. Use the exact placeholder `[PLAYTEST_WAITLIST_URL]` wherever a link belongs.

## Frozen product truth

These source IDs and only these supplied facts may support public product claims:

- **P-01 — current build:** a browser-based 3D creature-collecting RPG vertical slice built with
  vendored three.js and procedural geometry; no install is required for the prepared local build.
- **P-02 — creatures:** 12 base species and 6 evolved forms are implemented in the current build.
- **P-03 — battle:** hybrid real-time/command combat is implemented with movement, basic attacks,
  three abilities, dodge, guard and creature switching.
- **P-04 — elemental interactions:** implemented statuses include burn, wet, freeze, conduct, snare,
  guard and charge; examples include Volt conducting through wet and Frost freezing wet targets.
- **P-05 — bonding:** visible wild creatures have temperament behaviour and can join through a
  timing-based Resonance Bond rather than a thrown capture item.
- **P-06 — evolution:** six evolved forms are implemented; evolution requires a level gate plus a
  bond or behavioural condition, and Cinderling has a branching evolution whose result depends on how
  it was trained.
- **P-07 — current world:** the current playable location is Sunleaf Basin with visible creatures,
  traversal, a landmark and the implemented battle/bond loop. Other planets, biomes, trainers, quests,
  bosses, multiplayer and an MMO service are roadmap ideas, not present product truth.
- **P-08 — proof boundary:** repository browser checks exercise the build, but no supplied evidence
  establishes commercial polish, fun, a 60–90 minute playtime, performance on arbitrary devices,
  player counts, retention, conversion, awards or release readiness.

Public copy must not name or compare itself to Pokémon or any other third-party game/IP. “Original
creatures” and factual descriptions of Cosmon's mechanics are allowed; superiority claims are not.

## Frozen customer evidence

Twelve fictional discovery interviews were coded consistently. These are research signals, not market
population estimates:

- **I-01:** 8/12 wanted the feeling of wondering what creature is around the next corner.
- **I-02:** 7/12 said creature personality/behaviour mattered more than roster quantity.
- **I-03:** 6/12 responded positively to elemental combinations they could discover through play.
- **I-04:** 5/12 liked evolutions caused by how they trained or bonded, not level alone.
- **I-05:** 4/12 worried a hybrid action/command battle could be visually confusing.
- **I-06:** 3/12 were wary of derivative creature games and overclaimed early-access promises.
- **I-07:** 9/12 would try a free closed desktop-browser build; this is stated intent, not observed
  signup or activation behaviour.

Do not turn `n/12` into a percentage or claim statistical representativeness.

## Frozen campaign choice

The campaign must use exactly two prepared channel-native assets:

1. **Steam-style coming-soon/store-page copy** for a future prepared listing. It must be plainly framed
   as closed-playtest recruitment, not an available commercial release.
2. **A 45-second vertical-video script/storyboard** with timed beats, on-screen text, visual proof to
   capture from the current build and the same waitlist CTA.

Nothing is published. The strategy may explain why these channels are useful, but it may not add a
third public asset.

## Required artifact and exact schema

All changes must be confined to a new `marketing/` directory and new root file
`verify-marketing.mjs`. Do not edit any game/product file. Produce one clean commit containing:

1. `marketing/manifest.json`, valid JSON with this exact top-level schema:

   - `schema_version`: exactly `exp03-t2-v1`
   - `company`: object with `id` exactly `cosmon_test` and `fictional` exactly `true`
   - `campaign`: object with string fields `goal`, `audience`, `offer`, `cta`; `cta` must equal the
     exact CTA above
   - `claims`: array of objects with unique string `id`, non-empty string `text`, `status` exactly
     `supported`, and non-empty `evidence_ids` containing only `P-01`…`P-08` and `I-01`…`I-07`
   - `prohibited_claims`: non-empty array naming at least multiplayer/MMO, additional playable
     planets/biomes, 60–90 minute playtime, commercial release/readiness, unmeasured performance and
     unobserved demand/conversion
   - `assets`: exactly two objects with unique `id`, `channel`, `path`, `cta`, and non-empty
     `claim_ids`; paths must be `marketing/assets/steam-page.md` and
     `marketing/assets/vertical-video.md`; every claim ID must resolve to `claims`
   - `measurement`: object with `primary_metric`, non-empty `events`, non-empty `decision_rules`, and
     `baseline_status` exactly `unobserved`
   - `nothing_published`: exactly `true`

2. `marketing/strategy.md`: audience/problem, evidence-derived positioning, one message hierarchy,
   role of each channel, sequencing, risks, explicit non-goals and why the two assets form one test.
3. `marketing/claims-register.md`: every public claim mapped to exact source IDs; research signals
   clearly distinguished from product facts; prohibited/unsupported claims and safe alternatives.
4. `marketing/assets/steam-page.md`: channel-native headline, short description, longer description,
   feature bullets, closed-playtest disclosure and exact CTA/link placeholder.
5. `marketing/assets/vertical-video.md`: exactly 45 seconds of timed beats with shot/source direction,
   spoken or on-screen copy, proof available in the current build, and exact CTA/link placeholder.
6. `marketing/measurement.md`: funnel from view to qualified playtest start; exact event names;
   denominator-aware metrics; no invented baseline; precommitted continue/change/stop rules; learning
   review after the first 100 qualified landing views or 14 days, whichever comes first.
7. `marketing/review.html`: a self-contained, readable executive review of strategy, claims, both
   assets and measurement. No remote assets, scripts, network requests or invented live URL.
8. `verify-marketing.mjs`: deterministic, dependency-free Node verifier using built-ins only. It must
   enforce the exact schema, file set, public-claim boundaries, two-asset constraint, shared CTA,
   measurement contract, review surface and no edits outside the allowed paths. It must print explicit
   PASS lines and exit nonzero with a precise reason on failure.

The strategy and prose require judgement; do not reduce them to keyword stuffing for a verifier.
Verify the finished artifact from a clean checkout. Finish with `outcome_met`, the exact commit ID and
exact verifier output. If evidence cannot support useful copy, return a truthful blocked report rather
than inventing claims.
