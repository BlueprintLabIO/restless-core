# Local Core release runner

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
