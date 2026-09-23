# C56-T5 — Issuer metadata probe and members view

**Layer:** restlessd

**Friction served:** The cockpit had no way to know whether an issuer exists, where it is, or what it
supports.

**Change:**
- `GET /api/companies/{company}/members` is open to owners and administrators.
- It returns `mode` (`local` or `network`), the immutable `company_id`, the verified members, and
  the issuer metadata.
- The metadata is fetched with the JWKS client, cached for 60 s, bounded to 16 KiB, and its
  origins are checked.
- A missing issuer is reported as `issuer_unavailable`, never as an empty company.
- The gateway admits administrators to this one route family.

**Makes deletable:** nothing.
