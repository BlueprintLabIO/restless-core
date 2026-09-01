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

FROM rust:1-trixie AS build
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

FROM node:24-trixie-slim
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
ENV BUN_INSTALL=/usr/local/bun \
    HOME=/state/authority-home
COPY infra/account-plane/omp /opt/restless/omp
RUN npm install -g bun@1.4.0 \
    && cd /opt/restless/omp \
    && bun install --frozen-lockfile --production --omit=optional \
    && ln -sf /opt/restless/omp/node_modules/.bin/omp /usr/local/bin/omp \
    && chmod -R a+rX /opt/restless/omp \
    && npm cache clean --force \
    && rm -rf /usr/local/lib/node_modules/npm /usr/local/bin/npm /usr/local/bin/npx \
    && omp --version

COPY --from=build /src/target/release/restlessd /usr/local/bin/restlessd

# The owner cockpit's built SPA. The plane serves it directly to the browser
# (CELL_ARCHITECTURE.md §5), so a plane image without it can start but cannot
# do its main job. Built outside this Dockerfile and copied in, because the
# cockpit is a separate toolchain and baking node into this tier would widen
# the container that holds the credentials.
COPY web/build /opt/restless/cockpit
ENV RESTLESS_COCKPIT_DIR=/opt/restless/cockpit

# Hosted planes receive no Docker socket or host-control capability. Run the
# credential-holding process as the base image's unprivileged user; declaring
# /state here gives a fresh named volume the ownership needed for durable plane
# state without a privileged init container.
RUN mkdir -p /state/authority-home \
    && chown -R node:node /state /opt/restless/cockpit
USER node

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
