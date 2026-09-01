# Account-plane release topology

`compose.yml` is the reviewed, release-owned template that Fleet's deployment provider materialises
for one owner. Its raw bytes are part of the Core release manifest by SHA-256; changing the template
without producing a new manifest must stop deployment.

The topology contains exactly two services. `account-plane` runs the immutable Core image as an
unprivileged user, owns durable plane state, reaches Fleet JWKS/Infisical through its outbound network,
and exposes only port 7788 to the external TLS proxy network. `plane-database` is reachable only on an
internal pairwise network and has no host port or outbound network. Neither service receives a Docker
socket, host bind, privileged mode, or ambient Linux capabilities.

Database URL, database bootstrap password, readiness bearer, and Infisical machine-identity secret
enter as Compose secrets mounted under `/run/secrets`; the account plane never receives the database
bootstrap password, and PostgreSQL never receives the URL, readiness bearer, or Infisical credential.
The Runtime image reference and both Core/Fleet release identities are immutable deployment inputs.

Verify the rendered topology and print its template digest with:

```text
node scripts/verify-account-plane-compose.mjs
```
