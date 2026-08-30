# S26-T4 — Execute declarative gates once

Verification is a Runtime service over versioned definitions, not prose independently re-enacted by
producer, verifier and lead.

**Observed friction:** EXP-15 repeatedly ran full suites in several actor contexts. Concurrent native
commands collided, output attribution blurred, and a timeout or successful exit could be mistaken for
product evidence without checking engine errors or leaked processes.

**Layer:** Runtime + Evaluation.

**Deletion target:** prompt-authored gate command sequences, unkeyed reruns and pass claims inferred
from timeout/exit alone.

## Scope

- Define gate command, inputs, environment profile, resource needs, expected receipts and failure
  policy in versioned code/configuration.
- Key reusable results by candidate tree, gate-definition digest and toolchain/environment fingerprint.
- Run the ladder: focused failing gate, fresh blind scenario, then cumulative suite only after success.
- Isolate every invocation through T2/T3 and capture bounded stdout/stderr, exit, engine/runtime errors,
  produced receipts and leaked descendants.
- Coalesce identical in-flight requests; cache only conclusive results.

## Acceptance

- Three simultaneous requests for the same exact gate cause one execution and three linked receipts.
- Changing source, gate bytes or toolchain fingerprint forces a run.
- A zero exit containing a configured engine error, or leaving a child process, fails.
- Timeout terminates and classifies the process as infrastructure evidence; it never proves pass/fail.

