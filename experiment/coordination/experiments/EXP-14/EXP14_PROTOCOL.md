# EXP-14 controller protocol

The authoritative sprint is `/company/reference/exp-sprint-14-swift-arrival-tight-loop.md`.

## Invariants

1. Exec delegates executable work to one accountable lead and returns.
2. The lead does no planned production.
3. Production uses `/company/repos/swift-arrival`; independent players never receive that repository.
4. Each candidate playtest uses a fresh read-only copy under `/company/playtest` and fresh native/model
   processes.
5. Player Work is repo-less, source-blind and sequential.
6. Direct pixels, not commands, filenames, logs or model prose, establish gameplay.
7. The player receives an opaque target handle from `native-session launch`; it never enumerates or
   guesses titles.
8. At most 12 retained PNGs per run. Transient captures are deleted after the terminal manifest.
9. Stop on the first conclusive blocker. After one permitted evidence-driven repair, restart the
   two-run gate from zero.
10. Two fresh complete journeys are required. Founder taste remains unclaimed.

## Frozen product outcome

Collect the parcel, deliberately drop and recover it, enter the driver seat, drive the loaded truck
from route 0 to route 40, exit at the destination, unload in the valid zone and observe explicit
host-owned delivery completion. A clean negative run must reject on-foot route-zero unload. A clean
recovery run must permit an early exit and natural seat re-entry.

## Scratch Runtime boundary

`/company/tools/native-session` may implement only `launch`, `observe`, `act`, `export` and `stop`.
It owns process/window identity, exact capture, focus, bounded input, health and compact evidence. It
does not choose actions, interpret pixels, know game semantics, create Work, access credentials or
become a service/database.

## Terminal outputs

Write final synthesis only below `/company/outputs/exp14`: `RESULTS.md`, `FRICTIONS.md`, `metrics.json`
and compact per-run text/JSON manifests. Keep raw PNGs transient and outside the repository.
