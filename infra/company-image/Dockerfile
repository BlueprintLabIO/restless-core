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
        novnc \
        openbox \
        python3 \
        ripgrep \
        socat \
        supervisor \
        tigervnc-standalone-server \
        tini \
        websockify \
    && rm -rf /var/lib/apt/lists/*

# The ACP agent binary: omp (Oh My Pi), which speaks ACP natively and reports
# per-turn token and dollar usage on the session stream — the fact that moved
# the spend fuse out of the HTTP proxy and up into the daemon.
#
# omp's bundle uses `using` declarations, so it needs bun >= 1.3.14; node 24
# cannot parse it. BUN_INSTALL is a system path so the unprivileged `company`
# user can execute what root installed.
ENV BUN_INSTALL=/usr/local/bun
RUN npm install -g bun@1.3.14 \
    && bun install -g @oh-my-pi/pi-coding-agent@17.2.15 \
    && ln -sf ${BUN_INSTALL}/bin/omp /usr/local/bin/omp \
    && chmod -R a+rX ${BUN_INSTALL} \
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
    && corepack prepare pnpm@10.18.0 --activate \
    && pnpm --version

# Playwright library for browser-driven acceptance (T11). Uses the Debian
# chromium installed above — do NOT download Playwright's own browser build.
# Scripts must pass `executablePath: '/usr/bin/chromium'` to chromium.launch.
RUN PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm install -g playwright@1.62.1 \
    && npm cache clean --force

RUN groupadd --gid 2000 company \
    && useradd --uid 2000 --gid 2000 --no-create-home --home-dir /company/home company

COPY infra/company-image/entrypoint.sh /usr/local/bin/company-init
COPY infra/company-image/wait-for-display.sh /usr/local/bin/wait-for-company-display
COPY infra/company-image/start-chromium.sh /usr/local/bin/start-company-chromium
COPY infra/company-image/browser-broker.mjs /usr/local/lib/restless/browser-broker.mjs
COPY infra/company-image/supervisord.conf /etc/supervisor/conf.d/restless.conf
RUN chmod 0555 /usr/local/bin/company-init /usr/local/bin/wait-for-company-display /usr/local/bin/start-company-chromium \
    && chmod 0444 /usr/local/lib/restless/browser-broker.mjs /etc/supervisor/conf.d/restless.conf

# The coordination CLI, and where it finds the daemon from inside the
# container (TCP: unix sockets do not cross the Docker Desktop file share —
# probed in T10). RESTLESS_COMPANY is set per container at `docker run`.
COPY --from=cli /src/target/release/restless /usr/local/bin/restless
ENV RESTLESS_COORDINATOR=host.docker.internal:7791

# PID 1 is the seed + long-lived init. Agents never enter this path.
ENTRYPOINT ["/usr/local/bin/company-init"]
