# EXP-06 Codex arm evidence

Review target: the local production preview started from `site/` with
`npm run preview -- --host 127.0.0.1` after the final build. The final repair probe used the preview
reported by Astro at `http://127.0.0.1:4402/`.

## Checks run

- `npm ci`: 282 locked packages installed, 0 reported vulnerabilities.
- `npm run verify`: the prose and visual quality gate passed; `astro check` reported 24 files with 0
  errors, 0 warnings and 0 hints; `astro build` produced 11 static pages.
- Production route probe against the local preview: every listed site route returned HTTP 200 and a
  missing route returned HTTP 404.
- Rendered responsive matrix after `document.fonts.ready`: all ten content routes were measured at
  390 x 844 and 1440 x 1000 CSS pixels. Every measurement matched its document client width exactly,
  for 20 of 20 passing route and viewport combinations. The final browser console had no warnings or
  errors.
- At 390 x 844, `/findings/four-departments-one-invalid-evaluator/` measured 390 px client width and
  390 px scroll width after the repair. Its mobile finding track now uses `minmax(0, 1fr)` instead of
  allowing the large headline's min-content width to widen the entire article. The mobile headline
  scale keeps “departments” intact at this width, with overflow wrapping retained as a narrow-screen
  fallback. The header, callout and article body each rendered at exactly 390 px. The other nine
  content routes also measured 390 / 390.
- At 1440 x 1000, every content route measured 1440 px client width and 1440 px scroll width.
- The five-link mobile menu opened correctly and marked the current route.
- The responsive product demonstration, liquid-metal hero, evidence links, native `details`
  disclosures, focus rules, skip link and reduced-motion stylesheet were checked in the production
  output.
- Final output size: 1.1 MB total. The largest generated asset was the 56 KB shared stylesheet.

## Routes

- `/`
- `/product/`
- `/how-it-works/`
- `/research/`
- `/compare/`
- `/findings/`
- `/findings/four-departments-one-invalid-evaluator/`
- `/findings/supervision-needs-to-stay-available/`
- `/findings/teams-need-a-crossover/`
- `/findings/work-graph-is-a-record/`
- `/404.html`

## Design and source calibration

The final pass consulted Beautiful UI for calm information density, Cult UI's Cutout Card and Hero
Liquid Metal for precise edge treatment and one responsive signature, and Svelte Animations as the
source-first motion comparison. The implementation remains Astro-native with no client framework or
new dependency. Current competitor descriptions were rechecked against their official product pages
on 26 August 2026. The comparison remains positioning, not a performance benchmark.

## Known limitations

Restless remains under active dogfood. The experiment findings come from controlled, isolated runs
and do not establish market leadership or broad model generality. The site was not published. Local
builds intentionally omit canonical metadata and use a relative social-preview path; a public build
must set `PUBLIC_SITE_URL` to emit truthful absolute URLs.
