# Sprint 36 results — bounded published services

**Decision:** Core pass / Cloud public-hosting gate open

Core now turns one exact Work artifact into one bounded, authorised publication operation. The local
provider proves real TLS, WebSocket and UDP transport, scoped access, observation, recovery and
verified teardown. It remains loopback-only and `_test`-only by construction. It does not claim a
public endpoint or a completed Godot ENet application handshake.

## Acceptance audit

| Gate | Result | Evidence and limit |
| --- | --- | --- |
| Exact candidate and request | Pass | Mutable OCI references, manifest/image mismatch and invalid profile declarations fail before provider work. Exact retries reuse candidate and publication identity. |
| Authority and accounting | Pass | One exact owner decision and one resource grant are enforced by Authority uniqueness constraints; public and invite-only consequences differ. |
| Runtime isolation | Pass in Core | The local provider is allowed only for `_test`, binds loopback, clears inherited environment, receives no provider root, company mount or daemon working directory, and exposes only its declared port. Cloud must enforce OS resource and network limits. |
| HTTPS/WebSocket profile | Pass locally | A real self-signed TLS listener serves health and an authenticated WebSocket echo on the manifest paths. Revoked and invalid clients fail. |
| Godot/UDP profile | Contract pass locally | A real UDP listener proves readiness, scoped datagrams, build mismatch refusal and port release. Actual Godot ENet multiplayer remains a Cloud 14 external-client gate. |
| Invitations | Pass at Core boundary | Capabilities bind company, publication, candidate, subject and expiry; tamper, cross-scope, expiry, supersession and revocation fail. Public Cloud must authenticate the named subject before exchange. |
| Observations | Pass | Connection/message counts and last activity enter Authority. CPU, peak memory and storage are explicitly `null` when the local provider cannot measure them. |
| Recovery and idempotency | Pass | Concurrent authorize, duplicate dispatch, response ambiguity, provider death, corrupt marker and daemon-manager reconstruction converge on one operation and endpoint. |
| Terminal cleanup | Pass locally | Stop and expiry verify the process and route are absent, the TCP/UDP port can be rebound, invitation material and resource lease are retired, and the exact publication directory is removed. |
| Cloud contract consumption | Pass | `restless-cloud/scripts/published-service-compat-check.mjs` reads the Core-owned v1 corpus directly and rejects lineage drift; Cloud does not vendor a second schema. |
| Public web and two-player dogfood | Cloud profile partially exercised | Cloud 14 proved a real isolated UDP service, concurrent external Godot clients, authoritative delivery, scoped refusal, restart recovery and zero terminal workload residue. Owned wildcard DNS/trusted TLS, the owner product flow, two physically independent client sites, usage reconciliation and automatic expiry cleanup remain release gates. |
| Forbidden abstractions | Pass | No Runtime hostname, generic tunnel, arbitrary port forward, deployment workflow language, Kubernetes dependency or second publication authority was introduced. |

## Verification

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- live Postgres Authority fixture with a real child provider process, transport probes, restart,
  supersession, expiry and cleanup
- `npm run published-service:compat` and `npm run published-service:compat:test` in `restless-cloud`

The local contract fixture is intentionally disposable. Tests leave no provider process or scoped
publication directory. Existing unrelated development containers are outside this sprint and are not
counted as publication residue.

## Cloud 14 addendum — 2 September 2026

The managed-host run found and repaired a persistent-host lifecycle defect, then completed the UDP
scenario through exact lost-response replay, duplicate dispatch, two concurrent native clients,
cross-company/build/expiry/revocation refusal, independent gateway and artifact restarts, a post-restart
delivery and verified destruction. This strengthens the Core result without changing its boundary:
Core Sprint 36 passes, while the Cloud public-product gate remains open until owned DNS/TLS, owner-plane
controls, two physically independent external paths, usage reconciliation and automatic expiry cleanup
are counted. See the Cloud 14 execution report for exact evidence.
