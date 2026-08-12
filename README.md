# restless

An autonomous-company control plane for ordinary owner/operators. **The product runs the business for
you:** the human owner provides judgement, taste, sign-off, and the prepared last mile; the singleton
Exec and Staff do the work.

> **Status:** clean-slate rebuild. No runnable system yet — this repository holds the target
> architecture and sprint scaffolding only. See `ARCHITECTURE.md` and `CLAUDE.md`.

## What this is

Restless is the rebuild of an earlier control plane (held elsewhere) according to the architecture in
[`ARCHITECTURE.md`](./ARCHITECTURE.md). That architecture defines three logical layers:

1. **Constitutional Kernel** — governance, authority, secrets, budgets, external effects, recovery.
   Small, strict, deterministic.
2. **Organisational Intelligence (`OrgIntel`)** — self-running / self-healing / self-building
   coordination across agents. Opinionated but recoverable.
3. **Company Linux Runtime** — the persistent, capable computer where agents do real work and produce
   economic output. Powerful, messy, productive.

The kernel bounds authority. OrgIntel keeps the organisation coherent. The runtime gives agents the
freedom and tools to produce real economic work.

## Why a clean slate

The prior implementation accumulated load-bearing architecture that the target architecture
(`ARCHITECTURE.md` §3.4, §12) names as anti-patterns: a single universal command type for every
mutation, an append-only ledger capturing every internal action, a content-addressed asset-custody
machine, and a per-turn disposable execution sandbox. These are structurally hard to refactor away, so
we rebuild on the target architecture and lift only proven components from the prior system.
[`docs/SALVAGE.md`](./docs/SALVAGE.md) records what is proven and may be lifted, versus what is greenfield.

## Repository layout

```
ARCHITECTURE.md          target architecture (the v0.9 source of truth — read first)
CLAUDE.md                working agreement for humans and coding agents
AGENTS.md                symlink → CLAUDE.md (one source of truth)
docs/
  sprints/               sprint specs (sprint-NN.md); see docs/sprints/README.md
  SALVAGE.md             proven-component lift map from the prior implementation
crates/
  restlessd/             the daemon (empty stub — the §4.4 stable coordination core)
  restless/              the CLI (empty stub)
```

Layer crates (`kernel`, `orgintel`, `runtime`) are intentionally not pre-scaffolded. They are grown
from the first sprint slice that needs them.

## How we work

Sprint-driven, two founders on `dev`. See the "How we work" section of [`CLAUDE.md`](./CLAUDE.md):

> `ARCHITECTURE.md` → sprint spec (founders align) → coding agents break into tickets → founders
> align on tickets → implement as a goal-mode sprint on `dev`.

## License

TBD.
