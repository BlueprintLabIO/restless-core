# Sprint 38 appliance acceptance corpus

**Frozen:** 3 September 2026
**Counted host:** macOS 26.6.2 (25G83), arm64, user uid 501
**Stable service:** `io.restless.plane`
**Stable wake service:** `io.restless.wake-due`
**Stable root/socket:** `~/.restless`, `~/.restless/restlessd.sock`
**Stable owner origin:** `http://127.0.0.1:7788`

This is the executable product corpus for Sprint 38. A green unit suite is necessary but not
sufficient: the counted macOS lanes also require process, HTTP, database and owned-resource probes.
Every disposable company name ends in `_test`; a fixture that cannot prove its profile is refused.

## Frozen bounds

| Condition | Bound |
| --- | ---: |
| Fresh login to ready owner surface | 30 seconds |
| Forced daemon death to ready replacement | 30 seconds |
| Native wake invocation | 10 seconds |
| Local launch preparation | 60 seconds |
| Graceful service stop | 30 seconds |
| Schedule backlog replay | At most one execution per wake |

Stable, dev and test must use different roots, sockets, port ranges, logs, launch caches and Docker
resource namespaces. Stable uses port offset `0`; dev uses `1000..19999`; test uses
`20000..50000`. A service definition may contain paths and non-secret profile metadata, but never a
credential, invitation, prompt, company payload or bearer token.

## Machine lanes

| ID | Input | Durable/user result | Process/resource proof |
| --- | --- | --- | --- |
| M1 | Install then bootstrap `io.restless.plane` | One stable service becomes ready | One lock holder, one daemon pid, stable socket and HTTP 200 `/health` |
| M2 | Start a second daemon on the stable root | Refused before migration/listener start | Original pid/socket unchanged; second exits non-zero |
| M3 | `SIGKILL` the supervised daemon | Owner surface returns within 30s | Replacement pid differs; singleton count remains one |
| M4 | Run stable and explicit dev profiles together | Both surfaces work independently | Roots, sockets, offsets, logs and Docker names are disjoint |
| M5 | Invoke dev stop/reset/cleanup against stable identifiers | Operation is refused | Stable pid, root digest and Docker resource ids are unchanged |
| M6 | Corrupt or remove the wake service definition | Appliance reports degraded scheduling transport | Restless schedule truth remains intact; no schedule is deleted |
| M7 | Uninstall without purge | Service, wake entry, sockets and owned caches disappear | Company configuration and databases remain present |
| M8 | Upgrade failure before activation | Last-known-good remains active | Previous binary digest and ready health are restored |

## Schedule lanes

Every case asserts a durable occurrence row with one of `fired` or `skipped`, a delivery identity,
the subsequent `next_fire_at`, and no second Work/Attempt for the same occurrence.

| ID | Input | Expected decision |
| --- | --- | --- |
| S1 | Normal in-process due time | Fire the exact occurrence once |
| S2 | OS wake and timer arrive twice/reordered | One claim; duplicate signals are reads |
| S3 | Daemon dies before durable claim | Replacement claims the occurrence once |
| S4 | Daemon dies after claim but before completion | Reconcile the owed claim; never manufacture completion |
| S5 | Resume inside `catch_up_once` lateness bound | Fire one overdue occurrence, then advance beyond now |
| S6 | Resume after `skip_if_late` bound | Record skipped with lateness reason; do not wake actor |
| S7 | Several overdue recurring instants under `coalesce_latest` | Record superseded instants and fire only the latest useful one |
| S8 | Large overdue backlog | Consider a bounded set and execute no more than one occurrence |
| S9 | Schedule cancelled before a late wake | No claim; cancellation remains terminal |
| S10 | Sydney DST gap/fold | Deterministic next instant; no duplicate wall-clock execution |
| S11 | Clock moves backwards | Existing delivery identity prevents repeat execution |
| S12 | Timing requirement exceeds local guarantee | UI says `requires_always_on_runner`; no false local guarantee |

## Artifact-opening lanes

| ID | Input | Owner result | Security/cleanup proof |
| --- | --- | --- | --- |
| A1 | Exact released HTTPS artifact with embed allowed | Opens in the Cockpit content surface | Opaque session exchange; no bearer in URL/history/log |
| A2 | Embed denied by origin policy | Explains denial and offers only an allowed external open | No permissive iframe fallback |
| A3 | Exact Swift Arrival macOS artifact | Digest verified, native client launches and joins matching prepared session | No shell interpolation; reusable invitation stays out of argv/environment and the client receives only an expiring opaque local handle |
| A4 | Wrong digest/platform/audience or expired grant | Open is disabled/refused with the exact reason | No process starts and no material remains |
| A5 | Non-packaged visual outcome | Opens the existing Company Computer stream | Labelled `company_computer`; no public Runtime port |
| A6 | Repeated Open | Reuses prepared identity/session or reports it | No duplicate publication/download/process |
| A7 | Cancellation, expiry or uninstall during preparation | Preparation stops and access becomes terminal | Temporary download, handle and child process are absent |

## Baseline (before implementation)

- A daemon was running from a source checkout and answered `/health`, but no stable plane LaunchAgent
  was installed; only the cleanup reaper was present.
- `infra/launchd/io.restless.plane.plist` contained a checkout-specific debug-binary path and `/tmp`
  log path, so it was not a relocatable release artifact.
- `restless-dev` shared the default state root, target tree and owner ports unless the operator knew
  which environment variables to override.
- The resident scheduler used a five-second scan as its recovery source and had no OS wake entry.
- Artifacts could be inspected through resource/review/computer surfaces, but no single bounded Open
  action chose embedded, native or streamed execution.

The terminal dogfood records each lane's commands, timestamps and evidence under
`docs/dogfood/sprint-38/`. Screenshots and transient downloads are kept outside Git and removed after
the evidence is reduced to text.
