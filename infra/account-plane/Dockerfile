# Restless account plane — the per-owner Authority Plane (CELL_ARCHITECTURE.md §2).
#
# Build from the REPO ROOT:
#   docker build -f infra/account-plane/Dockerfile \
#     --build-arg SOURCE_REVISION="$(git rev-parse HEAD)" \
#     -t restless-account-plane:<version> .
#
# This is the tier that holds the owner's provider credentials, executes
# effects, enforces budgets and serves the owner cockpit. It performs no company
# work: company Runtimes are separate containers from the company image.
#
# It is deliberately NOT the company image. That image is a persistent company
# computer with a desktop, browser and toolchain; this one runs a single
# credential-holding service and should stay as small as its job.

FROM --platform=$BUILDPLATFORM rust:1-bookworm@sha256:59037199c44290f2befcdd58dcc540164763fc296950255aaefeef096a1866b0 AS build
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
COPY tools/codex-runner/restless-codex-runner.mjs tools/codex-runner/restless-ch-read.mjs tools/codex-runner/gpt-6-sol-0.156.1-model-catalog.json ./tools/codex-runner/
COPY tools/custom-harness/manage.mjs tools/custom-harness/presets.json tools/custom-harness/hermes.py tools/custom-harness/run.mjs tools/custom-harness/legacy-models.mjs tools/custom-harness/openclaw.mjs tools/custom-harness/setup.mjs ./tools/custom-harness/
COPY tools/harness-auth/auth.mjs ./tools/harness-auth/auth.mjs
COPY tools/schedule-test-proxy.py ./tools/schedule-test-proxy.py
# restlessd include_str!s this at compile time (crates/restlessd/src/context.rs:21),
# resolving to /src/docs. Copy the exact file rather than the directory: docs/ is
# hundreds of files and none of the rest belongs in a build context.
COPY docs/COMPANY_OPERATING_RULES.md ./docs/COMPANY_OPERATING_RULES.md

# Stamped into the binary so a running plane can report the exact revision it is
# (S27-T4). Without it a build from an exported tree reports "unknown", which is
# an honest answer but not a pinnable one.
ARG SOURCE_REVISION=""
ENV RESTLESS_SOURCE_REVISION=${SOURCE_REVISION}
# Keep compiled dependencies between source revisions on the workstation's
# persistent BuildKit builder. Copy the finished binary out of the cache mount
# so the image layer still contains the exact result of this build.
RUN --mount=type=cache,id=restless-cargo-registry-v1,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=restless-cargo-native-${BUILDARCH}-${TARGETARCH}-v2,target=/src/target,sharing=locked \
    target="$(cat /rust-target)" \
    && cargo build --release --locked --target "$target" -p restlessd \
    && install -Dm755 /src/target/$target/release/restlessd /out/restlessd

FROM node:24-trixie-slim@sha256:8ec5d7557396cfe32d21c3f9c13072355ceab22b584578ca4bb28af31120cffe AS node-runtime
FROM ubuntu:24.04@sha256:a853f94d226358a79c740cfc7bce0c289748f3fe3488d921d038ccd752c61b60
COPY --from=node-runtime /usr/local/ /usr/local/
ENV DEBIAN_FRONTEND=noninteractive
ENV NODE_OPTIONS=--no-network-family-autoselection

# ca-certificates for provider TLS; curl so the image can probe its own health
# without a second tool. Explicitly refresh the base image's PCRE2 library from
# Ubuntu security; installing curl alone does not describe that dependency.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        ca-certificates \
        curl \
        libpcre2-8-0 \
        libstdc++6 \
        tini \
    && rm -rf /var/lib/apt/lists/*

RUN npm install -g npm@11.21.0 \
    && test "$(npm --version)" = 11.21.0 \
    && npm cache clean --force

# The Authority plane starts OMP's credential broker and model gateway whenever
# a configured company has a model route. Keep that host-side dependency here,
# not in the credential-free Runtime. Its profile lives below the plane's
# persistent state mount so OAuth and broker identity survive plane restarts.
ARG OMP_VERSION=18.3.2
ENV BUN_INSTALL=/usr/local/bun \
    HOME=/state/authority-home
RUN npm install -g bun@1.3.14 \
    && bun install -g "@oh-my-pi/pi-coding-agent@${OMP_VERSION}" \
    && ln -sf ${BUN_INSTALL}/bin/omp /usr/local/bin/omp \
    && chmod -R a+rX ${BUN_INSTALL} \
    && npm cache clean --force \
    && omp --version

COPY --from=build /out/restlessd /usr/local/bin/restlessd
COPY services/native-sheets/package.json services/native-sheets/package-lock.json /opt/restless/native-sheets/
RUN npm --prefix /opt/restless/native-sheets ci --omit=dev --no-audit --no-fund && npm cache clean --force
COPY services/native-sheets/src /opt/restless/native-sheets/src
COPY services/native-sheets/NOTICE /opt/restless/native-sheets/NOTICE
ENV RESTLESS_SHEETS_WORKER=/opt/restless/native-sheets/src/worker.mjs

# Apply compatible security fixes that npm 11.21.0 has not bundled yet.
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

# The owner cockpit's built SPA. The plane serves it directly to the browser
# (CELL_ARCHITECTURE.md §5), so a plane image without it can start but cannot
# do its main job. Built outside this Dockerfile and copied in, because the
# cockpit is a separate toolchain; the production image includes only its
# built output and the bounded native workbook engine's runtime dependencies.
COPY web/build /opt/restless/cockpit
ENV RESTLESS_COCKPIT_DIR=/opt/restless/cockpit

# The plane serves the owner cockpit and the CLI endpoint. In network mode it
# verifies an identity assertion; in local mode it refuses a non-loopback bind.
# The mode is configuration, never a default: a plane with incomplete network
# entry configuration refuses to start rather than trusting its network.
EXPOSE 7788

# Release identity, not company state. Fleet reads this to confirm the running
# plane is the release its manifest deployed.
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl -fsS http://127.0.0.1:7788/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/restlessd"]
