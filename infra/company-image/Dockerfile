# Restless company image — the persistent company computer (ARCHITECTURE.md §5).
#
# Build from the REPO ROOT (the CLI stage needs the workspace):
#   docker build -f infra/company-image/Dockerfile -t restless-company-image:latest .
#
# Salvaged from the legacy infra/sandbox-agent/Dockerfile package list, with the
# two assumptions that made it a per-turn disposable sandbox stripped:
#   - the entrypoint is a long-lived init, NOT an agent (agents are ordinary
#     processes started later via `docker exec`);
#   - HOME lives on the persistent /company volume, not a tmpfs.

# The coordination CLI (T10): the agents' channel to layer 2, built from this
# workspace so the binary in the image always matches the source in the tree.
FROM rust:1-bookworm AS cli
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p restless

FROM node:24-bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        bash \
        ca-certificates \
        chromium \
        curl \
        fonts-liberation \
        g++ \
        git \
        jq \
        make \
        python3 \
        ripgrep \
        socat \
        tini \
    && rm -rf /var/lib/apt/lists/*

# The ACP agent binary, pinned to the version live-probed in the T3 spike
# (docs/sprints/sprint-01/t03-spike-report.md).
RUN npm install -g @agentclientprotocol/codex-acp@1.1.4 @openai/codex@0.144.5 \
    && npm cache clean --force

# Playwright library for browser-driven acceptance (T11). Uses the Debian
# chromium installed above — do NOT download Playwright's own browser build.
# Scripts must pass `executablePath: '/usr/bin/chromium'` to chromium.launch.
RUN PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm install -g playwright@1.62.1 \
    && npm cache clean --force

RUN groupadd --gid 2000 company \
    && useradd --uid 2000 --gid 2000 --no-create-home --home-dir /company/home company

COPY infra/company-image/entrypoint.sh /usr/local/bin/company-init
RUN chmod 0555 /usr/local/bin/company-init

# The coordination CLI, and where it finds the daemon from inside the
# container (TCP: unix sockets do not cross the Docker Desktop file share —
# probed in T10). RESTLESS_COMPANY is set per container at `docker run`.
COPY --from=cli /src/target/release/restless /usr/local/bin/restless
ENV RESTLESS_COORDINATOR=host.docker.internal:7791

# PID 1 is the seed + long-lived init. Agents never enter this path.
ENTRYPOINT ["/usr/local/bin/company-init"]
