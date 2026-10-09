# Local Core release runner

## Releasing fast (9 October 2026)

A release with nothing to rebuild takes about 20 minutes on the single runner (qualify 7.5,
publish 9, seal 3). Where the time went on 9 October, and what now keeps it short:

- **Cut once.** Five cancel-and-recut cycles in one afternoon each discarded up to 25 minutes,
  more than every build step combined. Batch the work, cut one release, and when one phase
  fails use "Re-run failed jobs": qualify, publish and seal are separate jobs. Two releases of
  `main` never run together (the workflow's concurrency group), so a second runner would not
  parallelise releases; it would only let the smokes and the UI-artifact job run beside one.
- **Publish's slow builds start beside qualification.** One runner cannot run the publish job
  until qualify finishes, so qualify starts `dagger call prebuild` detached: the release Rust
  binaries and, when the registry has no image from its exact inputs, the runtime tool base. It
  pushes nothing. Measured on 9 October: a cold prebuild took 339 s (tool base 309 s beside Rust
  84 s); repeating it took 43 s for the tool base. The Rust step is not a full cache hit: the
  uploaded source gets fresh timestamps, so cargo recompiles the workspace crates against the
  warm target cache (70-90 s, against 120-276 s cold in publish). Zeroing the timestamps would
  make cargo trust stale outputs in that persistent cache, so it stays this way. A failed or
  unfinished prebuild only means publish builds those parts itself.
- **Often-bumped pins come last in the company image.** Chrome sat under the desktop packages
  and the agent harnesses under pnpm, Playwright and Godot, so a Chrome bump rebuilt the tool
  base in 766 s and the Claude adapter bump in 241 s. The stages are now runtime-desktop, then
  runtime-authoring (pnpm, Resend, Playwright, Godot, accounts), then runtime-harnesses (uv, OMP,
  Codex, Claude), then runtime-tools (Chrome and its SBOM). Measured with `docker build` against
  a warm cache, a simulated Chrome bump rebuilt in 36 s, a harness bump in 161 s, and a change to
  the rarely touched authoring tools in 229 s.
- **The Connections smoke has a fast lane.** It took 6 of qualify's 7.5 minutes (5m16s of
  assertions on one shared plane, including a deliberate 45 s token expiry). It exercises the
  daemon, CLI, company image and host tools, never the cockpit or docs, so when every top-level
  tree entry except `web`, `docs` and `experiment` is byte-identical to a run that passed on this
  runner, it is skipped and the summary says so. Markers live in
  `~/.cache/restless-runner/connections-passed` (newest 40 kept). A cockpit-only release
  therefore skips the smoke; anything touching Rust, the images, scripts or workflows runs it.
- **The engine's cache is bounded.** Unconfigured, Dagger lets its cache use 75% of the disk; on
  9 October it held 400 GB, 71% of it copies of whole checkouts: `seal` (and the lint, cockpit,
  overlay, release-contract, sheets and issuer entry points) declared no ignore patterns, so
  each release stored the runner's 14 GB `.git` and each call from a worktree stored its Cargo
  targets and `node_modules` (17 GB). Every entry point that takes `source` now declares them
  (`target-*` included); `verify-workflow --source=.` went from a 17 GB upload to 0.2 GB. The
  engine's GC is capped at `{"gc": {"maxUsedSpace": "150GB", "reservedSpace": "40GB",
  "minFreeSpace": "15%"}}`, written to `/etc/dagger/engine.json` in the running
  `dagger-engine-v0.21.10` container and to `~/.config/dagger/engine.json`, which the CLI
  mounts when it provisions a new engine version. Restart the engine only while both runners
  are idle (`gh api repos/BlueprintLabIO/restless-core/actions/runners`). Pruning to that
  policy took 11 s and left 162 GB.
- **The cockpit still ships through a Core release.** Cloud serves it from the account-plane
  image (`COPY web/build`), so a UI change needs that image, and with the fast lane, the reused
  tool base and cached Rust its release is the short one.

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

### Runtime construction and tool-base reuse

The canonical company Dockerfile now exposes `runtime-tools` separately from
`company-runtime`. The base contains the pinned OS/Node image, agent/browser
tools, Godot export templates and fixed Unix accounts. Startup scripts, skills,
Core CLI/Bridge binaries and release metadata belong to the thin final image.
Local Docker builds still use the in-file base stage. A release can select an
already-admitted `restless-runtime-tools@sha256:...` through
`RUNTIME_TOOLS_IMAGE`; registry publication and admission of that separate base
have not yet been exercised.

```sh
"$HOME/.local/bin/dagger" --progress=plain call verify-runtime-tools --source=.
"$HOME/.local/bin/dagger" --progress=plain call verify-company-runtime --source=. --revision="$(git rev-parse HEAD)"
```

Dagger imports only the Dockerfile and GTK settings for the base. The final
image adds its actual Rust/runtime inputs, excluding Python test/cache output.
Composition metadata is applied after Dockerfile construction, so changing a
release revision does not replay image construction. Asset copies are grouped
in the canonical recipe; their existing paths and permissions are preserved.

The Runtime check executes CLI help, checks Bridge linkage and exact revision,
then exercises the immutable company-supervisor launcher. A small supervisor
fixture runs the production privilege assertion in an actual child: uid/gid
2000, no supplementary groups or capabilities, and `NoNewPrivs`. The fixture
replaces the company supervisor configuration only in the isolated checking
container and stops its exact supervisor process before exit. It never starts
an owner appliance or a hosted company. The previous Python contract check
still expected Supervisord's uid-only drop; it now follows the immutable
launcher introduced by `8c63e3dc`.

Observed Linux AMD64 results on 2 October 2026:

- Base/tool execution: 224 seconds cold, 3 seconds repeated, 2 seconds after an
  unrelated Rust input changed. Node, Codex, OMP, pnpm, Godot, Chromium, WebSocket
  imports, Unix accounts and export templates were exercised.
- The grouped final image first built and passed binary/metadata checks in 861
  seconds. The updated metadata/privilege check first took 1,051 seconds, then
  3 seconds repeated and 9 seconds with a different revision. The latter reran
  the metadata and actual privilege checks without replaying Dockerfile
  construction. This demonstrates that boundary, not a full-release speedup.
- Shared qualification passed in 43 seconds; the focused supervision and
  release-contract checks passed 5/5 and 10/10.

Logs are in the operator's `qualification/core-runtime-tools-20261002`,
`core-runtime-grouped-20261002` and `core-runtime-metadata-20261002` directories.
The first final-image attempt was deliberately interrupted after exposing
expensive per-file Dockerfile translation; its exit status remains recorded as
a failure. Cold construction is still expensive. ARM, separate base publication,
full signed release publication and hosted company behaviour remain unproved.

### Runner pilot

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

Dispatch the immutable release from `main` so its identity matches Cloud's trusted Core workflow reference. Record the exact GHCR bundle digest for Cloud's `core_release` input. Actions artifacts now expire after one day; GHCR holds the immutable published bundle. Monitor local Docker and disk usage before starting a build, and review images before cleanup. Never prune company volumes. The runner can use the local Docker daemon, so restrict repository write access and review workflow changes before manual dispatch.

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
