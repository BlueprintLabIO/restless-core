# Restless Core UI artifact carrier.
#
# Build from the repository root after `npm --prefix web run build`:
#   docker build -f infra/core-ui-artifact/Dockerfile \
#     -t restless-core-ui:<version> .
#
# This image deliberately has no runtime, server, source tree or platform
# adapter. Consumers copy or mount the one immutable SPA directory into the
# account/Fleet service that implements the versioned platform contract.

FROM scratch

LABEL org.opencontainers.image.title="Restless Core UI"
LABEL org.opencontainers.image.description="Canonical Restless Core SvelteKit SPA artifact"
LABEL org.opencontainers.image.licenses="Apache-2.0"

COPY web/build/ /opt/restless/core-ui/
