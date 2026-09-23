# C56-T1 — Issuer library and self-hosted host

**Layer:** issuer (`services/identity`)

**Friction served:** The self-hosted issuer and Cloud's `fleet-web` + `fleet-api` implement the same
contract twice, so every change has to be made twice.

**Change:**
- `services/self-hosted-identity` becomes `services/identity`.
- `src/issuer.mjs` (`createIssuer`) is the shared library. It takes two ports: **Placement**
  (`resolve(companyId)` → owner, plane, company and cell IDs, Core origin and host, readiness) and
  **Mail**.
- `src/main.mjs` is the self-hosted host, with `placement-static.mjs` and `mail-smtp.mjs`.
- Secure deployments use `__Host-` cookies.

**Makes deletable:** `src/server.mjs` (deleted). In Cloud: the duplicate issuer (C56-T7).
