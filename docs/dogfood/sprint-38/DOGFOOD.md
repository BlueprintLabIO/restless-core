# Sprint 38 seven-day dogfood ledger

**Started:** 4 September 2026

**Release at start:** `7e894de42b0ec9c059b0`; final day-zero repair `dc2d9f2b288e96a97ef4`

**Terminal decision due after:** 11 September 2026

This ledger records ordinary use of the stable appliance. A green implementation run starts the clock;
it does not satisfy elapsed reliability.

| Day | Required observation | Result |
| --- | --- | --- |
| 0 — 4 Sep | Install, upgrade, crash restart, wake, rollback, uninstall/reinstall, desktop/mobile Open QA | Pass; see `RESULTS.md` |
| 1 | Ordinary owner entry and scheduled work; no daemon babysitting | Pending |
| 2 | Repeated artifact Open and one concurrent dev session | Pending |
| 3 | Mac sleep across a due instant; verify declared misfire policy | Pending |
| 4 | Ordinary owner entry and schedule audit | Pending |
| 5 | Real reboot/login and singleton recovery | Pending |
| 6 | Founder-controlled Swift Arrival pickup, drive and unload | Pending |
| 7 | Replay exact cleanup/isolation probes and publish terminal decision | Pending |

For each day record the installed release, owner entry result, schedule occurrence identities, recovery
latency, manual intervention, unexplained Attention, artifact friction and residue. Any repair restarts
the affected lane's observation window; it does not erase the failure.
