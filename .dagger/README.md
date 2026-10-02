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
