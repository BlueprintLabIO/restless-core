# UXG-254 — Surface disk, memory and CPU pressure on the company computer

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | missing-function | US-147 |

## Problem

The Runtime is started with CPU, memory and process limits, and a runaway is OOM-killed inside its cgroup, but the doctor reports only container, volume, image, supervisor and browser state. No page shows disk use on the company volume, memory pressure, OOM kills or CPU saturation. A build that fails on a full disk or a browser killed for memory appears to the owner as an unexplained failed run.

## Evidence

- `crates/restless-engine/src/runtime.rs:1341-1363` (RuntimeDoctor has no resource fields)
- `crates/restless-engine/src/runtime.rs:1686-1697` (cpus, memory without swap, pids limits)
- `web/src/lib/components/CompanyLimits.svelte:418-500` (Computer section shows sleep and hours only)
- `web/src/routes/[companyId]/company/doctor/+page.svelte:232-240` (raw state is a JSON dump)

## Suggested direction

Probe volume free space, memory headroom and recent OOM kills, and raise a Health check with a plain next step when one is low.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
