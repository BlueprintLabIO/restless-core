# EXP-01 T0 — harness verification report

**Date:** 23 August 2026  
**Disposition:** passed; counted cognitive arms remained unopened

## Inputs

- Restless `dev` activation commit: `ff573da`.
- Renamed harness: `experiment/coordination-lab/v2/`.
- GPT runtime allocation: `gpt-5.6-sol` lead route and `gpt-5.6-terra` Staff route.
- Host headroom before verification: 155 GiB free; root target 14 GiB; experiment target 619 MiB.

## Observations

| Run | Result | Raw result SHA-256 |
|---|---|---|
| `exp01-t00-faults` | 39/39 fault, recovery, lease, artifact and database checks | `565957d21ac1cdab16dcffe44333e2798725bd8973b9ce45a70813bd1b4646e4` |
| `exp01-t00-baseline` | 39/39 B0/B1/B2 isolation and runtime-capability checks | `c5791207aa2fa9359d986ec66e2221df60ae2b1dfe2e023292b784e0526be218` |
| `exp01-t00-artifact-architecture` | 22/22 artifact, wake, integration, native-proof and drain checks | `177c681628d0e45ea60e54761dcdbea2cf7c1a90ad20d88b4ec99117f226bd47` |
| `exp01-t00-terra-handoff-r3` | Valid first-party terminal artifact handoff | `29f401c61f54b1d69b3a6e6941ed79b7d9777f2634cecf2115a4e529d55ed1d3` |

The valid Terra Attempt ran for 50.75 seconds and ended `produced`. It created the exact declared file,
passed its byte-level gate, returned terminal callback `attempt-e687d00593`, woke the accountable lead,
and exposed clean commit `3f795711bdee0dfd462d38b37f9871a5794feb95`. The turn used 1,437 output
tokens, 261 reasoning tokens and seven tools; 231,936 input tokens were reported cached.

The probe ran as an asynchronous supervised process. While its predecessor Terra Attempt remained
running, the Exec session independently inspected Git, the live Work/Attempt and its container at
04:19:21Z. This directly exercised post-dispatch availability rather than holding the executive loop
inside the actor turn.

Generated databases, workspaces and the marker artifact were confirmed ignored. The r3 database
returned `PRAGMA quick_check = ok`.

## Invalid preflight attempts

- `exp01-t00-terra-handoff` reached prompt assembly before the focused probe had established the native
  review capability. No model turn started. The probe now follows the normal full-run ordering.
- `exp01-t00-terra-handoff-r2` produced commit
  `bed78aeb35fdfde77e050c562dcebdae9f66f4b6`, but its reporting wrapper queried telemetry columns not
  owned by the Attempt table and crashed after the callback. Stable Attempt fields replaced that
  query. This is harness-invalid, not cognitive failure or counted evidence.

## Decision

Wave 0 runtime feasibility is closed. The renamed substrate, first-party Staff route, terminal callback,
artifact handle, native-review path and independent Exec availability are proved strongly enough to
freeze E01/E02. No fixed elapsed-time outcome rule was added.
