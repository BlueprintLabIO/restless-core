# Immutable Core release

`.github/workflows/core-release.yml` is the Core-owned GHCR release path for the canonical
application. It runs from an explicit `restless-core-release-*` tag (or a manual dispatch once the
workflow is present on the default branch) and builds `web/` once. Ordinary branch pushes cannot
publish packages. The resulting directory, including `core-ui-manifest.json`, is then consumed
unchanged by both the UI carrier and account-plane image builds.

The workflow publishes three `linux/amd64` images:

- `restless-core-ui`, a source-free carrier for the complete static application;
- `restless-core-account-plane`, which serves that same application and implements the owner API;
- `restless-core-company-runtime`, the persistent company computer.

Each image is pushed under the source revision, captured by its registry digest and receives a
GitHub build-provenance attestation. Promotion uses only the digest-qualified references written to
`core-release-manifest.json`; the source-revision tags are discovery aids, not deployment identity.

## Manifest contract

`restless-core-release.v1` binds all three image references to:

- the exact Core source revision;
- the complete UI artifact, payload and route-manifest SHA-256 identities;
- the product and platform-capability contract versions owned by the web source;
- the account-plane API, identity-assertion and durable-schema versions owned by the Rust source.
- the exact bytes and repository path of the reviewed owner-plane Compose template.

The workflow uploads that manifest, its checksum and the full UI artifact manifest as one immutable
evidence bundle. Cloud's v4 `CORE_UI_ARTIFACT_DIGEST`, `CORE_UI_ROUTE_MANIFEST_DIGEST`,
`CORE_PRODUCT_CONTRACT_VERSION` and `CORE_CAPABILITY_CONTRACT_VERSION` values come from this bundle;
they are never transcribed from a package lock or independently rebuilt Cloud code.

The bundle also includes `infra/account-plane/cloud-compose.template.yaml` and
`owner-plane-compose.sha256`. The manifest's `deployment.ownerPlaneCompose.sha256` is the raw,
lowercase SHA-256 value Cloud supplies as `COOLIFY_PLANE_COMPOSE_TEMPLATE_SHA256`; it hashes the
template bytes exactly, including whitespace. The template uses only the provider's reviewed token
set and its validator rejects mutable image lines, build directives, host ports, Docker-socket
mounts, privileged containers, host networking or PID namespaces, and missing state/database
volumes or network-entry variables. The Runtime bootstrap token is an operation-scoped file beside
the Compose project and enters the read-only account plane only as a file-backed Compose secret; it
is never a service environment variable or a template value. Core accepts either an owner-only
source file or, on Linux, a file whose exact path the kernel reports as a read-only mount. A broadly
readable file on a writable filesystem—or merely beneath a read-only parent mount—still fails closed,
and group/world-writable source modes are rejected even for an exact read-only mount.

The generator rejects a dirty tracked checkout, a source revision that differs from `HEAD`, stale UI
bytes, mutable image tags, missing image roles, duplicate image digests and non-positive contract
versions.

## Hosted runtime compatibility gate

Publishing this template establishes an immutable, reviewable deployment input; it does not by
itself assert that the current account-plane binary can run the hosted topology. Promotion must
remain fail-closed until the account plane consumes the external plane database, desired release
identity, runtime-bootstrap secret, and Fleet readiness/activity/deletion contracts. The current
company lifecycle also invokes the local `docker` CLI, while the reviewed hosted template
deliberately grants neither a Docker socket nor privileged host control. A safe remote runtime
driver (or another equally isolated company-runtime substrate) is therefore required before this
template is deployable end to end.

Focused local checks are:

```sh
npm --prefix web run check:artifact
node --test scripts/core-release-contract.test.mjs
docker compose -p restless-plane-contract -f infra/account-plane/cloud-compose.template.yaml config --no-interpolate --quiet
```
