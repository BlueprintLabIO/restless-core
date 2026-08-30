# S27-T3 · Refuse the adversarial cases, provably

**Layer:** Authority Plane + Evaluation.

**Observed outcome or friction:** This is a security boundary, and the repository's own rule applies
with unusual force: a check that happens to pass is not evidence. Verification that returns "invalid"
for every malformed input looks identical, from the outside, to verification that returns "invalid"
because a library call silently failed. Both make the tests green.

**Work:** An adversarial suite covering, each as a distinct case with its own refusal reason: expired;
not-yet-valid; wrong audience; unknown issuer; unknown key version; unsupported contract version;
wrong plane route; tampered signature; a valid assertion replayed after consumption; and an assertion
scoped to another company (T2's case, exercised here as an attack rather than a unit).

Ten inputs, ten distinct recorded refusals. A single catch-all rejection fails this ticket even when
every input is refused, because it cannot distinguish a working verifier from a broken one.

Per Core's testing convention this is a security boundary and earns adversarial tests. It is not a
licence to test ordinary wiring.

**Evidence:** The suite runs headlessly and prints each input with its distinct observed refusal
reason. Then the inverse check, which is the one that matters: temporarily disable signature
verification and confirm the suite fails — and fails on the signature case specifically, not merely
somewhere. A suite that still passes with verification disabled is testing nothing, and finding that
out later would be finding it out in production.

**Deletion target:** Trust that verification works because it compiles and the happy path returns 200.
