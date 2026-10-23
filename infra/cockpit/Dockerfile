# The owner cockpit on its own (S104): the built SPA and the script that installs it into the folder
# the plane serves it from. Shipped beside the account plane, so a release that changes only the
# screens replaces this small image and nothing else; the plane picks the new files up per request.
# The base is the pinned Ubuntu the company image is built on, used only for its shell and coreutils:
# Node is not needed here, and the Node image's OpenSSL and bundled npm failed the release scan.
FROM ubuntu:24.04@sha256:a853f94d226358a79c740cfc7bce0c289748f3fe3488d921d038ccd752c61b60

# The base's own security updates, so a fix published since the pin is in the image.
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get upgrade --yes --no-install-recommends     && rm -rf /var/lib/apt/lists/*

COPY web/build /cockpit
COPY infra/cockpit/install.sh /usr/local/bin/restless-cockpit-install
RUN chmod 0755 /usr/local/bin/restless-cockpit-install

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/restless-cockpit-install", "check"]
ENTRYPOINT ["/usr/local/bin/restless-cockpit-install"]
