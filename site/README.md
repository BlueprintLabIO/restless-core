# Restless public research site

This Astro package is the staged public surface for Restless. It presents the owner promise alongside
a source-first research journal. It is a candidate publication only: building it does not publish it,
create an account, or make an external claim on the company's behalf.

The S20 baseline is the accepted visual direction from `experiment/exp-07-candidate-a` at
`4e454bc2b00c7bc3ee9d97e3d61f8ca256f6d6ea`, adapted into this package without importing its old
static build system. The surface uses one editorial identity: paper, ink, cobalt, lime, a modern
sans-serif and an italic editorial serif. The research record is intentionally legible before it is
persuasive.

The main routes are:

- `/product/` explains the owner promise and its boundary.
- `/how-it-works/` follows one accountable outcome.
- `/research/` separates direction, evidence and unknowns.
- `/research/corpus/` accounts for EXP-01 through EXP-07, including deferrals.
- `/journal/` contains the public research notes and their source locators.
- `/compare/` records dated, source-first category observations.

## Local verification

```sh
npm install
npm run verify
npm run dev
```

The site is static. Journal entries live in `src/content/journal/` and are validated at build time.
The site-specific quality gate rejects visual and prose defaults deliberately excluded from this
identity. Legacy `/findings/` links redirect to `/journal/`.

To package the candidate from a clean install:

```sh
docker build --tag restless-site-candidate site
```

Run `npm run verify` from `site/` first: it validates that every journal locator resolves against the
repository's frozen EXP-01 through EXP-07 source files. The standalone Docker build then repeats the
corpus-shape, design, type and static-build checks before producing an Nginx image containing only
`dist/`.

## Publication boundary

Do not deploy, connect a publishing integration, or describe this site as publicly live without a
separate owner-approved effect. Any publication review must use the built site as the native review
target and retain the exact research limits shown here.
