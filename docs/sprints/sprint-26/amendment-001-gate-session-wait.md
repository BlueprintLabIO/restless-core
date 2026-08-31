# Sprint 26 amendment 001 — wait for the governed gate session

**Status:** Passed and active for EXP-16

**Date:** 30 August 2026

**Parent evidence:** [`run-report.md`](run-report.md) and
[`activation-receipt.json`](activation-receipt.json)

## Why the parent receipt needed amendment

EXP-16 produced a deterministic contradiction on its first real game gate:

- the declared command returned exit code 0;
- Runtime captured no terminal output;
- the real verifier was still running; and
- Runtime counted and killed one or two members of the marked process group as leaks.

The game and its verifier were not the cause. Runtime launched the gate through GNU `setsid` without
waiting for the child session. When Docker's entry process was already a process-group leader,
`setsid` forked; its parent returned 0 to `docker exec` while the child continued executing the real
gate. Runtime therefore observed a false terminal boundary, empty evidence and apparently leaked
processes.

The controller reproduced this directly before changing source: `docker exec` returned after a
`WRAPPER-START` marker while the recorded process group still contained the wrapper and
`verify-delivery.sh`. This is a first-party Runtime defect, not a model, product or provider failure.

## Bounded repair

The gate wrapper now uses `setsid --wait`. Docker stays attached to the child session and receives its
real stdout, stderr and exit status. Runtime's existing timeout and exact process-group reaper remain
the safety boundary.

The integrated fixture now contains a regression gate that prints a start marker, waits one second and
prints a terminal marker. Acceptance requires:

- observed duration of at least 900 ms;
- both markers in captured output; and
- zero leaked processes.

One unit regression also pins the waiting-session launch contract.

Restarting the patched daemon exposed a separate byte-integrity regression: committed migration 20
had lost one trailing newline after it was already applied. SQLx correctly refused to start. The file
is restored byte-for-byte to the applied SHA-384 checksum; the schema and migration meaning did not
change.

## Requalification evidence

- Focused unit regression: 1 passed, 229 filtered.
- Full live integrated fixture: 1 passed, 229 filtered, 35.38 seconds. Its three expected negative gate
  lines remained negative evidence.
- EXP-16 qualification Work: `6b9c41f2-235e-4a04-8ec2-d33ddcad9783`.
- EXP-16 qualification Attempt: `153c3779-8408-4ea2-8997-4f97cd614c4a`.
- Governed gate: `e68f14f8-aec8-4b38-bf5b-d095ac248cba`.
- Gate run: `b6a8f2f8-8e21-42ab-afd5-bf57f4cefb24`.
- Runtime leased port 41504 plus exact temporary-directory and process-group resources, then released
  all three with reason `governed gate finished`.
- Terminal evidence: exit 0, `PASS mechanics`, 8,809 ms, zero leaked processes.
- The frozen EXP-16 company container and product candidate bytes were unchanged.
- A clean S0 replay then passed `route-zero-rejection` and `mechanics`, both with terminal output and
  zero Runtime leaks.

## Disposition

This amendment supersedes only the gate-session launch fingerprint in the parent activation receipt.
All other Sprint 26 coordinates and claims remain scoped to that receipt. EXP-16 may count gates only
while using this patched host daemon semantics or a later build that independently passes the same
regression.

