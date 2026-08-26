# Restless public site

The public landing page and experiment findings journal. This is a separate Astro package inside the
Restless monorepo; it does not share runtime code or visual identity with the owner cockpit.

## Local verification

```sh
npm install
npm run verify
npm run dev
```

The site is static. Findings live in `src/content/findings/` and are validated at build time. The
site-specific quality gate rejects the visual and prose defaults deliberately excluded from this
identity.

## Coolify

Deploy with the repository's private GitHub App integration:

- base directory: `/site`
- build pack: `dockerfile`
- exposed port: `80`
- health path: `/`

The Docker image builds and verifies the site, then serves the static output with Nginx.
