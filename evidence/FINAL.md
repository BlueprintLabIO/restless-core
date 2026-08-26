# Restless greenfield candidate: final evidence

## Native review target

- URL: `http://127.0.0.1:8080/`
- Refresh command: `npm run build && npm run review:probe`
- Probe record: `evidence/review/probe.json`
- Captures: `evidence/review/home-mobile.png` and `evidence/review/home-desktop.png`
- The target is served from the exact built `dist/` candidate by `scripts/server.js`. `npm run review:probe` leaves this server running and writes its PID to `evidence/review/server.pid`.

## Checks run and observations

All commands ran from the repository root on 26 August 2026.

### `npm run verify`

Observed exit code 0. Output:

```text
Built 9 routes into dist/
VERIFY PASS: 9 routes, 2 viewports, no overflow, structure, keyboard skip and product interaction.
```

The verifier built the site, started the production-like static server, and opened every required route in native Chromium at 390x844 and 1440x1000. For each route it observed HTTP success, one `h1`, a `main` landmark, and `scrollWidth <= clientWidth`. It also tabbed from the top of the home page and observed the skip link as the first focus target, then changed the product demonstration to the Research example and observed the mission copy update.

The same run checked the generated public HTML for the prohibited em dash character and checked source/output presence for reduced-motion CSS, primary navigation, main landmarks and Open Graph image metadata.

### `npm run build`

Observed exit code 0. Output:

```text
Built 9 routes into dist/
```

The generated `dist/` output is committed.

### `npm run review:probe`

Observed exit code 0. The live probe started `http://127.0.0.1:8080/`, opened the exact built home page in native Chromium, and captured full-page screenshots. Its observations were:

- 390x844: HTTP 200, title `A company that keeps going | Restless`, expected home `h1`, scroll width 390 and client width 390.
- 1440x1000: HTTP 200, same title and `h1`, scroll width 1440 and client width 1440.

The exact timestamp and process ID are in `evidence/review/probe.json`.

### Production container

`command -v docker && docker --version` exited 1 with no output because Docker is not installed in this runtime. The production image was therefore not built or started here. The multi-stage `Dockerfile` builds with Node 24 and serves the generated files from nginx on port 80. This container path remains unverified in this environment.

## Routes

- `/`
- `/product/`
- `/how-it-works/`
- `/research/`
- `/compare/`
- `/journal/`
- `/journal/one-worker-default/`
- `/journal/parallel-thresholds/`
- `/journal/supervision-correctness/`

All are generated as independent static documents and work without client JavaScript. JavaScript progressively adds the mobile menu and home demonstration controls.

## Creative directions explored

1. **The attentive company, selected.** An editorial operating publication in warm paper, sharp black, electric blue and a single acid proof colour. The signature is an “attention exchange”: owner instruction enters on one side, the company’s framed, produced and repaired work appears in a dark handoff sheet, and only the irreducible decision returns. This made the product legible as a change in the owner’s day rather than an agent topology.
2. **The night operations room, rejected as generic.** A dark control-room interface with live status, glowing nodes and animated operational traces. It could dramatise autonomy, but it closely resembled the interchangeable dark AI SaaS dashboard explicitly warned against. It also made safety substrate look like the headline.
3. **The company newspaper, not selected.** A restrained broadsheet where every route read as a dispatch from an autonomous company. It supported the research-stage honesty and finding pages well, but the static editorial metaphor did not natively demonstrate delegation and the prepared last mile strongly enough on home.
4. **The owner’s cleared desk, considered.** A spatial still-life where scattered tasks progressively leave a physical desk and one decision card returns. It had emotional clarity but depended too much on animation and illustration to explain the system at mobile size.

Direction 2 was explicitly rejected as the most generic. Direction 1 kept the editorial confidence of Direction 3 and used one product-specific signature rather than filling the site with dashboard cards.

## Important design and product choices

- The top-level promise is “Your company keeps going.” It leads with owner attention and continuity, not architecture or safety.
- The home demonstration is a concrete, readable handoff. Its baseline state communicates framing, Staff production, repair and owner judgement without animation or JavaScript. Controls swap among launch, research and operations examples and never imply a real external action.
- Serif italics mark irreducible human judgement. Monospace marks observed state and boundaries. The visual grammar repeats across the product, research and finding pages.
- The product route makes the prepared last mile concrete with a preserved production-order state while clearly disabling the identity-bound action.
- The organisation route uses increasing indentation only at wide viewports to show a handoff without drawing a generic agent network. Mobile removes indentation to preserve reading order and width.
- Research and comparison copy labels intended product direction, controlled observations and interpretation separately. It does not claim adoption, availability or an unrun win.
- Every page has a unique title, description, canonical URL, Open Graph title/description/image, Twitter card metadata, semantic header/nav/main/footer landmarks, a skip link and visible focus treatment.
- Reduced motion disables animation and transitions. The entire required journey remains readable with JavaScript disabled.

## External research observations and sources

Observed on 26 August 2026. These are dated observations of public first-party pages, not claims that the products were run or independently verified.

1. [opencompany primary site](https://www.opencompany.cloud/). The page presented opencompany as an open-source, multiplayer AI workspace. It described a self-building wiki, workflows, model-agnostic sessions, connected sources and cloud sandboxes. The comparison interprets its centre as a shared workspace for humans and agents.
2. [Lindy primary site](https://www.lindy.ai/). The page presented Lindy as an “AI teammate” that connects to company tools, knows company context and does work for a team. It visibly offered hosted sign-up and named multiple integrations. The comparison interprets its centre as tool-connected AI teammates and automation.
3. [CrewAI introduction documentation](https://docs.crewai.com/en/introduction). The documentation described CrewAI as an open-source framework combining structured, event-driven Flows with autonomous agent Crews. The comparison interprets its centre as developer orchestration of multi-agent applications.
4. Google Fonts CSS was used to load Manrope, Newsreader and DM Mono at runtime. The local fallback stack keeps the site readable if that request fails. No external visual component or competitor identity was imported.

The comparison page states its observation date, links each primary source, distinguishes source description from Restless interpretation and says directly that no common benchmark was run.

## Accessibility and viewport review

- Native Chromium automated rendering covered every route at 390x844 and 1440x1000.
- Full-page home captures at both viewports were visually inspected. The desktop composition kept a clear hero-to-demo relationship, generous spacing and legible finding rows. The mobile composition preserved hierarchy, kept the handoff sheet inside the viewport and linearised all multi-column sections without clipping.
- The verifier observed no horizontal overflow on any route at either required viewport.
- Native links and buttons preserve keyboard operation. The first Tab reaches a visible skip link. All focusable elements receive a high-contrast blue focus ring.
- Primary and footer navigation are labelled. The mobile menu button exposes `aria-expanded` and `aria-controls`. Product choice feedback uses `aria-live`.
- Colour is not the only carrier of state: labels and text identify prepared, provisional and interactive states.
- `prefers-reduced-motion: reduce` removes pulse and transition effects. No content depends on motion.

## Known limits and unverified claims

- Docker was unavailable, so the nginx container and its health check were not executed. Browser verification used the included Node static server against the same committed `dist/` files.
- The public canonical domain is the clearly non-live placeholder `https://restless.example`. It must be replaced when an owner chooses a real publication domain. Nothing was deployed or published.
- Social metadata structure and the SVG preview are present, but no third-party social crawler was used.
- Google Fonts are a public network dependency. Fallback fonts work, but their exact line breaks differ. A production publication may choose to self-host licensed font files.
- Automated checks do not replace expert assistive-technology review. Screen reader behaviour, forced-colours mode and zoom beyond target viewport checks were not manually audited.
- The three findings are bounded controlled observations from the supplied evidence pack. They do not establish customer demand, general reliability, financial performance or superiority over the compared products.
- The home demonstration is illustrative and local. Its controls stage copy only and perform no company work or external effect.
