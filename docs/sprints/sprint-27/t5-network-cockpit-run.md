# S27-T5 · Reach the cockpit over a network, end to end

**Layer:** Full slice + Evaluation.

**Observed outcome or friction:** T1–T4 can each pass while the product does not work. Four green
components are not a human reaching their business over a network, and this repository's rule is that
no component is described as working until it has been run with stated inputs and observed output.
This ticket is what makes the sprint a slice rather than four boundaries.

**Work:** From a clean checkout, one command that builds the release, boots a plane in network mode
and one cell from the published manifest, mints a test assertion with a test issuer, and serves the
cockpit on a non-loopback address. A browser reaches it, signs in with the assertion, and reads the
company — with the cell asleep, then with it awake.

The test issuer stands in for Cloud's Better Auth. It exists to prove Core's verifying half and must
not grow into a Core identity product; Cloud owns issuing.

**Evidence:** A recorded run: the exact command, the non-loopback address served, the cockpit
rendering the company with the cell asleep, and a wake plus one Attempt proceeding afterwards. Then
the negative case in the same run — the same browser without a valid assertion is refused, and refused
with the reason, not merely served nothing.

Clean up in the same turn: stop the plane and cell, remove the `_test` company, and run
`restless-reap --check` before reporting the sprint complete.

**Deletion target:** Component-level confidence as a substitute for a run; the claim that Restless is
reachable, made from four passing test suites.

## Technical run — 30 August 2026

`scripts/restless-network-release-run` now supplies the clean-checkout, one-command run. The
non-interactive verification invocation was:

```text
RESTLESS_NETWORK_TEST_AUTO_WAKE=1 RESTLESS_NETWORK_TEST_KEEP_RUNNING=0 \
RESTLESS_NETWORK_TEST_REGISTRY_PORT=15001 RESTLESS_NETWORK_TEST_POSTGRES_PORT=15433 \
RESTLESS_NETWORK_TEST_PLANE_PORT=17889 RESTLESS_NETWORK_TEST_PORT_OFFSET=7100 \
scripts/restless-network-release-run
```

From clean revision `9fda0f56f505bbebb07e553f2e23f58e74cf72d5`, it built and pushed an
account plane and Runtime to a disposable registry, resolved both back to immutable digests, booted
the network plane with an initially sleeping `_test` company, exchanged a signed test assertion for
a session, and read that company's cockpit. It then started the Runtime by the resolved digest and
observed both the Runtime health revision and the authenticated awake cockpit:

```text
account plane: 127.0.0.1:15001/restless/account-plane@sha256:9c7b3ccaff8a66b5813672f5ad5890ac83f9c3f61ce05d7606b9ab51292a4531
Runtime:       127.0.0.1:15001/restless/company-runtime@sha256:3437e0fbc60dd013f0c2a3c61fdd32b4b628ce549be9f849efc92b2a2f191bc8
sleeping cockpit probe: PASS
awake Runtime health and cockpit probes: PASS
```

The first execution exposed two harness errors rather than weakening production policy: curl will
not replay a `Secure` session cookie over loopback HTTP, and network-mode entry correctly refuses a
state-changing request without an `Origin`. The harness now extracts the test session explicitly and
sends the matching loopback origin. The successful run exited zero and its trap removed the plane,
cell, Postgres, registry, Docker network, company volume and temporary home; exact-name Docker checks
after exit returned no resources.

This closes the machine-observable portion only. The configured listener accepts non-loopback
connections, but this run deliberately probed it through loopback and no in-app browser was available
to observe the rendered sleeping/awake states. The ticket therefore remains `[~]` until the stated
non-loopback browser observation and same-browser negative case are recorded.
