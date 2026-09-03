# Published service contract v1

This directory is the released compatibility corpus for `published-service.v1`.
Core owns the contract. Provider companions consume these fixtures and must not
invent a second service manifest, invitation, receipt, or cleanup vocabulary.

The two allowed profiles are an HTTPS endpoint with a WebSocket path and a
Godot ENet-compatible UDP endpoint. Both name exactly one internal port and an
immutable OCI image digest. The contract contains no command, mount, arbitrary
port set, environment map, tunnel, provider credential, or Runtime address.

The two `publish-request-*.json` files are complete, exact provider inputs.
`provider-ready.json` and `provider-cleanup.json` demonstrate receipts Cloud 14
must return to Core; the ready receipt identifies the publication-scoped access
key without exposing its material. The `.invalid-*.json` files must remain
rejected before a provider call.
