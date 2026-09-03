# restless

An autonomous-company control plane for ordinary owner/operators. **The product runs the business for
you:** the human owner provides judgement, taste, sign-off, and the prepared last mile; the singleton
Exec and Staff do the work.

> **Status:** active clean-slate rebuild with a runnable local company appliance. The walking
> skeleton spans the coordination daemon, OrgIntel, persistent company Runtime, owner cockpit and
> Authority boundaries; it is not yet a production release. See `ARCHITECTURE.md`, `CLAUDE.md` and
> the current sprint report for proved versus open outcomes.

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
LLM_CURE.md              failure modes and their cures — how we think; read before designing
AGENTS.md                symlink → CLAUDE.md (one source of truth)
docs/
  sprints/               sprint specs (sprint-NN.md); see docs/sprints/README.md
  COORDINATION_THEORY.md accountability boundaries and current team-shape theory
  SALVAGE.md             proven-component lift map from the prior implementation
crates/
  restlessd/             daemon: coordination, Authority adapters and owner API
  restless/              owner/operator CLI
  restless-orgintel/     recoverable per-company organisational state
  restless-model-gateway/  host-side model routing and spend accounting
infra/company-image/     persistent company-computer image
scripts/restless-dev     supported local cockpit stack
web/                     live company-scoped owner cockpit (SvelteKit)
```

The workspace still avoids speculative layer crates. A boundary becomes a crate or service only
when a proved slice needs an independent ownership or failure boundary.

`web/` began as the strongest salvaged surface, then had its fixture truth removed. It now reads and
writes through company-scoped owner APIs; the supported local entrypoint is `scripts/restless-dev`,
not a standalone Vite shell. See [`web/README.md`](./web/README.md).

The hosted control plane and public website live in a separate, private repository. This repository
contains the open local company cell, architecture, and experiment evidence.

## How we work

Sprint-driven, two founders on `dev`. See the "How we work" section of [`CLAUDE.md`](./CLAUDE.md):

> `ARCHITECTURE.md` → sprint spec (founders align) → coding agents break into tickets → founders
> align on tickets → implement as a goal-mode sprint on `dev`.

## License

Restless Core is licensed under the [Apache License 2.0](./LICENSE). The Restless name, marks, visual
identity, hosted service, and private cloud control plane are not licensed by this repository.
