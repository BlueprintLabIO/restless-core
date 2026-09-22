# Native Documents collaboration service

Core owns document identity, membership, comments, reviews and versions. This service synchronizes
Yjs document bodies using a narrowly scoped PostgreSQL credential and Core-issued document tokens.

## Local Linux installation

Local `restlessd` startup and company creation provision this service automatically, separately from
the company desktop. Docker keeps the container running when the desktop is stopped. Startup checks
its database and Core JWKS readiness before recording `documents.status: ready` in startup Doctor.
That status proves service readiness, not the availability of agent document commands.

The default image is `restless-native-documents:local`; if absent, Core builds it from this directory
using the configured Restless source checkout. `RESTLESS_NATIVE_DOCUMENTS_IMAGE` can select an already
installed image. Core refuses to silently build a replacement for a missing explicit image override.

Each cell has a container named `restless-local-docs-<cell UUID>`, bound to a kernel-selected IPv4
loopback port through Linux host networking. Core records that port under
`$RESTLESS_HOME/native-documents/<cell UUID>.json`; the owner websocket proxy uses that record.
Hosted installations retain per-cell service DNS and their existing provisioner.

The container receives only `cells/<company>/native-documents-database.url`, mounted read-only. It
runs as the credential file's owner, with a read-only root filesystem, dropped capabilities, and
bounded memory/CPU/processes. Database passwords are never placed in Docker environment variables.
The container is labelled with its installation root; reconciliation refuses to replace a container
owned by another installation. Restart reconciles stopped containers and image/issuer changes.
`down --destroy` removes the local Docs container and endpoint before dropping a disposable cell.
Ordinary `down` leaves Core collaboration running.

## Verification

Build the image with Docker, then run the ignored `local_service_starts_reuses_recovers_and_cleans_up`
Rust test with Docker access and `RESTLESS_TEST_DATABASE_URL` set to a disposable-capable PostgreSQL
admin URL. It creates an isolated `_test` cell, serves a real Core JWKS, verifies readiness, container
reuse and restart recovery, then removes the container, endpoint, database and roles on completion.
The ordinary `local_documents::tests` also check endpoint binding and container ownership refusal.
