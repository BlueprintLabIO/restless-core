# Company operating rules — maintainer notes

`COMPANY_OPERATING_RULES.md` is injected verbatim into every Exec and Staff prompt, so it holds only
runtime instruction. This file keeps what maintainers need and agents do not.

Standing rules for every agent in every company, injected into every wake. This is **not** the
product's constitution — that is a document about what Restless is, for the people and coding
agents who build it. This is the opposite direction: runtime instruction, for the agents who
operate inside a company. A mission says what one company is for; this says how anyone working
here behaves, always. It is short on purpose — every line costs tokens on every wake of every agent, so
a line earns its place only by having prevented an observed failure.

Ported from the legacy `docs/constitution/` set (`domain-concepts`, `helm-core`, `helm-port`,
`tech-stack`) in the sibling helm repo, **selectively**. That set is ~100KB and encodes the legacy
architecture as much as the product's soul. The selection is recorded below: what was carried, what was deliberately left, and why. Read it
before adding anything back.

## What was carried, left, and deferred

The legacy set is a genuine achievement and also the architecture this repo is a clean-slate
rebuild *away from*. Porting it wholesale would reinstall the anti-patterns through the one
document every agent reads every wake — the highest-leverage possible way to poison the well.

**Carried** (product soul, architecture-neutral):
claim/observation/evidence/fact; model-assisted judgement escalating but never unlocking; the three
kinds of action; tool/provider acceptance as evidence rather than outcome; terminal reports never
silently lost; authority never implicit; the prepared last mile; mature components first.

**Deliberately left** — these are the anti-patterns `CLAUDE.md` names, and they are the *spine* of
`helm-core.txt`, not incidental to it:

- **The universal command.** `HELM COMMAND` / `COMMAND ENVELOPE` / `COMMAND LIFECYCLE` /
  `ATOMIC APPLICATION`, and operating agreement 5 — *"every company change is expressed as a
  versioned command and recorded in the Ledger."* Internal coordination here is ordinary
  recoverable state, not a kernel command algebra (ARCHITECTURE.md §3.4, §4.7, §12).
- **The append-everything ledger.** `LEDGER`. Only governance-relevant truth is durable here; the
  operational event stream may be compacted, repaired, or regenerated (§3.2, §4.4, §12).
- **"Helm is the authoritative record"** (operating agreement 1). Here, files and git hold the
  work; the coordination store is recoverable by design and is not the company's memory.
- **Governed asset custody**, which the legacy set assumes throughout its evidence and asset
  versioning. Work is files (§2.4, §5.3, §12).

**Deferred, not rejected** — right ideas, no evidence yet:
`helm-port.txt`'s full apparatus (profiles, conformance classes, discovery, marketplace, two
independent implementations before a capability stabilises). The thin version of its spine —
class + purpose + ordinary argv → generic receipt, with idempotent replay — already exists here as
the governed-process effect surface and has run live. The rest is speculative generality until
repeated real tools demonstrate a shared need.

**Vocabulary**: `domain-concepts.txt` is largely sound and mostly survives in ARCHITECTURE.md's own
language. It is not re-imported wholesale because it carries the ledger and command vocabulary with
it, and words shape architecture.
