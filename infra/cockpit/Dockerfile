# The owner cockpit on its own (S104): the built SPA and the script that installs it into the folder
# the plane serves it from. Shipped beside the account plane, so a release that changes only the
# screens replaces this small image and nothing else; the plane picks the new files up per request.
# The base is the pipeline's pinned Node image (publish.ts NODE_IMAGE), used here only for its shell.
FROM node:24.18.1-alpine3.23@sha256:c2cc26d8f991c2db236ad51a61efee843c482372d6d22570787309d511694110

COPY web/build /cockpit
COPY infra/cockpit/install.sh /usr/local/bin/restless-cockpit-install
RUN chmod 0755 /usr/local/bin/restless-cockpit-install

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/restless-cockpit-install", "check"]
ENTRYPOINT ["/usr/local/bin/restless-cockpit-install"]
