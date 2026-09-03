# S38-T7 — Upgrade, recover and clean exact machine resources

**Layer:** Machine host and operations

**Serves:** A singleton becomes a liability if an update can corrupt its only state or uninstall leaves
hidden processes and credentials.

## Work

- Stage and preflight a pinned release before activation; drain new effects and re-observe durable work.
- Activate one version atomically and retain one last-known-good recovery target where migration allows.
- Stop boot loops after a bounded number of failures and expose one actionable recovery state.
- Make uninstall and explicit purge separate operations with exact ownership manifests.
- Audit service definitions, sockets, processes, launch material, caches, containers and logs after
  failure and removal.

## Acceptance

Successful upgrade retains company/schedule/artifact truth. Injected preflight and startup failures
roll back or stop once without duplicate effects. Uninstall preserves company data by default and
leaves no running service, socket, temporary launch material or owned cache.

## Makes deletable

In-place binary overwrite, migration-on-every-restart and broad filesystem/container deletion.
