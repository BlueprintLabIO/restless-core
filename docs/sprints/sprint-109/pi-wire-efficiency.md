# Model gateway request efficiency

The Runtime's pi-native requests carried execution objects (`tool`, `runner` and
`session`) on each tool declaration. These are not fields in the pinned pi-ai
18.8.6 Tool contract. The relay removes only those fields from active and inactive
tools before forwarding; it preserves declared fields, unknown extensions,
messages and options. The Runtime's original request is still received and
parsed by the relay, so this does not remove all serialization or memory costs.

Two captured synthetic Staff requests were 549,149 and 520,867 bytes; applying
the same projection produced 143,395 and 115,023 bytes, reductions of 73.9%
and 77.9% on the relay-to-broker hop. These are payload sizes, not measurements
of CPU, total memory, provider tokens or whole-task cost. Anthropic converts
only declared tool fields, so no provider-token saving is claimed here.

The loopback relay test verifies the actual forwarded request, unchanged tool
schemas and provider-specific fields, messages and options. The gateway suite
passed 62 tests. `scripts/verify-pi-tool-wire.py` exercises a scheduled worker
against a local mock provider, requires a successful real command output in a
subsequent provider request and a produced attempt, and cleans its test plane.
It uses synthetic credentials and makes no paid provider calls. This local
verification does not establish the full resource gate or a hard AI cap.
