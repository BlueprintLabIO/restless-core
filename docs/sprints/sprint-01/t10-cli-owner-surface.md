# T10 · CLI owner surface

**Layer:** Owner surface — the CLI is the only owner surface this sprint. The SPA is not in this repo yet.
**Serves:** Without this the owner cannot issue a directive, watch a run, answer a judgement request, or take over. §7.4 says support only the human interactions real dogfood requires.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T1, T5.

## Build

Unix socket to `restlessd`. Commands:

| Command | Does |
|---|---|
| `up` / `down` / `status` | Company environment lifecycle |
| `tell "<directive>"` | Issue an owner directive |
| `watch` | Stream the operational event stream |
| `attach` | Drop into the company shell or browser session |
| `goals` / `commitments` / `inbox` | Inspect coordination state |

**Answering a blocked judgement request** goes through `tell` — the human judgement point in the sprint acceptance criteria needs a way to unblock, and it should not be a separate command.

## Posture (§4.2, CLAUDE.md → product soul)

Outcomes, decisions, risk and next actions first. Roles, prompts, permissions, spend detail and logs only on request. **This is a calm work surface, not an agent administration dashboard** — even in a terminal.

## Acceptance

A full Cosmon run is driven start to finish from the CLI: directive in, work observable, judgement point answered, artifact located.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
