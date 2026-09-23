# C56-T3 — Membership admin API and issuer metadata

**Layer:** issuer

**Friction served:** Membership could only be managed on the accounts origin's own People screen,
outside the company.

**Change:**
- `/api/admin/v1/companies/{company}/` exposes:
  - `members` (GET);
  - `invitations`;
  - `invitations/{id}/cancel`;
  - `members/{id}/role`, `suspend`, `reinstate` and `remove`.
- Credentialed CORS answers only that company's Core origin. POST requires JSON and the exact
  origin.
- Only the owner changes roles or manages administrators. Invitations return a copyable link.
- `/.well-known/restless-issuer` publishes the metadata document.
- `configure.mjs` refuses secure origins that don't share a parent domain.

**Makes deletable:** the issuer's People screen (C56-T6).
