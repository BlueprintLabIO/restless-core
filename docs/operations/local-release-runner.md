# Local Core release runner

## Shared qualification path (2 October 2026)

The pinned Dagger `0.21.10` module in `.dagger/` runs the same source qualification
locally and in `Immutable Core release`: Rust workspace checking, cockpit
checking/building, native Documents checking/building, the pinned native Sheets
engine, issuer package imports and workflow lint. The publishing job exports
the cockpit from that same Dagger graph instead of installing and building it
again. Dagger's persistent engine holds the compiler and npm caches.

```sh
scripts/release/install-dagger.sh
export DAGGER_NO_NAG=1 NODE_OPTIONS=--no-network-family-autoselection
"$HOME/.local/bin/dagger" --progress=plain call qualify --source=.
"$HOME/.local/bin/dagger" --progress=plain call cockpit --source=. export --path=web/build
"$HOME/.local/bin/dagger" --progress=plain call issuer --source=. export --path=/tmp/core-issuer-output
```

The issuer export includes its Core-owned package metadata, declarations and
three implementation files, without `node_modules` or an entire identity image.
Its actual package exports are imported during qualification. This is packaging
evidence; signed issuer publication and Cloud consumption are later migration
work.

`verify-account-plane --source=. --revision="$(git rev-parse HEAD)"` builds the
canonical account-plane Dockerfile, executes its supported help command and
loads the bundled Sheets engine. It does not connect to an owner appliance or
prove company readiness. Account-image inputs are filtered before upload and
again when constructing the Docker context; keep both lists aligned with the
canonical Dockerfile's actual `COPY` inputs and the cockpit source.

The replacement pilot is `restless-core-dagger-drive`, installed at
`~/.local/share/restless-runners/core-dagger` and supervised by
`restless-core-dagger-runner.service`. It uses Node 24 on PATH, the required
`NODE_OPTIONS`, a separate Docker configuration directory and mode 0600 runner
credentials. The service has `UMask=0022` so checked-out public source can be read
by non-root checking containers. Credentials retain their explicit 0600 mode.
This WSL pilot still depends on the workstation; the independent Linux builder
in the target architecture remains migration work.

Dispatch just the shared checks with:

```sh
gh workflow run immutable-core-release.yml --repo BlueprintLabIO/restless-core --ref dev -f publish=false
```

Normal immutable publication uses `publish=true`, which remains the default.
Its image construction, multi-architecture attestations, signatures and v1
bundle assembly still use the existing publisher below. Qualification and UI
construction have moved first; do not describe this as a fully migrated release
or promote an unsigned local check image.

### Observed migration evidence

- Core `78dfd5ce` passed [Actions run 36974008003](https://github.com/BlueprintLabIO/restless-core/actions/runs/36974008003)
  on `restless-core-dagger-drive`. The complete qualification job took 19 seconds
  with warm caches. Its output was `Core qualification passed: Rust workspace,
  cockpit check/build, native Documents check/build, pinned native Sheets engine,
  issuer artifact imports, workflow lint`. Publication was deliberately skipped
  with `publish=false`.
- Local qualification of the updated `76df714e` base passed, including its new
  Docs/Sheets navigation checks. An earlier cold qualification of `0f27a7c5` took
  228 seconds before the issuer/lint checks were added.
- The account-image check on `0f27a7c5` executed the image's supported help
  command, found the bundled cockpit and imported the actual Odoo Sheets engine.
  Before pre-call filtering, an unrelated documentation edit took 170 seconds
  to reconstruct the graph even though compilation was cached. With filtering,
  the first call took 221 seconds, an exact repeat took 2 seconds and an unrelated
  documentation edit took 3 seconds. These are image-check timings, not full
  release or company-readiness timings.
- The offline `restless-core-local` registration (id 21) was removed after the
  replacement runner passed that workflow. Only the replacement Core runner
  remained registered and online.

## Existing immutable publisher

`Immutable Core release` and `Identity image` are manually dispatched GitHub Actions workflows. Their jobs run on a repository-scoped Linux x64 runner labelled `restless-core` on the owner machine. They still publish signed immutable images and the signed Core release bundle to GHCR with the existing GitHub OIDC workflow identity. The bundle format and Cloud trust policy stay the same.

The pilot runner is installed at `~/.local/share/restless-runners/core` and runs as the enabled user service `restless-core-actions-runner.service`. Docker, Docker Compose, and Docker Buildx are installed; the Compose and Buildx CLI plugins are in `~/.docker/cli-plugins`. The service uses a separate `DOCKER_CONFIG` directory under the runner installation so its Buildx builder cannot collide with Cloud's. Check `systemctl --user status restless-core-actions-runner.service`, `docker buildx version`, and repository Settings → Actions → Runners before dispatching. Keep the host online throughout the release. The service has linger enabled to survive logout.

Dispatch the immutable release from `dev` so its identity matches Cloud's trusted Core workflow reference. Record the exact GHCR bundle digest for Cloud's `core_release` input. Actions artifacts now expire after one day; GHCR holds the immutable published bundle. Monitor local Docker and disk usage before starting a build, and review images before cleanup. Never prune company volumes. The runner can use the local Docker daemon, so restrict repository write access and review workflow changes before manual dispatch.

The release builder has a fixed name and persistent cache volume. It is limited
to 40 GB RAM without swap, gets a lower CPU share than normal desktop work,
and runs at most three BuildKit steps together. Its cache keeps compiled Cargo
dependencies across source revisions and targets 45 GB maximum use with 25 GB
free disk space. Do not prune its volume as routine cleanup. A build can still
take hours on emulated ARM64; these bounds aim to keep the workstation usable
and make the next release faster.

Each image is signed as soon as it is published. If the runner disconnects or
the host restarts, rerun the failed workflow attempt at its **same revision**:
the workflow verifies
any already signed images and builds only the missing ones. An existing tag
without a valid workflow signature, source label, both architectures and
attestations stops the run instead of being overwritten. A completed signed
bundle can be admitted by Cloud even if a later artifact-upload step fails.
