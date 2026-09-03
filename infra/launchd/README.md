# Restless launchd services

Do not copy a plist from this checkout into `~/Library/LaunchAgents`.

`restless appliance install` generates `io.restless.plane.plist` and
`io.restless.wake-due.plist` from the activated, content-addressed release. The
definitions contain absolute release links and stable profile paths, are
validated as secret-free before activation, and are removed by `restless
appliance uninstall` without deleting company data.

The older `io.restless.reap.plist` is independent orphan-cleanup support. It is
not the control-plane supervisor or a schedule authority.
