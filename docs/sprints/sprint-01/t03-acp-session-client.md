# T3 · ACP session client (spike two, purge one)

**Layer:** Runtime — how agents run as ordinary processes (§5.2, §4.3).
**Serves:** The sprint's main technical unknown. `docs/SALVAGE.md` flags the legacy extraction as high-friction: the ACP orchestration is interleaved with sandbox transport bridging and custody `await_result`.
**Makes deletable:** The losing branch. See below.
**Depends on:** Nothing. **Blocks T4, T9, and the companies.** Start here.

## This is a branch-and-purge ticket (LLM_CURE.md frame 3)

Build the smallest runnable version of **both** candidates in the scratchpad, then let a live probe decide. Do not resolve this by argument.

- **(a) Extract** from legacy `contained.rs` (~400–500 LOC), cutting away the envelope/fence/tunnel scaffolding.
- **(b) Fresh client** against the ACP spec: JSON-RPC 2.0 over stdio — `initialize` → `authenticate` → `session/new` → `session/prompt`, with `session/update` notifications streamed back, and client-side `fs/read_text_file`, `fs/write_text_file`, `session/request_permission` handlers.

Working hypothesis: (b) is *smaller* than (a), because a persistent container deletes the fence and tunnel machinery that made the original complex. Hypothesis, not a conclusion.

## Second fork inside this ticket: where the client runs

- **(i)** `restlessd` on the host spawns agents via `docker exec` and speaks JSON-RPC over that stdio.
- **(ii)** A small in-container supervisor process.

Prefer (i) — it avoids building a whole component — unless the spike shows otherwise.

## Acceptance

Live-probe a real agent binary end to end: initialise → session → prompt → observe streamed output → cancel. **Record result, agent version, and probe time** (CLAUDE.md → "Probe, never guess"). Then delete the losing branch — not behind a feature flag, deleted. Git holds it.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
