# S04-T11 · CLI-first runtime reconciliation

**Layer:** Company Runtime / Owner surface
**Serves:** AC11 and the prepared-last-mile rule: normal company operation must not require the owner
to translate a Restless action into raw Docker commands
**Depends on:** S04-T10 (runtime lifecycle changes are owner acts)
**Makes deletable:** the manual `docker build`, `docker exec`, PID inspection and binary-copy repair
transcript used during this sprint's live Aris run

---

## The observed friction

Sprint 4's live run found that the persistent Aris container carried an older `restless` binary than
the daemon and source tree. `restless up` reported the already-running container as healthy and did
not reconcile it. Repair then escaped through raw Docker to rebuild the image, replace or copy the
binary, inspect concurrent Kimi processes, and pause the Exec so its old broad cleanup did not kill a
critic.

Docker is still the correct V0 packaging and lifecycle mechanism (`company-runtime` §2.1). The defect
is that it leaked through the Runtime Bridge into ordinary owner operation.

## Classification

- Image identity and process state are **deterministic and enumerable**. Probe them; do not ask an
  agent to narrate them.
- Work inside the company computer is **open-ended**. Keep one generic door into Linux; do not add a
  Restless API for every command or process.
- Losing a live session during an image refresh is **guarded**, not an invariant: refuse automatic
  replacement while supervised actors run, and preserve the volume when the owner explicitly stops.

## Scope

1. **`restless attach -c <company> -- <command...>`** runs an ordinary command in the company
   computer. Omitting the command keeps the interactive shell. This is the generic runtime surface;
   there is no `restless ps`, file API, or command catalogue.
2. **`restless doctor -c <company>`** reports container state, running and target image identities,
   source and image digests, and `current | required | unknown`. Unknown never renders as current.
3. **`restless up -c <company> --reconcile`** hashes the actual image inputs, rebuilds only when the
   source label differs, replaces an outdated container, reuses the named volume, and seeds the
   mission through the existing path.
4. **Active-session refusal.** Reconciliation refuses while a supervised Exec or Staff session is
   running and names the actors. The owner may wait or use the existing `down` command explicitly.

**Not in scope:** a custom container runtime; one verb per Linux operation; process semantics in
OrgIntel; daemon/Postgres host-service installation; hiding Docker from out-of-band security and
recovery verification.

## Acceptance

Headless, observed against Aris.

1. `restless attach -c aris -- ps ...` and a file probe work without the operating transcript
   containing a raw Docker command.
2. `restless doctor -c aris` reports the deliberately stale/unlabelled runtime as `required` or
   `unknown`, never `current`, and gives the exact Restless reconciliation command.
3. `restless up -c aris --reconcile` refuses while a supervised actor runs.
4. After the runtime is explicitly stopped, reconciliation rebuilds/replaces the container; a marker
   written to `/company` before replacement remains afterward, and `doctor` reports `current`.
5. The in-container `restless --help` exposes the current CLI, proving image reconciliation rather
   than only host-side compilation.

---
Sprint spec: [`../sprint-04.md`](../sprint-04.md)
