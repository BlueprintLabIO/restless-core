# Sprint 38 seven-day dogfood ledger

**Started:** 4 September 2026

**Release at start:** `7e894de42b0ec9c059b0`; final day-zero repair `dc2d9f2b288e96a97ef4`

**Terminal decision due after:** 11 September 2026

This ledger records ordinary use of the stable appliance. A green implementation run starts the clock;
it does not satisfy elapsed reliability.

| Day | Required observation | Result |
| --- | --- | --- |
| 0 — 4 Sep | Install, upgrade, crash restart, wake, rollback, uninstall/reinstall, desktop/mobile Open QA | Pass; see `RESULTS.md` |
| 1 — 5 Sep | Ordinary owner entry and scheduled work; no daemon babysitting | Pass; no economic occurrence was due on Saturday, and the wake-only transport correctly settled no work |
| 2 — 6 Sep | Repeated artifact Open and one concurrent dev session | Pass; repeated same-origin Open was idempotent and an isolated dev plane ran beside stable without sharing state |
| 3 | Mac sleep across a due instant; verify declared misfire policy | Pending |
| 4 | Ordinary owner entry and schedule audit | Pending |
| 5 | Real reboot/login and singleton recovery | Pending |
| 6 | Founder-controlled Swift Arrival pickup, drive and unload | Pending |
| 7 | Replay exact cleanup/isolation probes and publish terminal decision | Pending |

For each day record the installed release, owner entry result, schedule occurrence identities, recovery
latency, manual intervention, unexplained Attention, artifact friction and residue. Any repair restarts
the affected lane's observation window; it does not erase the failure.

## Day 1 — 5 September 2026

- **Release and supervision:** `dc2d9f2b288e96a97ef4` remained current. The plane had run under
  `launchd` since 15:07 AEST on 4 September with one launch, the same PID, no exit and no manual
  daemon intervention. Recovery latency was not applicable because no restart occurred.
- **Owner entry:** `/health` returned 200 in 0.000453 s and the owner shell returned 200 in
  0.003243 s. CLI and API independently reported `ready`; the model gateway and `launchd` schedule
  transport reported ready.
- **Durable schedule decision:** the wake-only LaunchAgent had completed 1,062 runs with last exit
  code 0. Its 09:02:26 AEST observation reached the live plane. No economic occurrence was due on
  Saturday: Aris schedule `45155efc-fa27-4347-9459-0da474850873` retained exactly its prior fired
  occurrences and advanced to 09:00 AEST Monday, 7 September. No Work was manufactured to make the
  dogfood look active.
- **Attention and artifact friction:** Aris projected four source-owned owner items: two existing
  Work handoffs and two distinct Authority approval records for the same party. Runtime/browser were
  unavailable because every retained company Runtime was stopped, not because owner entry failed.
  No artifact Open was assigned to day 1, so none was claimed.
- **Residue and diagnostics:** no Sprint 38 container, volume, image, temporary root or launch-cache
  entry existed. The scheduled reaper incorrectly labelled the live stable socket stale at 09:00;
  direct CLI, socket and owner API checks immediately proved it healthy. This is a diagnostic false
  positive to retain as evidence, not a daemon repair. After final activation the only new severity
  event was one 1.237 s slow read-only Authority query; no crash, recovery, schedule or owner error
  was observed.

## Day 2 — 6 September 2026

- **Release and supervision:** `dc2d9f2b288e96a97ef4` remained current and `ready`. The stable
  plane still had one `launchd` run, the same PID and no exit. Owner health returned 200 in
  0.000730 s and the owner shell in 0.001038 s; no stable recovery or manual intervention occurred.
- **Repeated artifact Open:** the retained Aris volume was started from
  `restless-company-image:latest`; Company Computer became `ready` immediately. Two requests through
  the same-origin owner boundary both returned the exact `company_computer` route
  `/aris/company/computer`, whose shell returned 200 in 0.001238 s. Runtime container count stayed
  one, so repeated Open created no duplicate. A request without an Origin was correctly refused by
  the local-owner boundary. The Runtime was then stopped and its ephemeral container removed while
  its durable company volume remained.
- **Concurrent development:** an isolated `dev` plane using namespace `s38_day2_dev`, its own
  temporary state root, database, socket and port 26168 ran concurrently with stable on port 7788.
  Dev reported `development` with zero companies while stable remained `ready` with all six company
  definitions and the same release/PID. The bounded dev session was stopped; its port closed, its
  isolated database was dropped and its temporary root was moved to Trash.
- **Schedules, Attention and friction:** the wake LaunchAgent had completed 2,493 runs with last exit
  code 0. Sunday had no due economic occurrence; Aris retained its next weekday occurrence at 09:00
  AEST Monday. The same four source-owned owner items remained and no new unexplained Attention
  appeared. The full source-building `restless-dev` path was below its declared 30 GiB safety floor
  with 27 GiB free, so this lane used the installed exact release to test profile concurrency rather
  than manufacturing build headroom or claiming source-build ergonomics.
- **Residue:** no Sprint 38/day-2 test container, volume, image, temporary root or launch-cache entry
  remained. No new daemon warning or error appeared after the day-1 observation.
