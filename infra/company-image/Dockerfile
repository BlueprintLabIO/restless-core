# Restless company image — the persistent company computer (ARCHITECTURE.md §5).
#
# Salvaged from the legacy infra/sandbox-agent/Dockerfile package list, with the
# two assumptions that made it a per-turn disposable sandbox stripped:
#   - the entrypoint is a long-lived init, NOT an agent (agents are ordinary
#     processes started later via `docker exec`);
#   - HOME lives on the persistent /company volume, not a tmpfs.
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

RUN groupadd --gid 2000 company \
    && useradd --uid 2000 --gid 2000 --no-create-home --home-dir /company/home company

COPY entrypoint.sh /usr/local/bin/company-init
RUN chmod 0555 /usr/local/bin/company-init

# PID 1 is the seed + long-lived init. Agents never enter this path.
ENTRYPOINT ["/usr/local/bin/company-init"]
