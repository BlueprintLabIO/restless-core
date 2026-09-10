# S42-T2 — Add bounded application lifecycle

**Layer:** Runtime + machine host  
**Serves:** Persistent company software needs safe evolution without making Restless a generic package manager.

## Work

- Express install/update/remove as exact release-owned or company-authored plans with declared source,
  digest/signature, platform, files, state roots, processes and readiness probe.
- Snapshot or checkpoint declared persistent state before material change and retain one known-good
  rollback target where technically possible.
- Separate disable, remove program while retaining data and explicit data purge.
- Re-observe version/readiness after Runtime image replacement and reconcile missing or changed builds.
- Bound network download, installer execution, retries, temporary paths and cleanup ownership.

## Acceptance

The lifecycle corpus converges on one exact build or one actionable failed state. Rollback preserves
declared data, purge requires explicit exact scope, and arbitrary registry/package/path input cannot
reach execution.

## Makes deletable

Mutable `latest` installs, package-manager folklore and broad app-data cleanup.
