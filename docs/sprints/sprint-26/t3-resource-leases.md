# S26-T3 — Lease scarce runtime resources

Ports, displays, temporary directories and process groups become scoped resources with identity,
ownership and cleanup rather than numbers hidden in commands.

**Observed friction:** two verifier modes started on fixed port `24632`. One host failed to bind while
another client attached to the surviving host, producing mixed evidence that looked like a product
failure.

**Layer:** Runtime + Authority Plane.

**Deletion target:** guessed/fixed ports, shared display assumptions, pid-file folklore and ad hoc
process killing.

## Scope

- Allocate a lease before process launch and inject the resolved resource into the command.
- Bind every child process to an Attempt-owned process group and every client to the server lease token.
- Heartbeat or reconcile leases through observed process state; expiry is cleanup, never correctness.
- Reclaim on normal completion, cancellation, crash and scheduler restart.
- Expose current holder, age and cleanup state to diagnostics without exposing unrelated process data.

## Acceptance

- Concurrent native gates receive distinct endpoints and cannot cross-connect.
- Killing a server terminates its process group and releases its port/display/temp directory.
- Restarting the scheduler reconciles live and dead holders without double allocation.
- An unleased hard-coded endpoint is rejected by the governed native-gate path.

