# Core delivery

GitHub Actions schedules the pinned Dagger module; the module owns qualification,
image construction, exact-image checks, scanning and release sealing. The existing
`immutable-core-release.yml` identity on `dev` remains the trusted publisher.

Source qualification is read-only:

```sh
dagger call qualify --source=.
dagger call verify-native-documents --source=. --revision="$(git rev-parse HEAD)"
```

Dispatch `Immutable Core release` on `dev` with `publish=true` and
`platform=linux/amd64` for the alpha. ARM uses the separate native ARM runner;
do not dispatch it until that runner exists. Each v2 release manifest declares
only its qualified platform. Legacy v1 manifests and bundles remain admissible.

Publication builds a reusable Runtime tool base, then the account plane, Runtime
and native Documents images. It exercises images imported from their published
digests and records provenance, SPDX SBOMs and a scan refreshed for the supplied
UTC day. High/Critical findings fail qualification and print the package and fix.
Signing uses standard OCI attestations and the existing signed bundle/handoff
contract. Job credentials are mounted secrets or temporary registry state, and
the workflow removes its registry configuration on completion or failure.

Images and bundles use `<revision>-<architecture>` discovery tags; Runtime tools
use an input hash. Consumers select immutable digests after admission. A retry
inspects an existing image, requalifies it and finishes missing evidence. A bundle
with different composition is rejected. Retry the same source after a transient
failure; a dependency fix legitimately creates a new revision.

The native Documents artifact check uses an isolated JWKS server and an unavailable
store fixture: it proves the non-root image serves liveness and refuses readiness.
It does not prove PostgreSQL integration. Cloud staging probes cover the complete
company path before promotion. Source qualification and a registry upload alone
do not establish a deployable release; complete signed publication and admission
must be observed.

## Independent product libraries

`Core product artifacts` runs on relevant pushes to `dev`. It delivers the UI
kit, Office and issuer as small OCI packages in the existing
`ghcr.io/blueprintlabio/restless-core-release` repository. This path builds no
Rust or Runtime image. The whole cockpit's asset delivery is a separate migration.

The publisher first computes the actual package inputs, including resolved
Office imports, dependency locks and build/check recipes. A signed
`<kind>-inputs-<input SHA>-<UTC scan day>` marker can reuse an exact qualified
payload. Registry observations are fresh; authentication/network failures are
not treated as missing artifacts. A marker is created only after qualification
and sealing finish, so an interrupted earlier release cannot suppress a build.
Existing input markers and source discovery tags are never overwritten.

Reuse verifies the exact dev library workflow identity, source, input hash,
archive and evidence. It retains the artifact's original Core revision; the
current integration commit may include unrelated changes. A new scan day or
changed source/recipe inputs requires qualification. The standard signature and
receipt remain independently verifiable.

To force package qualification without consulting published input markers:

```sh
dagger call library --source=. --kind=ui --revision="$(git rev-parse HEAD)" \
  --epoch="$(git show -s --format=%ct HEAD)" --scan-period="$(date -u +%F)" \
  export --path=/tmp/core-ui-qualification
dagger call verify-library-inputs --source=. stdout
```

The canonical input-boundary checks modify owned snapshots of actual sources.
They cover UI-only, imported Office helper, issuer, shared dependency and recipe
changes, plus removed files, unrelated changes and symlink rejection. No company,
model provider or live appliance participates.

Cloud imports these signed OCI packages with `scripts/fetch-core-library.mjs`,
tracks their exact archives and qualification evidence, and verifies admission
before release. Fleet Web consumes the issuer archive directly. Git release
tags and issuer extraction from the account-plane image are retired.
