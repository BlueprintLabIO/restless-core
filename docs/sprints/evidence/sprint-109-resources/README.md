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
