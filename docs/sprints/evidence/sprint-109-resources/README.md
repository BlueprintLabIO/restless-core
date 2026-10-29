# Sprint 109: resource efficiency, first slice

Observed on 29 October 2026 using official isolated `restless-dev` stacks and six
synthetic `_test` companies. No production company data or paid model was used.

## Matched idle comparison

Both stacks used 0.5 CPU / 1 GiB limits for the Runtime and 1 GiB for each
computer, with the browser idle timeout scaled to 1.5 seconds. All six browsers
were woken, then received read-only health requests every 150 ms for about 12
seconds. The broker checks its idle state every five seconds.

Before: all six Chrome processes remained running. Computer-container memory
summed to 2,368.6 MiB. After: all six stopped, summing to 608.81 MiB, a 74.3%
reduction in this sample. These figures describe the computer containers, not
the entire host or application. Short-window CPU samples are not a throughput
benchmark. Source measurements are in `idle-computers.json`.

The first after run woke only one browser and measured 505.71 MiB (78.6% less).
The all-six-woken result above is the fairer comparison and supersedes that
number. Untouched computers also started with Chrome stopped.

`scripts/verify-compute-idle.mjs` records actual Docker memory/CPU/process counts,
asserts Chrome stops despite health probes, and checks no model requests occur
while idle. A synthetic request to the loopback mock first proves request
logging works. Both the one-browser and all-browser variants passed.

## Owner path and AI session checks

The live Runtime probe `cold_browser_opens_for_owner_and_survives_broker_restart`
passed against `resource3_test`: an actual desktop WebSocket opens from cold,
Chrome remains awake while watched, sleeps after disconnect, an HTTP link opens
from cold, and restarting only the broker recognizes a running browser.
The accelerated timeout exposed stale activity tracking in direct host wakes;
waking through the broker fixes it.

The ACP suite passed 15 tests (two live-process probes excluded). It verifies
persisted context size/time gates actual session reuse after serialization,
legacy locators remain readable, warm/small sessions keep resuming, and large
expired or oversized histories reconstruct. Runtime suite: 37 passed, with the
live probe run separately. This demonstrates policy enforcement, not a measured
production token or dollar saving.

## Remaining A0 work

This slice does not complete A0. Host-wide CPU/RAM admission and reserve,
configurable resource caps and their Resources UI, storage attribution/history,
AI budget coverage and representative useful-outcome/token benchmarks remain
open. Existing spend and native monthly token limits must be reused and audited.
The desktop container itself still starts eagerly; only Chrome is lazy here.
Production was not rolled out by this evidence run.

## Shared host cap and Resources observations

The next Core slice places company Runtimes, split computers, local burst jobs,
local MCP workers and local native Documents under the Fleet-installed shared
parent. The account plane and database remain outside it. A configured missing,
unbounded or swappable pool refuses new starts before replacing a live Runtime.
Matching-image reconciliation also checks the parent; Documents stop cleanly
before a parent/image change.

Resources uses actual cgroup counters, not Docker allowance totals presented as
usage. Company work includes local jobs/tool workers/Documents, the computer is
separate, and shared host capacity and reserved platform RAM are labelled. Host
readings are owner-only. AI rows reuse the existing seven-day usage projection;
unknown tokens stay unknown and price estimates are labelled as estimates.
Existing spend, native-token/turn and runtime-hour controls remain authoritative.
The page makes no model call and does not wake Chrome.

### Application load check

Four actual `restless job run` jobs exercised a temporary test slice: two
memory workers first, then two CPU workers. With a shared 768 MiB cap the pool
reported 737,370,112 bytes and 3,759 memory.high events, with no OOM kill in that
sample. The CPU workers used 2.05298 CPU seconds over 4.071255 wall seconds under
a shared half-core allowance. The owner answered all 16 health probes, and its
compute API remained available. Per-job allowances exceeded the shared cap;
the parent constrained their aggregate. `shared-jobs.json` records the result.
`scripts/verify-compute-caps.mjs` reproduces this through the real job interface,
refuses another workload sharing the temporary slice, and restores its limits.

Two real company computers inside the shared pool were woken and returned to
Chrome STOPPED despite health probes. The calibrated mock recorded zero model
calls while idle. The existing company budget was set to zero for one synthetic
Exec wake: the real wake refused for its budget before a model request, and the
original $10 budget was restored. This checks existing budget enforcement; it is
not an AI cost-saving benchmark.

### UI checks

The Reno-scale seed passed `capture-surfaces` at 1440, 1024 and 390 pixels,
plus dark/reduced-motion at desktop and phone size. The built-in browser was
also inspected at desktop and phone sizes. `verify-resources.mjs` measured zero
movement of the following section when readings arrived, retained readings
after a failed refresh, showed unknown values for unavailable/unconfigured
pools, and recovered after retry. `verify-motion open-latest` passed.

Visual calibration: Beautiful UI's quiet information hierarchy, Cult UI's
stable card geometry, and Origin UI Svelte's compact table spacing were compared
with the live desktop and phone result. Existing Page/Section/Row primitives
remain the implementation; no component library or runtime was added.

### Still open

This is shared hard-cap enforcement and observation, not completion of A0.
At-capacity heavy-work queue/resume reasons, per-company CPU/RAM editing,
resource history/capacity purchasing, wider idle-service reductions and matched
AI cost/quality benchmarks remain open. Storage is a shared filesystem reading,
not a fabricated per-company quota. No paid production resize was applied.

The complete self-hosted identity multiplayer journey passed under the shared
pool with actual verified owner/admin/member accounts, live document coediting,
role changes and revocation. Native Documents used the configured parent.
Opening an owner-only Resources link as a member exposed a notification effect
that read its own reactive writes; running that navigation outcome untracked
removed the loop, and the full journey then passed without page errors.
A live Runtime test also confirmed that a missing configured pool refuses
reconciliation without stopping or replacing the existing container.
