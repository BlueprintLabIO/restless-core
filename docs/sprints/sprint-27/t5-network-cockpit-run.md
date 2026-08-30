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
