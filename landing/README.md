# Restless Core — public page

The open-source front door. One static page built from the product's own components.

```sh
cd landing
npm ci
npm run dev      # http://localhost:5174
npm run build    # static site in build/
npm run check
```

## How it is built

- **It renders the product, not screenshots.** `$lib` is `web/src/lib`: the company office canvas,
  `WorkBoard`, `OutcomeFolio`, `HoldApprove`, the wordmark and the design tokens are the same
  files the owner workspace ships. `$site` is this page's own code (`src/site`).
- **The night shift** (`src/site/NightShift.svelte`) pins the company floor and lets scroll drive
  the camera between places on it. The office runs with `explorable={false}`, so it never captures
  the wheel or the keyboard, and `setCamera()` aims it.
- **The hero reads the visitor's clock** and the campus darkens to match. The cursor is a spotlight.
  Switching tabs and coming back greets the visitor.
- **Previewing**: `?hour=23` shows another time of day; `?motion=reduced` shows the static page that
  reduced-motion visitors get (no pinning, no camera, the same content as a normal page).
- **Illustrative data.** Lantern Studio (`web/src/lib/ui/showcase/lanternStudio.ts`) is the README's
  example business. The page says so wherever it shows it.

## Assets and licences

`npm run sync:assets` (run by `dev` and `build`) copies `web/static/vendor` next to the page, including
the upstream MIT licence for the office engine. The copy is git-ignored. The page footer credits Pixel
Agents (MIT) and JIK-A-4's MetroCity pack (CC0). Provenance of the furniture, floor and wall sprites
upstream is not documented, so confirm it before using them as brand art beyond this page.

## Not here

- No analytics and no cookies.
- Hosted Restless (Cloud) is in private beta and has its own page in the Cloud repository.
