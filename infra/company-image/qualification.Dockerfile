# Test-only overlay for the public Core startup patch. NOT a release artifact:
# build-baked binaries/identity still belong to CORE_BASE. Ship through the full
# Dockerfile/release pipeline after qualification, never update Cloud's lock here.
ARG CORE_BASE
FROM ${CORE_BASE}
LABEL io.restless.experimental="browser-profile-guard"
COPY entrypoint.sh /usr/local/bin/company-init
COPY recover-browser-profile.py /usr/local/lib/restless/recover-browser-profile.py
RUN command -v flock && chmod 0555 /usr/local/bin/company-init \
    && chmod 0444 /usr/local/lib/restless/recover-browser-profile.py
