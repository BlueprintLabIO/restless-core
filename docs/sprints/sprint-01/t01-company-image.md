# T1 · Company image + persistent container lifecycle

**Layer:** Runtime — this is the company computer itself (ARCHITECTURE.md §5).
**Serves:** Greenfield #1 in `docs/SALVAGE.md`. Every other ticket runs inside this.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** Nothing. Can start immediately, in parallel with T3.

## Build

- `infra/company-image/Dockerfile` on a Debian base: node, python3, git, ripgrep, chromium, build tools, and the ACP agent binary. Package list salvaged from the legacy `infra/sandbox-agent/Dockerfile`.
- **The entrypoint is a long-lived init, not an agent.** This single line is the whole inversion away from the legacy per-turn disposable sandbox (§5, §17 step 2). Agents are ordinary processes started later, not the reason the container exists.
- One named volume per company mounted at `/company`, seeded with the §5.3 skeleton: `mission.md`, `org/`, `goals/`, `projects/`, `decisions/`, `knowledge/`, `outputs/`, `repos/`, `workspaces/`.
- `runtime::container` module in `restlessd`: up (create if absent, then start), down (stop, keep volume), status, exec.

## Acceptance

`restless up cosmon` produces a running container with `/company` seeded. Write a file into `/company`, run `down`, run `up` — the file, Git history, and browser profile survive.

## Salvage

Adapter image package list. **Re-validation:** strip the single-entrypoint and tmpfs-home assumptions; confirm the image runs as a persistent multi-process company computer.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
