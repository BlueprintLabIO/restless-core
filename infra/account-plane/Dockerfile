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

FROM rust:1-bookworm@sha256:82150a52ec202c1b14d7817e14516c392bb7f5cfebd88f1ed531cb37ebd39922 AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
# restlessd include_str!s this at compile time (crates/restlessd/src/context.rs:21),
# resolving to /src/docs. Copy the exact file rather than the directory: docs/ is
# hundreds of files and none of the rest belongs in a build context.
COPY docs/COMPANY_OPERATING_RULES.md ./docs/COMPANY_OPERATING_RULES.md

# Stamped into the binary so a running plane can report the exact revision it is
# (S27-T4). Without it a build from an exported tree reports "unknown", which is
# an honest answer but not a pinnable one.
ARG SOURCE_REVISION=""
ENV RESTLESS_SOURCE_REVISION=${SOURCE_REVISION}
RUN cargo build --release -p restlessd

FROM node:24-bookworm-slim@sha256:ba849c60be29959425b8734d57b8b4b7d56f98edd9504c9af091d5281095a71e
ENV DEBIAN_FRONTEND=noninteractive

# ca-certificates for provider TLS; curl so the image can probe its own health
# without a second tool. Nothing else: this tier stays narrow because it is the
# one holding the credentials.
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        ca-certificates \
        curl \
        tini \
    && rm -rf /var/lib/apt/lists/*

# The Authority plane starts OMP's credential broker and model gateway whenever
# a configured company has a model route. Keep that host-side dependency here,
# not in the credential-free Runtime. Its profile lives below the plane's
# persistent state mount so OAuth and broker identity survive plane restarts.
ARG OMP_VERSION=18.0.10
ENV BUN_INSTALL=/usr/local/bun \
    HOME=/state/authority-home
RUN npm install -g bun@1.3.14 \
    && bun install -g "@oh-my-pi/pi-coding-agent@${OMP_VERSION}" \
    && ln -sf ${BUN_INSTALL}/bin/omp /usr/local/bin/omp \
    && chmod -R a+rX ${BUN_INSTALL} \
    && npm cache clean --force \
    && omp --version

COPY --from=build /src/target/release/restlessd /usr/local/bin/restlessd

# The owner cockpit's built SPA. The plane serves it directly to the browser
# (CELL_ARCHITECTURE.md §5), so a plane image without it can start but cannot
# do its main job. Built outside this Dockerfile and copied in, because the
# cockpit is a separate toolchain and baking node into this tier would widen
# the container that holds the credentials.
COPY web/build /opt/restless/cockpit
ENV RESTLESS_COCKPIT_DIR=/opt/restless/cockpit

# `index.html` alone can be a stale or partial build. The post-build manifest
# cryptographically identifies every SPA byte and the canonical route set, and
# is served beside the SPA so Fleet can compare the running UI with its pinned
# Core release.
RUN test -s /opt/restless/cockpit/index.html \
    && test -s /opt/restless/cockpit/core-ui-manifest.json \
    && test -s /opt/restless/cockpit/legal/restless-core/LICENSE \
    && test -s /opt/restless/cockpit/legal/pixel-agents/NOTICE.md \
    && test -s /opt/restless/cockpit/legal/pixel-agents/LICENSE

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
