# Restless company image — the persistent company computer (ARCHITECTURE.md §5).
#
# Build from the REPO ROOT (the CLI stage needs the workspace):
#   docker build -f infra/company-image/Dockerfile -t restless-company-image:<version> .
#
# Salvaged from the legacy infra/sandbox-agent/Dockerfile package list, with the
# two assumptions that made it a per-turn disposable sandbox stripped:
#   - the entrypoint is a long-lived init, NOT an agent (agents are ordinary
#     processes started later via `docker exec`);
#   - HOME lives on the persistent /company volume, not a tmpfs.

# The coordination CLI (T10): the agents' channel to layer 2, built from this
# workspace so the binary in the image always matches the source in the tree.
FROM rust:1-trixie AS cli
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p restless

FROM node:24-trixie-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        bash \
        ca-certificates \
        chromium \
        curl \
		dbus-x11 \
        fonts-liberation \
        g++ \
        git \
        jq \
        make \
        novnc \
        openbox \
		pcmanfm \
        python3 \
		ripgrep \
		scrot \
		socat \
        supervisor \
		tint2 \
        tigervnc-standalone-server \
        tini \
		unzip \
		wmctrl \
		websockify \
		xdotool \
		xterm \
		x11-xserver-utils \
    && rm -rf /var/lib/apt/lists/*

ARG RESTLESS_CORE_VERSION=0.0.0
ARG RESTLESS_SOURCE_REVISION=unknown
ARG RESTLESS_API_CONTRACT_VERSION=1
ARG RESTLESS_ASSERTION_CONTRACT_VERSION=1
ARG RESTLESS_SCHEMA_VERSION=21
ENV RESTLESS_CORE_VERSION=${RESTLESS_CORE_VERSION} \
    RESTLESS_SOURCE_REVISION=${RESTLESS_SOURCE_REVISION} \
    RESTLESS_API_CONTRACT_VERSION=${RESTLESS_API_CONTRACT_VERSION} \
    RESTLESS_ASSERTION_CONTRACT_VERSION=${RESTLESS_ASSERTION_CONTRACT_VERSION} \
    RESTLESS_SCHEMA_VERSION=${RESTLESS_SCHEMA_VERSION}

# The ACP agent binary: omp (Oh My Pi), which speaks ACP natively and reports
# per-turn token and dollar usage on the session stream — the fact that moved
# the spend fuse out of the HTTP proxy and up into the daemon.
#
# omp's bundle uses `using` declarations, so it needs bun >= 1.3.14; node 24
# cannot parse it. BUN_INSTALL is a system path so the unprivileged `company`
# user can execute what root installed.
ENV BUN_INSTALL=/usr/local/bun
RUN npm install -g bun@1.4.0 \
    && bun install -g @oh-my-pi/pi-coding-agent@18.0.11 \
    && ln -sf ${BUN_INSTALL}/bin/omp /usr/local/bin/omp \
    && chmod -R a+rX ${BUN_INSTALL} \
    && npm cache clean --force

# First-party Codex session runtime. Restless drives this through the documented
# app-server protocol when a Codex actor backend is selected; OMP remains a
# separately supported ACP backend. Pinning the CLI makes thread, event,
# approval and usage semantics part of the company-image contract rather than
# whatever npm happens to publish on rebuild day.
ARG CODEX_CLI_VERSION=0.152.0
RUN npm install -g "@openai/codex@${CODEX_CLI_VERSION}" \
    && test "$(codex --version)" = "codex-cli ${CODEX_CLI_VERSION}" \
    && npm cache clean --force

# S04-T2. pnpm, pinned, installed system-wide.
#
# The Study repo's `.husky/pre-commit` runs `pnpm lint-staged`, so without pnpm
# the company **cannot commit at all** — discovered by trying, not by reading.
# Two details, both learned the hard way:
#
#  * The version is pinned to the 10.x line because `pnpm-lock.yaml` is
#    `lockfileVersion: '9.0'` and the repo declares `pnpm.onlyBuiltDependencies`
#    in package.json. pnpm 11 stopped reading that field and fails the commit
#    with ERR_PNPM_IGNORED_BUILDS. `corepack prepare pnpm@latest` is therefore
#    exactly the wrong instinct — it installs a version the repo predates.
#  * It is prepared here, as root, into a system path. Corepack's per-user cache
#    lives under the company user's home on the *volume*, so activating as root
#    at runtime leaves the company user still on whatever it had.
ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0
RUN corepack enable pnpm \
    && corepack prepare pnpm@10.34.5 --activate \
    && pnpm --version

# An ordinary open-source CLI, not a Restless provider adapter. Authority
# injects RESEND_API_KEY only into the exact governed child process.
RUN npm install -g resend-cli@2.12.0 mcp-remote@0.8.1 \
    && resend --version \
    && command -v mcp-remote \
    && npm cache clean --force

# Playwright library for browser-driven acceptance (T11). Uses the Debian
# chromium installed above — do NOT download Playwright's own browser build.
# Scripts must pass `executablePath: '/usr/bin/chromium'` to chromium.launch.
RUN PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm install -g playwright@1.62.1 \
    && npm cache clean --force

# Scenario-native game delivery lane (Sprint 19). Godot is a self-contained
# Runtime tool, not a Restless game abstraction. The official Linux downloads
# publish arm64 and x86_64 builds; pin both source artifacts so an image rebuild
# cannot silently take a different engine binary. The first live need is a
# Windows x86_64 export, so retain exactly its debug/release templates rather
# than permanently shipping every platform's multi-gigabyte template set.
# Templates live in the image and are linked into each persistent company home
# at boot.
ARG TARGETARCH
ARG GODOT_VERSION=4.7.2
RUN set -eux; \
    case "$TARGETARCH" in \
        amd64) godot_platform=linux.x86_64; godot_binary="Godot_v${GODOT_VERSION}-stable_linux.x86_64"; godot_sha256=cadd3204e728a35d3f13adb7fd0d7902636b79f6b95c40c265eb73b6c35329e4 ;; \
        arm64) godot_platform=linux.arm64; godot_binary="Godot_v${GODOT_VERSION}-stable_linux.arm64"; godot_sha256=5dd0d86405cf7e8adf79fb6377b38ba682a2846cb378ffe5364f38c01ad29b9d ;; \
        *) echo "unsupported Godot image architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    curl --fail --location --silent --show-error --http1.1 --retry 5 --retry-all-errors --retry-delay 2 "https://downloads.godotengine.org/?flavor=stable&platform=${godot_platform}&slug=${godot_platform}.zip&version=${GODOT_VERSION}" --output /tmp/godot.zip; \
    echo "${godot_sha256}  /tmp/godot.zip" | sha256sum --check --status; \
    unzip -q /tmp/godot.zip -d /tmp/godot; \
    install -m 0755 "/tmp/godot/${godot_binary}" /usr/local/bin/godot; \
    curl --fail --location --silent --show-error --http1.1 --retry 5 --retry-all-errors --retry-delay 2 "https://downloads.godotengine.org/?flavor=stable&platform=templates&slug=export_templates.tpz&version=${GODOT_VERSION}" --output /tmp/godot-templates.tpz; \
    echo "f298490b8d44d934be425a5a65a51bf15f422428b229a06a6e11d9ffea248011  /tmp/godot-templates.tpz" | sha256sum --check --status; \
    mkdir -p "/opt/restless/godot/export_templates/${GODOT_VERSION}.stable"; \
    unzip -j -q /tmp/godot-templates.tpz \
        templates/windows_debug_x86_64.exe \
        templates/windows_release_x86_64.exe \
        templates/version.txt \
        -d "/opt/restless/godot/export_templates/${GODOT_VERSION}.stable"; \
    chmod -R a=rX /opt/restless/godot; \
    rm -rf /tmp/godot /tmp/godot.zip /tmp/godot-templates.tpz

RUN groupadd --gid 2000 company \
    && useradd --uid 2000 --gid 2000 --no-create-home --home-dir /company/home company \
    && useradd --uid 2001 --gid 2000 --no-create-home --home-dir /tmp/restless-effect effect

COPY infra/company-image/entrypoint.sh /usr/local/bin/company-init
COPY infra/company-image/wait-for-display.sh /usr/local/bin/wait-for-company-display
COPY infra/company-image/start-chromium.sh /usr/local/bin/start-company-chromium
COPY infra/company-image/start-desktop-panel.sh /usr/local/bin/start-company-desktop-panel
COPY infra/company-image/focus-chromium.sh /usr/local/bin/focus-company-chromium
COPY infra/company-image/start-terminal.sh /usr/local/bin/start-company-terminal
COPY infra/company-image/start-godot.sh /usr/local/bin/start-company-godot
COPY infra/company-image/refresh-desktop-launchers.sh /usr/local/bin/refresh-company-desktop-launchers
COPY infra/company-image/git-credential-restless /usr/local/bin/git-credential-restless
COPY infra/company-image/skills /opt/restless/skills
COPY infra/company-image/tint2rc /etc/restless/tint2rc
COPY infra/company-image/company-browser.desktop /usr/share/applications/company-browser.desktop
COPY infra/company-image/company-terminal.desktop /usr/share/applications/company-terminal.desktop
COPY infra/company-image/company-godot.desktop /usr/share/applications/company-godot.desktop
COPY infra/company-image/browser-broker.mjs /usr/local/lib/restless/browser-broker.mjs
COPY infra/company-image/release-health.mjs /usr/local/lib/restless/release-health.mjs
COPY infra/company-image/runtime-agent.mjs /usr/local/lib/restless/runtime-agent.mjs
COPY tools/scenario/restless-scenario.mjs /usr/local/lib/restless/restless-scenario.mjs
COPY tools/web-review/restless-web-review.mjs /usr/local/lib/restless/restless-web-review.mjs
COPY tools/codex-runner/restless-codex-runner.mjs /usr/local/lib/restless/restless-codex-runner.mjs
COPY tools/codex-runner/restless-codex-parity-preflight.mjs /usr/local/lib/restless/restless-codex-parity-preflight.mjs
COPY tools/codex-runner/restless-codex-task-run.mjs /usr/local/lib/restless/restless-codex-task-run.mjs
COPY tools/codex-runner/restless-codex-longitudinal-run.mjs /usr/local/lib/restless/restless-codex-longitudinal-run.mjs
COPY infra/company-image/supervisord.conf /etc/supervisor/conf.d/restless.conf
RUN chmod 0555 /usr/local/bin/company-init /usr/local/bin/wait-for-company-display \
        /usr/local/bin/start-company-chromium /usr/local/bin/start-company-desktop-panel \
        /usr/local/bin/focus-company-chromium /usr/local/bin/start-company-terminal \
        /usr/local/bin/start-company-godot /usr/local/bin/refresh-company-desktop-launchers \
        /usr/local/bin/git-credential-restless \
        /usr/local/lib/restless/restless-scenario.mjs \
        /usr/local/lib/restless/restless-web-review.mjs \
        /usr/local/lib/restless/restless-codex-runner.mjs \
        /usr/local/lib/restless/restless-codex-parity-preflight.mjs \
        /usr/local/lib/restless/restless-codex-task-run.mjs \
        /usr/local/lib/restless/restless-codex-longitudinal-run.mjs \
    && chmod -R a=rX /opt/restless/skills \
    && chmod 0444 /usr/local/lib/restless/browser-broker.mjs \
        /usr/local/lib/restless/release-health.mjs /usr/local/lib/restless/runtime-agent.mjs \
        /etc/supervisor/conf.d/restless.conf \
        /etc/restless/tint2rc /usr/share/applications/company-browser.desktop \
        /usr/share/applications/company-terminal.desktop /usr/share/applications/company-godot.desktop
RUN ln -s /usr/local/lib/restless/restless-scenario.mjs /usr/local/bin/restless-scenario
RUN ln -s /usr/local/lib/restless/restless-web-review.mjs /usr/local/bin/restless-web-review
RUN ln -s /usr/local/lib/restless/restless-codex-runner.mjs /usr/local/bin/restless-codex-runner

# The coordination CLI, and where it finds the daemon from inside the
# container (TCP: unix sockets do not cross the Docker Desktop file share —
# probed in T10). RESTLESS_COMPANY is set per container at `docker run`.
COPY --from=cli /src/target/release/restless /usr/local/bin/restless
ENV RESTLESS_COORDINATOR=host.docker.internal:7791

# PID 1 is the seed + long-lived init. Agents never enter this path.
ENTRYPOINT ["/usr/local/bin/company-init"]
