# Restless public site

The public product site and experiment findings journal. This is a separate Astro package inside the
Restless monorepo; it does not share runtime code or visual identity with the owner cockpit.

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

The interface stays Astro-native and adds no component runtime. Its notched navigation geometry takes
direction from Cult UI's Cutout component, the responsibility line adapts Magic UI's Animated Beam to
a solid single-colour path, and the restrained headline entrance takes direction from Motion
Primitives' Text Effect. These are small, native implementations inside the existing design system,
not imported visual identities. Cult UI and Magic UI are MIT licensed.

## Coolify

Deploy with the repository's private GitHub App integration:

- base directory: `/site`
- build pack: `dockerfile`
- exposed port: `80`
- health path: `/`

The Docker image builds and verifies the site, then serves the static output with Nginx.
