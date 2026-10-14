# UXG-284 — Stop shipping the gallery and office demo routes, with a founder’s email, to every install

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Shell & cross-cutting | medium | security | US-176 |

## Problem

/gallery/* and /office-demo are ordinary routes with no dev guard, so every customer’s cockpit serves fixture pages with fake companies, fake "Sign out" buttons and a founder’s personal name and email address hard-coded into the bundle. Anyone who guesses the path sees them, and the email ships in public JavaScript.

## Evidence

- `web/src/routes/gallery/home/+page.svelte:109` (personal name and email)
- `web/src/routes/gallery/home/+page.svelte:115` (inert Sign out button)
- `web/src/routes/office-demo/+page.svelte:1-5` (demo route)
- `web/src/app.html:32-37` (gallery and office-demo treated as top-level routes)

## Suggested direction

Exclude the gallery and demo routes from production builds (or gate them on dev) and replace personal data with neutral fixtures.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
