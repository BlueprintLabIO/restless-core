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

# Releases may reuse a separately qualified tool base by exact OCI digest.
# Local builds still use the stage below, keeping one canonical image recipe.
ARG RUNTIME_TOOLS_IMAGE=runtime-tools

# The coordination CLI (T10): the agents' channel to layer 2, built from this
# workspace so the binary in the image always matches the source in the tree.
FROM --platform=$BUILDPLATFORM rust:1-bookworm@sha256:59037199c44290f2befcdd58dcc540164763fc296950255aaefeef096a1866b0 AS cli
# Compile on the builder CPU; only the final image executes target binaries.
# Debian's target GNU compiler/sysroot also serves Rust dependencies that build C.
ARG BUILDARCH
ARG TARGETARCH
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS} \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
RUN set -eu; \
    case "${TARGETARCH}" in \
        amd64) compiler=gcc-x86-64-linux-gnu; sysroot=libc6-dev-amd64-cross; target=x86_64-unknown-linux-gnu ;; \
        arm64) compiler=gcc-aarch64-linux-gnu; sysroot=libc6-dev-arm64-cross; target=aarch64-unknown-linux-gnu ;; \
        *) echo "unsupported Rust image architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    if [ "${BUILDARCH}" = "${TARGETARCH}" ]; then compiler=gcc; sysroot=libc6-dev; fi; \
    apt-get update; \
    apt-get install --yes --no-install-recommends "$compiler" "$sysroot"; \
    rm -rf /var/lib/apt/lists/*; \
    rustup target add "$target"; \
    printf '%s' "$target" > /rust-target

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,id=restless-cargo-registry-v1,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=restless-cargo-native-${BUILDARCH}-${TARGETARCH}-v2,target=/src/target,sharing=locked \
    target="$(cat /rust-target)" \
    && cargo build --release --locked --target "$target" -p restless -p restless-runtime-bridge \
    && install -Dm755 /src/target/$target/release/restless /out/restless \
    && install -Dm755 /src/target/$target/release/restless-runtime-bridge /out/restless-runtime-bridge

FROM node:24-trixie-slim@sha256:8ec5d7557396cfe32d21c3f9c13072355ceab22b584578ca4bb28af31120cffe AS runtime-desktop

ENV DEBIAN_FRONTEND=noninteractive

# Node's pinned image bundles an older npm dependency tree. Keep the package
# manager current independently of the Node runtime used by company projects.
RUN npm install -g npm@11.21.0 \
    && test "$(npm --version)" = 11.21.0 \
    && npm cache clean --force

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        bash \
        ca-certificates \
        chromium \
        curl \
        dbus-x11 \
        dmz-cursor-theme \
        fonts-dejavu-core \
        fonts-inter \
        fonts-liberation \
        fonts-noto-color-emoji \
        g++ \
        git \
        jq \
        libpcre2-8-0 \
        make \
        arc-theme \
        adwaita-icon-theme \
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
		util-linux \
		unzip \
		wmctrl \
        websockify \
        xdotool \
        x11-utils \
        xterm \
		x11-xserver-utils \
    && rm -rf /var/lib/apt/lists/*

# noVNC is a browser application. Debian's package also installs Node 20 and
# its dependency tree, even though this image already has pinned Node 24.
# Ship the same noVNC 1.6 client directly, including its licenses and package
# identity for the SBOM; its desktop URL and websockify root stay unchanged.
ARG NOVNC_VERSION=1.6.0
ARG NOVNC_SHA256=5066103959ef4e9b10f37e5a148627360dd8414e4cf8a7db92bdbd022e728aaa
RUN curl --fail --location --retry 3 \
        "https://codeload.github.com/novnc/noVNC/tar.gz/refs/tags/v${NOVNC_VERSION}" \
        --output /tmp/novnc.tar.gz \
    && printf '%s  %s\n' "$NOVNC_SHA256" /tmp/novnc.tar.gz | sha256sum --check - \
    && mkdir -p /usr/share/novnc \
    && tar -xzf /tmp/novnc.tar.gz --strip-components=1 --directory=/usr/share/novnc \
        "noVNC-${NOVNC_VERSION}/app" "noVNC-${NOVNC_VERSION}/core" "noVNC-${NOVNC_VERSION}/vendor" \
        "noVNC-${NOVNC_VERSION}/vnc.html" "noVNC-${NOVNC_VERSION}/vnc_lite.html" \
        "noVNC-${NOVNC_VERSION}/defaults.json" "noVNC-${NOVNC_VERSION}/mandatory.json" \
        "noVNC-${NOVNC_VERSION}/LICENSE.txt" "noVNC-${NOVNC_VERSION}/AUTHORS" "noVNC-${NOVNC_VERSION}/package.json" \
    && rm /tmp/novnc.tar.gz \
    && test ! -e /usr/bin/node \
    && test -s /usr/share/novnc/vnc.html \
    && test -s /usr/share/novnc/core/rfb.js

# System defaults apply to existing persistent homes that have no personal
# override. Arc is GTK-only in Debian; use Openbox's bundled Breeze theme with
# Restless colors, rather than naming Arc as a nonexistent Openbox theme.
COPY infra/company-image/gtk-settings.ini /etc/gtk-3.0/settings.ini
COPY infra/company-image/gtk-settings.ini /etc/gtk-4.0/settings.ini
RUN test -f /usr/share/themes/Breeze-ob/openbox-3/themerc \
    && sed -i \
        -e 's/<name>Clearlooks<\/name>/<name>Breeze-ob<\/name>/' \
        -e 's/<name>sans<\/name>/<name>Inter<\/name>/g' \
        -e 's/<size>8<\/size>/<size>10<\/size>/g' \
        -e 's/<size>9<\/size>/<size>10<\/size>/g' \
        /etc/xdg/openbox/rc.xml \
    && sed -i \
        -e 's/#475057/#26384d/g' \
        -e 's/#EFF0F1/#e9edf5/g' \
        -e 's/#3498db/#5888bb/g' \
        -e 's/#d0d2d2/#dad7cf/g' \
        -e 's/#31363b/#354252/g' \
        -e 's/#a5a7a9/#818995/g' \
        -e 's/#3daee9/#315f91/g' \
        -e 's/#fcfcfc/#ffffff/g' \
        -e 's/#BBDCEE/#dce8f2/g' \
        -e 's/#bdc3c7/#566577/g' \
        -e 's/#CED4D9/#69798b/g' \
        -e 's/#798085/#9ba7b5/g' \
        /usr/share/themes/Breeze-ob/openbox-3/themerc

# Keep the desktop independently exercisable without downloading agent and
# authoring runtimes; the full company image consumes this exact same stage.
FROM runtime-desktop AS runtime-harnesses

# The ACP agent binary: omp (Oh My Pi), which speaks ACP natively and reports
# per-turn token and dollar usage on the session stream — the fact that moved
# the spend fuse out of the HTTP proxy and up into the daemon.
#
# omp's bundle uses `using` declarations, so it needs bun >= 1.3.14; node 24
# cannot parse it. BUN_INSTALL is a system path so the unprivileged `company`
# user can execute what root installed.
ENV BUN_INSTALL=/usr/local/bun
RUN npm install -g bun@1.3.14 \
    && bun install -g @oh-my-pi/pi-coding-agent@18.3.2 \
    && ln -sf ${BUN_INSTALL}/bin/omp /usr/local/bin/omp \
    && chmod -R a+rX ${BUN_INSTALL} \
    && npm cache clean --force

# First-party Codex session runtime. Restless drives this through the documented
# app-server protocol when a Codex actor backend is selected; OMP remains a
# separately supported ACP backend. Pinning the CLI makes thread, event,
# approval and usage semantics part of the company-image contract rather than
# whatever npm happens to publish on rebuild day.
ARG CODEX_CLI_VERSION=0.155.1
RUN npm install -g "@openai/codex@${CODEX_CLI_VERSION}" \
    && test "$(codex --version)" = "codex-cli ${CODEX_CLI_VERSION}" \
    && npm cache clean --force

# Certified Claude Code ACP transport. The adapter pins its Agent SDK and the
# bundled native Claude Code binary; Runtime sessions still receive only a
# short-lived Restless model capability, never the host Anthropic API key.
ARG CLAUDE_AGENT_ACP_VERSION=0.73.0
ARG CLAUDE_AGENT_ACP_INTEGRITY="sha512-xKnGIntdBbr2dDS2NEsVGdjoLH62EaWjfYlp/U7TYdxUJzERlApe2gliYW3rVFTeWGjG0dUyPszhG9TWhsqGlA=="
ARG CLAUDE_AGENT_SDK_VERSION=0.3.257
ARG CLAUDE_CODE_BUILD_VERSION=2.1.257
RUN archive="$(npm pack "@agentclientprotocol/claude-agent-acp@${CLAUDE_AGENT_ACP_VERSION}" --silent --pack-destination /tmp)" \
    && test "$(node -e 'const {createHash}=require("node:crypto"); const {readFileSync}=require("node:fs"); process.stdout.write("sha512-"+createHash("sha512").update(readFileSync(process.argv[1])).digest("base64"))' "/tmp/${archive}")" = "${CLAUDE_AGENT_ACP_INTEGRITY}" \
    && npm install -g "/tmp/${archive}" \
    && adapter_root="$(npm root -g)/@agentclientprotocol/claude-agent-acp" \
    && test "$(node -p 'require(process.argv[1]).version' "${adapter_root}/package.json")" = "${CLAUDE_AGENT_ACP_VERSION}" \
    && test "$(node -p 'require(process.argv[1]).version' "${adapter_root}/node_modules/@anthropic-ai/claude-agent-sdk/package.json")" = "${CLAUDE_AGENT_SDK_VERSION}" \
    && test "$(node -p 'require(process.argv[1]).version' "${adapter_root}/node_modules/@anthropic-ai/claude-agent-sdk/manifest.json")" = "${CLAUDE_CODE_BUILD_VERSION}" \
    && rm -f "/tmp/${archive}" \
    && npm cache clean --force
ENV RESTLESS_CLAUDE_AGENT_ACP_BUILD="claude-agent-acp-${CLAUDE_AGENT_ACP_VERSION}" \
    RESTLESS_CLAUDE_CODE_BUILD="claude-code-${CLAUDE_CODE_BUILD_VERSION}" \
    RESTLESS_CLAUDE_AGENT_SDK_BUILD="${CLAUDE_AGENT_SDK_VERSION}"

# Named at this boundary so release verification can prove the complete
# harness toolchain without redownloading unrelated large authoring runtimes.
FROM runtime-harnesses AS runtime-tools

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
#  * Install it in the system prefix. Corepack's per-user activation cache is
#    below root's home during the build, not the persistent company user's home.
RUN npm install -g pnpm@10.34.6 \
    && test "$(pnpm --version)" = 10.34.6 \
    && npm cache clean --force

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
    case "${TARGETARCH:-$(dpkg --print-architecture)}" in \
        amd64) godot_platform=linux.x86_64; godot_binary="Godot_v${GODOT_VERSION}-stable_linux.x86_64"; godot_sha256=cadd3204e728a35d3f13adb7fd0d7902636b79f6b95c40c265eb73b6c35329e4 ;; \
        arm64) godot_platform=linux.arm64; godot_binary="Godot_v${GODOT_VERSION}-stable_linux.arm64"; godot_sha256=5dd0d86405cf7e8adf79fb6377b38ba682a2846cb378ffe5364f38c01ad29b9d ;; \
        *) echo "unsupported Godot image architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    curl --fail --location --show-error --progress-bar --http1.1 --connect-timeout 30 --speed-limit 1024 --speed-time 60 --retry 5 --retry-all-errors --retry-delay 2 "https://github.com/godotengine/godot-builds/releases/download/${GODOT_VERSION}-stable/${godot_binary}.zip" --output /tmp/godot.zip; \
    echo "${godot_sha256}  /tmp/godot.zip" | sha256sum --check --status; \
    unzip -q /tmp/godot.zip -d /tmp/godot; \
    install -m 0755 "/tmp/godot/${godot_binary}" /usr/local/bin/godot; \
    curl --fail --location --show-error --progress-bar --http1.1 --connect-timeout 30 --speed-limit 1024 --speed-time 60 --retry 5 --retry-all-errors --retry-delay 2 "https://github.com/godotengine/godot-builds/releases/download/${GODOT_VERSION}-stable/Godot_v${GODOT_VERSION}-stable_export_templates.tpz" --output /tmp/godot-templates.tpz; \
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
    && useradd --uid 2001 --gid 2000 --no-create-home --home-dir /tmp/restless-effect effect \
    && install -d -o company -g company -m 0770 /company

RUN npm install -g ws@8.22.0 && npm cache clean --force

# npm 11.21.0 still bundles older compatible releases of these libraries.
# Replace the installed JavaScript dependencies, keeping the patched versions
# explicit until npm includes them in its published bundle.
RUN npm --prefix /tmp/npm-security-fixes install \
        --ignore-scripts --omit=dev --no-audit --no-fund --save-exact \
        brace-expansion@5.0.11 undici@6.28.1 \
    && rm -rf /usr/local/lib/node_modules/npm/node_modules/brace-expansion \
        /usr/local/lib/node_modules/npm/node_modules/undici \
    && cp -a /tmp/npm-security-fixes/node_modules/brace-expansion \
        /tmp/npm-security-fixes/node_modules/undici \
        /usr/local/lib/node_modules/npm/node_modules/ \
    && rm -rf /tmp/npm-security-fixes \
    && npm --version \
    && npm cache clean --force

# Startup code and Core binaries change more often than the installed tools.
# An external tool base must be the exact artifact selected by the release.
FROM ${RUNTIME_TOOLS_IMAGE} AS company-runtime

# Group immutable assets so the Dockerfile frontend does not construct a large
# intermediate Runtime filesystem for every individual file.
COPY infra/company-image/entrypoint.sh \
    infra/company-image/company-volume-init.sh \
    infra/company-image/assert-runtime-privileges.py \
    infra/company-image/wait-for-display.sh \
    infra/company-image/start-chromium.sh \
    infra/company-image/start-desktop-panel.sh \
    infra/company-image/focus-chromium.sh \
    infra/company-image/start-terminal.sh \
    infra/company-image/start-godot.sh \
    infra/company-image/start-runtime-bridge.sh \
    infra/company-image/start-company-supervisor.sh \
    infra/company-image/company-supervisorctl.sh \
    infra/company-image/company-tools.sh \
    infra/company-image/validate-company-supervisor-config.py \
    infra/company-image/refresh-desktop-launchers.sh \
    infra/company-image/git-credential-restless \
    infra/company-image/tint2rc \
    infra/company-image/Xresources \
    infra/company-image/company-browser.desktop \
    infra/company-image/company-terminal.desktop \
    infra/company-image/company-godot.desktop \
    infra/company-image/browser-broker.mjs \
    infra/company-image/release-health.mjs \
    infra/company-image/supervisord.conf \
    infra/company-image/company-supervisord.conf \
    /tmp/restless-runtime-assets/
COPY infra/company-image/skills /opt/restless/skills
COPY tools/scenario/restless-scenario.mjs \
    tools/web-review/restless-web-review.mjs \
    tools/codex-runner/restless-codex-runner.mjs \
    tools/codex-runner/gpt-6-sol-0.156.1-model-catalog.json \
    tools/codex-runner/restless-codex-parity-preflight.mjs \
    tools/codex-runner/restless-codex-task-run.mjs \
    tools/codex-runner/restless-codex-longitudinal-run.mjs \
    /usr/local/lib/restless/
ENV RESTLESS_CODEX_GPT6_SOL_CATALOG=/usr/local/lib/restless/gpt-6-sol-0.156.1-model-catalog.json
RUN set -eu; \
    for mapping in \
        entrypoint.sh:company-init \
        company-volume-init.sh:company-volume-init \
        assert-runtime-privileges.py:assert-runtime-privileges \
        wait-for-display.sh:wait-for-company-display \
        start-chromium.sh:start-company-chromium \
        start-desktop-panel.sh:start-company-desktop-panel \
        focus-chromium.sh:focus-company-chromium \
        start-terminal.sh:start-company-terminal \
        start-godot.sh:start-company-godot \
        start-runtime-bridge.sh:start-runtime-bridge \
        start-company-supervisor.sh:start-company-supervisor \
        company-supervisorctl.sh:company-supervisorctl \
        company-tools.sh:company-tools \
        validate-company-supervisor-config.py:validate-company-supervisor-config \
        refresh-desktop-launchers.sh:refresh-company-desktop-launchers \
        git-credential-restless:git-credential-restless; do \
        install -Dm0555 "/tmp/restless-runtime-assets/${mapping%%:*}" "/usr/local/bin/${mapping#*:}"; \
    done; \
    for mapping in \
        tint2rc:/etc/restless/tint2rc \
        Xresources:/etc/restless/Xresources \
        company-browser.desktop:/usr/share/applications/company-browser.desktop \
        company-terminal.desktop:/usr/share/applications/company-terminal.desktop \
        company-godot.desktop:/usr/share/applications/company-godot.desktop \
        browser-broker.mjs:/usr/local/lib/restless/browser-broker.mjs \
        release-health.mjs:/usr/local/lib/restless/release-health.mjs \
        supervisord.conf:/etc/supervisor/conf.d/restless.conf \
        company-supervisord.conf:/etc/supervisor/conf.d/company.conf; do \
        install -Dm0444 "/tmp/restless-runtime-assets/${mapping%%:*}" "${mapping#*:}"; \
    done; \
    chmod 0555 /usr/local/lib/restless/restless-scenario.mjs \
        /usr/local/lib/restless/restless-web-review.mjs \
        /usr/local/lib/restless/restless-codex-runner.mjs \
        /usr/local/lib/restless/restless-codex-parity-preflight.mjs \
        /usr/local/lib/restless/restless-codex-task-run.mjs \
        /usr/local/lib/restless/restless-codex-longitudinal-run.mjs; \
    chmod 0444 /usr/local/lib/restless/gpt-6-sol-0.156.1-model-catalog.json; \
    chmod -R a=rX /opt/restless/skills; \
    ln -s /usr/local/lib/restless/restless-scenario.mjs /usr/local/bin/restless-scenario; \
    ln -s /usr/local/lib/restless/restless-web-review.mjs /usr/local/bin/restless-web-review; \
    ln -s /usr/local/lib/restless/restless-codex-runner.mjs /usr/local/bin/restless-codex-runner; \
    rm -rf /tmp/restless-runtime-assets

# The coordination CLI, and where it finds the daemon from inside the
# container (TCP: unix sockets do not cross the Docker Desktop file share —
# probed in T10). RESTLESS_COMPANY is set per container at `docker run`.
COPY --from=cli /out/restless /out/restless-runtime-bridge /usr/local/bin/
ENV RESTLESS_COORDINATOR=host.docker.internal:7791

# Release metadata changes with source, not with the pinned toolchain. Keep it
# last so a new revision can reuse the large browser and agent installation layers.
ARG RESTLESS_CORE_VERSION=0.0.0
ARG RESTLESS_SOURCE_REVISION=unknown
ARG RESTLESS_API_CONTRACT_VERSION=1
ARG RESTLESS_ASSERTION_CONTRACT_VERSION=1
ARG RESTLESS_SCHEMA_VERSION=73
ENV RESTLESS_CORE_VERSION=${RESTLESS_CORE_VERSION} \
    RESTLESS_SOURCE_REVISION=${RESTLESS_SOURCE_REVISION} \
    RESTLESS_API_CONTRACT_VERSION=${RESTLESS_API_CONTRACT_VERSION} \
    RESTLESS_ASSERTION_CONTRACT_VERSION=${RESTLESS_ASSERTION_CONTRACT_VERSION} \
    RESTLESS_SCHEMA_VERSION=${RESTLESS_SCHEMA_VERSION}

# PID 1 is the seed + long-lived init. Agents never enter this path.
ENTRYPOINT ["/usr/local/bin/company-init"]
