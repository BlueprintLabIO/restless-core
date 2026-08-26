# Final evidence

Checked 26 August 2026 in `/Users/yao/Learning/restless-exp07-codex-greenfield`.

## Review target

Run `npm ci && npm run dev -- --port 4173`, then open `http://localhost:4173/`.

For the production container, run `docker build -t restless-exp07-arm-c:review .` followed by
`docker run --rm -p 8087:80 restless-exp07-arm-c:review` and open `http://localhost:8087/`.

## Routes

- `/`
- `/product/`
- `/how-it-works/`
- `/research/`
- `/compare/`
- `/journal/`
- `/journal/one-worker/`
- `/journal/demand-shape/`
- `/journal/supervision/`

## Exact checks run

- `npm run verify`: passed. Vite built all nine HTML entries. The verification script confirmed every
  built route, title marker, main landmark, H1, skip link, public-copy em dash constraint and shared
  asset output.
- In-app browser route matrix at `1440x1000`: all nine routes loaded with their expected pathname,
  distinct title and H1; each exposed a main landmark, labelled primary navigation and skip link; none
  had document-level horizontal overflow.
- In-app browser route matrix at `390x844`: the same nine checks passed at every route; none had
  document-level horizontal overflow.
- Home interaction: choosing `Account research` changed the selected state to
  `false, true, false` and returned `40 accounts qualify. Approve the prepared first outreach for the
  top six.`
- Mobile menu: activating `Menu` changed `aria-expanded` to `true` and made the primary navigation
  visible.
- Keyboard focus: the skip link reported a solid 3px `rgb(239, 76, 58)` outline with 4px offset after
  keyboard focus.
- Mobile comparison: the document remained 390px wide while the 780px comparison table scrolled in
  its 370px local region.
- Browser console: no errors or warnings observed on the inspected home interaction.
- `rg` public-copy check: no em dash characters in HTML, JavaScript or CSS.
- `docker build -t restless-exp07-arm-c:review .`: passed using Node 22 Alpine and nginx 1.27 Alpine.
- Running container route probe on port 8087: HTTP 200 observed for all nine routes. The container was
  then stopped; its health check was still in its initial startup interval when inspected.

## Design choices

The design thesis is a quiet dispatch desk. Cool paper, near-black operational fields, compressed
Archivo and editorial Newsreader keep the publication serious without resembling a generic dark AI
dashboard. One vermilion signal marks the scarce moment when owner judgement enters.

The home signature is an interactive workline: owner direction enters once, company work happens in
the middle, and one prepared native decision exits. It demonstrates the product boundary with real
product language and remains legible without JavaScript. Motion is limited to one scroll reveal and
is removed under `prefers-reduced-motion`.

Three explored directions preceded implementation: a dark control room, a dense research broadsheet
and the quiet dispatch desk. The control room was rejected as interchangeable SaaS imagery. The
broadsheet was rejected because it made the research, rather than the owner outcome, the product.

## Product and comparison sources

- `BRIEF.md`, `PRODUCT-TRUTH.md` and `EVIDENCE.md` supplied the product and research claim boundary.
- [opencompany product site](https://www.opencompany.cloud/), accessed 26 August 2026.
- [Lindy product site](https://www.lindy.ai/) and [Lindy documentation](https://docs.lindy.ai/),
  accessed 26 August 2026.
- [CrewAI product site](https://crewai.com/) and
  [CrewAI introduction](https://docs.crewai.com/en/introduction), accessed 26 August 2026.
- The implementation used the frontend-design skill as its design-quality frame. No external visual
  assets or source code were copied. Archivo and Newsreader are bundled through Fontsource packages.

## Known limits

- This is a static research publication. The demonstration uses representative scenarios and does not
  connect to a live Restless company or imply general availability.
- The category comparison is a dated reading of public first-party material. It does not measure
  reliability, quality, cost or owner attention under matched workloads.
- Social metadata uses a root-relative SVG preview. Some social crawlers require a deployed absolute
  image URL, which cannot be supplied truthfully before a public origin exists.
- The nginx health check was defined and the container served all routes, but the manual probe occurred
  before the first 30-second health interval completed.
