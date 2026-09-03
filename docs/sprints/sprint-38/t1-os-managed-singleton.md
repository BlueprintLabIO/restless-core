# S38-T1 — Install one OS-managed singleton

**Layer:** Machine host and owner plane

**Serves:** Restless must be available after login, reboot and daemon failure without a terminal-kept
process.

## Work

- Add install, inspect, start, stop and uninstall operations for a user-level macOS `launchd` service.
- Add a stable singleton lock and identity bound to one state root; refuse a second writer before
  migration or port binding.
- Expose readiness and degraded/crash-loop states through `restless status` and the owner entry.
- Keep secrets out of the service definition and load them through the existing host custody path.
- Generate a Linux `systemd --user` unit and verify its contract without claiming a live Linux run.

## Acceptance

A fresh install, login, reboot simulation and forced daemon kill converge on one ready singleton within
the frozen bound. Duplicate start fails safely. Uninstall removes only the exact service and owned
sockets while preserving company data.

## Makes deletable

Background shell sessions, hand-maintained startup commands and process-name-based singleton guesses.
