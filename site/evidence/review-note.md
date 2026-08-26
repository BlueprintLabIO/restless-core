# Public site review note

Checked 26 August 2026.

## Local review target

From `site/`:

```sh
npm ci
npm run dev -- --host 127.0.0.1 --port 4321
```

Open `http://127.0.0.1:4321/`. The product demonstration is also available directly at `http://127.0.0.1:4321/product/`.

## Sources and fair comparison

The product account follows `AGENTS.md`, `ARCHITECTURE.md` dated 23 August 2026, `LLM_CURE.md`, `docs/specs/owner-cockpit.md`, `docs/specs/orgintel.md`, `docs/specs/evaluation-dogfood.md`, `docs/FRONTEND_DESIGN_REFERENCES.md`, and the implemented owner models and design tokens in `web/src/lib/`. Product hypotheses are described as hypotheses. The site does not turn target architecture into a claim of market proof.

Competitor descriptions on `/compare/` were checked against vendor-controlled pages, not review sites:

- [OpenCompany](https://www.opencompany.cloud/): calls itself a multiplayer AI workspace with a self-building wiki, workflows, model-agnostic sessions, connected sources and cloud sandboxes. This supports the page's workspace framing.
- [Relevance AI](https://relevanceai.com/agents): presents teams of specialist agents built in plain language, with visual composition, AI building and MCP paths. This supports the builder/workforce framing.
- [Lindy](https://www.lindy.ai/blog/announcing-a-new-way-to-create-ai-employees): describes AI employees, triggers, actions and teams assembled like an advanced workflow.
- [Sintra AI](https://help.sintra.ai/en/articles/9607295-sintra-helpers-explained): source linked for the role-helper and shared-business-context description.
- [Manus](https://manus.im/blog/introducing-wide-research): source linked for its general-agent and parallel research description.

The comparison deliberately says what each alternative is good for and states that no controlled head-to-head evaluation has established Restless as better, cheaper or a market leader. OpenCompany is acknowledged as more productised for a shared agent workspace.

Maggie Appleton's original essay [The Dark Forest and Generative AI](https://maggieappleton.com/ai-dark-forest) argues that cheap generation floods the web with coherent but generic material and that specificity, evidence and original synthesis are ways out. That criticism informed the short concrete copy, named limitations, dated sources, restrained vocabulary and rejection of anonymous quotations. It was treated as an argument, not experimental proof about this site.

## Design decisions

The visual system comes from the operator product: IBM Plex typography, a sparse matrix wordmark, paper and cool-metal surfaces, status blue and owner-judgment violet. The liquid-metal hero is the one expressive signature. It is an inline SVG with CSS motion and a reduced-motion stop, not a video or third-party runtime.

The final pass consulted:

- [Beautiful UI](https://www.beautifului.dev/), specifically its calm composition of AI-native status, evidence, approvals and chat at high information density.
- [Cult UI Expandable Screen source](https://github.com/nolly-studio/cult-ui/blob/main/apps/www/registry/default/ui/expandable-screen.tsx), specifically continuity between states and keeping the transformed content primary.
- [Magic UI Shimmer Button](https://magicui.design/docs/components/shimmer-button), specifically the discipline of one resolved signature interaction. Its gradient shimmer, perpetual button motion and React dependency were rejected as a poor fit.

No component code was copied. The product demo uses native Astro, semantic tabs and a small inline script. The tabs expose Attention, Work, People and Company while preserving one outcome in context. Arrow keys move between tabs. Without JavaScript, the default Work result remains legible.

## Routes

The production build emitted 11 pages:

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

## Overflow repair

The confirmed defect was reproduced on the built `/findings/four-departments-one-invalid-evaluator/` page at 390 by 844 CSS pixels after `document.fonts.ready`: `clientWidth` was 390 and `scrollWidth` was 411. The mobile finding title used `18vw` type (70.2px at this viewport); the intrinsic width of “departments” expanded the finding grid track and its full-width header to 411.19px. This was not a site-wide container failure.

The narrow repair changes only the existing mobile `.finding-header h1` fluid type step, from `18vw` to `16.5vw`. At 390px the heading is 64.35px, still within the established display scale, while its longest word fits the 354.8px content measure. No overflow clipping, global word-breaking or page-wide width constraint was added.

`scripts/responsive-check.mjs` is the deterministic regression check. It discovers every built public `index.html` content route, starts a local static server unless `RESPONSIVE_CHECK_BASE_URL` is supplied, launches the installed Chromium through the pinned `puppeteer-core` development dependency, uses the two required viewports, waits for `document.fonts.ready`, prints route/status/client/scroll measurements and exits nonzero for a failed route or `scrollWidth > clientWidth`.

## Verification performed

From `site/`:

```sh
npm ci
npm run verify
node scripts/responsive-check.mjs
npm run preview -- --host 127.0.0.1 --port 4321
NODE_PATH=/usr/local/lib/node_modules node /tmp/restless-site-check.cjs
NODE_PATH=/usr/local/lib/node_modules node /tmp/restless-site-a11y-check.cjs
```

Observed native results:

- `npm run verify` ran the prose and visual quality gate, `astro check`, and the production build. The quality gate passed; Astro checked 25 files with 0 errors, 0 warnings and 0 hints; the build emitted 11 pages.
- The original browser probe requested all ten content routes. Every response was HTTP 200 with a non-empty title and H1.
- The original product interaction probe passed at 1440 by 1000 and 390 by 844: clicking Attention exposed its named panel, ArrowRight selected Work, and no console or page errors were recorded.
- The focused repaired-route probe waited for fonts at both viewports. One Tab focused the visible “Skip to content” link at `(12, 12)` with a computed 3px outline. The repaired route recorded no console or page errors.
- In a reduced-motion Chromium context, `prefers-reduced-motion: reduce` matched and the hero-copy, metal-core and orbit animation durations each computed to `0.00001s`.

## Deterministic route measurements

Each row below is `clientWidth / scrollWidth` after fonts were ready. Every navigation returned HTTP 200.

| Public content route | 390 × 844 | 1440 × 1000 |
| --- | ---: | ---: |
| `/` | 390 / 390 | 1440 / 1440 |
| `/compare/` | 390 / 390 | 1440 / 1440 |
| `/findings/` | 390 / 390 | 1440 / 1440 |
| `/findings/four-departments-one-invalid-evaluator/` | 390 / 390 | 1440 / 1440 |
| `/findings/supervision-needs-to-stay-available/` | 390 / 390 | 1440 / 1440 |
| `/findings/teams-need-a-crossover/` | 390 / 390 | 1440 / 1440 |
| `/findings/work-graph-is-a-record/` | 390 / 390 | 1440 / 1440 |
| `/how-it-works/` | 390 / 390 | 1440 / 1440 |
| `/product/` | 390 / 390 | 1440 / 1440 |
| `/research/` | 390 / 390 | 1440 / 1440 |

## Known limitations

- This is a local review candidate. It was not published or deployed.
- The deterministic responsive check covers the ten public content routes generated as `index.html`. The separately generated `/404.html` is an error document, not a content route, and was not included in the route matrix.
- Chromium was the only rendering engine inspected. Safari/WebKit and Firefox were not run.
- No Lighthouse, screen-reader or other assistive-technology session was run. Accessibility observations cover rendered semantics, keyboard tab behavior, focus indication, reduced motion, overflow and Chromium inspection, not certification.
- The responsive check treats document-level horizontal overflow as the failure. It also prints out-of-viewport descendant bounds for diagnosis; the home-page liquid-metal illustration intentionally extends inside an `overflow: hidden` hero and does not increase document `scrollWidth`.
- No production origin has been chosen, so the static candidate intentionally omits canonical and `og:url` values rather than inventing a domain. Social title, description, image and image alt are present, but no real social platform preview was tested.
- Competitor pages can change after the check date. The comparison is positioning from vendor descriptions, not a feature audit or purchasing recommendation.
- The numerical research observations come from repository experiment records and retain their stated conditions. They do not prove general market performance.
