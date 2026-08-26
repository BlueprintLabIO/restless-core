# Restless public site

The public product site and experiment findings journal. This is a separate Astro package inside the
Restless monorepo. It does not share runtime code with the owner cockpit, but it deliberately uses the
same Bridge Light typography, semantic colours, matrix mark and surface logic. The public site is the
exterior of the same product, not a second brand.

The main narrative is split across five routes so each question has room to be answered:

- `/product/` explains the owner promise and product boundary.
- `/how-it-works/` shows how responsibility and review move through the company.
- `/research/` separates observed evidence, current theory and open questions.
- `/compare/` gives a sourced market comparison without claiming an unrun benchmark.
- `/findings/` keeps the complete experiment record.

## Local verification

```sh
npm install
npm run verify
npm run dev
```

The site is static. Findings live in `src/content/findings/` and are validated at build time. The
site-specific quality gate rejects the visual and prose defaults deliberately excluded from this
identity.

## Design source notes

The interface stays Astro-native and adds no component runtime. Its notched action geometry takes
direction from Cult UI's [Cutout Card](https://www.cult-ui.com/docs/components/cutout-card), while its
signature liquid-metal object is a lightweight SVG and CSS interpretation of Cult UI's
[Hero Liquid Metal](https://www.cult-ui.com/docs/components/hero-liquid-metal). Product surfaces inherit
the owner cockpit's Bridge Light system. These are small implementations inside one visual identity,
not a library collage.

## Coolify

Deploy with the repository's private GitHub App integration:

- base directory: `/site`
- build pack: `dockerfile`
- exposed port: `80`
- health path: `/`

The Docker image builds and verifies the site, then serves the static output with Nginx.
