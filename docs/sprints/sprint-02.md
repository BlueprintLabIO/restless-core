# Sprint 02 — Does OrgIntel earn its place?

**Status:** Draft for founder alignment
**Date:** 13 August 2026
**Supersedes:** the mid-sprint-01 draft of this file, which assumed the runs had not happened. They have.
**Spec refs:** `orgintel` §1.2 (falsification), `cross-layer-contract`
§3.1 / §16, `company-runtime` §4 (Runtime Bridge), `evaluation-dogfood` §21,
`owner-cockpit` §12.2, `ARCHITECTURE.md` §16.4 (platform work originates from friction)

---

## Outcome

> **Answer, with evidence from a real run, whether an Exec-led organisation beats one strong agent on
> the same mission — and establish the cross-layer seams while they are still cheap to establish.**

Sprint 01 settled the substrate and the ontology. Three materially different companies ran the same
wake loop, commitments, messages and effect surface, and no company-specific vocabulary entered the
schema. The negative claim held: no universal command enum, no ledger, no custody, no workflow engine.

It also produced one finding that reframes everything after it.

### The finding sprint 02 exists to act on

**No company ever delegated.** `spawn_requests: []` in every wake of all three companies, including
Cosmon's largest (124 tool calls, 12 species, 6 branching evolutions). Every artifact sprint 01
produced came from **one agent with durable files** — configuration **A** of
`orgintel` §1.2, not configuration C.

So the sprint that was meant to test OrgIntel accidentally tested the baseline, and **the baseline
did well**: a playable 3D creature-collector, a working sales loop, an operating loop with 90 effect
receipts, for $10.67 across 8 turns.

The cause is now understood and is *not* model reluctance. A forced-delegation run showed the Exec
decomposing the work correctly — a groundwork commit, disjoint file ownership, optional-chained hook
call-sites so unimplemented branches are no-ops, a shared-state contract — and then never dispatching,
because **spawning staff is the only company capability that is not a tool call**. Everything else is
a tool the Exec reaches for mid-turn; `spawn` is a field in a JSON envelope emitted at the end. An
agent working naturally writes files, so it wrote briefs.

That is a plumbing gap standing in front of the product's central hypothesis. Sprint 02 removes it,
then tests the hypothesis properly.

---

## Two tracks, one of which is the point

**Track A — the thesis.** Make the comparison possible, run it, act on the result.
**Track B — the seams.** Structural work whose cost is decided by *when* you do it, not by evidence.

Track B is included despite the "observe before modelling" rule because of a distinction sprint 01
paid to learn. The sprint-01 spec justified holding the provider key host-side like this:

> *Near-zero cost now; miserable to retrofit once three company images assume ambient environment
> variables.*

We violated that in sprint 01 — the key now enters the container via `docker exec -e` — and getting
it back is exactly the retrofit that sentence warned about. **Boundaries are cheap early and
expensive late; features are the opposite.** Track B is boundaries only.

---

## Acceptance criteria

Headless with stated inputs and observed output (CLAUDE.md → "Verifying"). Nothing is described as
working until it has been run.

### Track A

1. **Delegation happens.** On one Cosmon run the Exec dispatches at least two staff, each gets a
   worktree and a supervised process, and their work merges. `spawn_requests` is non-empty and the
   OrgIntel actor rows exist.
2. **A comparison run completes** in all three organisation modes (`single_agent`, `minimal_team`,
   `orgintel`) on one versioned scenario, with identical starting runtime, model, and budget
   envelope (`evaluation-dogfood` §21.1, §25 rule 3: *baselines must receive credible
   tools, models, budgets and time*).
3. **A run report compares them** on the primary metric — accepted output per unit of owner
   attention, cost and time — with manual acceptance recorded, not an automatic score (§21.2).
4. **The result is acted on.** If C does not beat A, that is the finding and it is written down as
   such. §25 rule 10: *preserve failures honestly; do not reinterpret every run as success.*

### Track B

5. **One shared-semantics package exists** and both services import it. Identifiers, statuses and
   envelope types match `cross-layer-contract` §3.1's ownership table.
6. **A Runtime Bridge runs inside the sandbox**, holds an outbound connection, launches ACP
   processes, and owns their process trees. `docker exec` is no longer how agents are started.
7. **No provider credential is present in any company container** — verified by grepping container
   env and filesystem, restoring the guarantee sprint 01 lost.
8. **A party-level double effect is refused or flagged**: charging the same party twice for the same
   thing under two different idempotency keys is detected.

---

## Tickets

Each names its layer, the observed failure it serves (§16.7), and what it makes deletable.

| ✓ | Ticket | Layer | Evidence | Depends |
|---|---|---|---|---|
| [~] | **S02-T1 · Shared semantics package** | Cross-layer | SPA speaks `mission/ops/market`, cockpit spec speaks `attention/work/authority`, daemon speaks `commitments/effects/wakes` — three vocabularies, one system | — · docs done 94e673d; code half open |
| [x] | **S02-T2 · `restless spawn` as a tool** | OrgIntel | Forced-delegation run: correct decomposition, zero dispatch | — · a66f689 |
| [x] | **S02-T3 · Evaluation harness, three org modes** | Cross-cutting | §1.2 falsification has never run | T1 · ddfb545, 2b77eda |
| [~] | **S02-T4 · The A/B/C comparison run** | All | Sprint 01 measured the baseline by accident | T2, T3 · running |
| [ ] | **S02-T5 · Runtime Bridge** | Runtime | F7 identity trusted as-sent; leaked Chromium at 908% CPU; credential regression | T1 |
| [ ] | **S02-T6 · Split `restlessd` along the plane seam** | Kernel / OrgIntel | Nine spec components, two planes, one process; F12 — one company's hung Docker took down all three | T1 |
| [x] | **S02-T7 · Party-level reconciliation** | Kernel | Greg charged twice under two keys; idempotency guards requests, not decisions | — · 3343b18 |
| [x] | **S02-T8 · Organisational health signals** | OrgIntel | Every health signal we have is substrate-level; we can say the disk is full, not that the company is stuck | — · 206c0b2 |
| [ ] | **S02-T9 · Attention queue (minimal)** | Owner surface | Aris blocked on the owner and the entire surface was a JSON blob in a terminal | T1 |

**Status (2026-08-14):** T2, T3, T7 and T8 have landed, plus the gateway purge
(2,999 lines). T4 — the comparison the sprint exists for — is executing. T5 (Runtime
Bridge) and T6 (plane split) are carried; they are boundary work whose cost is set by
when it happens, not by this sprint's deadline, and doing them before T4 reports would
have inverted the sprint's own priority.

**If only three tickets land, they are T2, T3 and T4.** That is the sprint's job; everything else is
support. A sprint that ships five pieces of infrastructure and does not answer the thesis question
has failed at the thing it was for.

### Notes per ticket

**T1** implements `cross-layer-contract` §16 step 1 — "shared IDs and common envelope types in a
small versioned package" — *not* a schema registry or service bus (§16's closing line, and §17's
exclusions). The document hygiene half of this is **done**: the six specs now live under
`docs/specs/` without versions in their filenames, name their parent correctly, and are routed from
`CLAUDE.md`. What remains is the code half — making the daemon's identifiers and statuses match
§3.1's ownership table.

**T2** makes delegation reachable at the moment the Exec decides to delegate. Also add a warning when
`parse_termination` drops a malformed `spawn` entry — silently discarding intent is how we spent
three runs believing the Exec did not want to delegate. **Deletes:** the envelope's `spawn` field,
which is the closest thing in the codebase to a universal-command smell.

**T5** is one change that fixes four things: identity (the bridge authenticates once as its own
company, dissolving F7 rather than mitigating it), process ownership (a process group instead of
diffing PIDs from outside), the `docker exec` dependency, and the credential regression. Communication
is **outbound** from the bridge (`company-runtime` §4.4). **Deletes:** the PID-diff reaper,
the inbound TCP identity convention.

**T6** creates the place the Authority Kernel will eventually live without building it. The kernel
proper stays deferred — the sprint-01 posture ("no governance this sprint") has a named expiry that
has **not** fired: every provider is still simulated.

---

## What we are trying to learn

- Does an Exec-led team beat one strong agent on the same mission, on the same budget?
- If it does, on which dimension — output quality, owner attention, elapsed time, or recovery?
- If it does not, is that because teams are wrong for work this size, or because our delegation
  machinery is still too costly to use?
- What does a company actually do with staff once dispatch is cheap — parallel work, review,
  recovery, or none of them?
- Do organisational health signals fire on anything real, or is the company simply never stuck?

---

## Risk register

Every risk named, one disposition each. Default accepted.

| Risk | Disposition | Why |
|---|---|---|
| Provider credential inside the company container | **Pending fix** | Regressed in sprint 01. T5 restores it. Expires before any live provider. |
| The comparison is unfair to the baseline | **Guarded** | §25 rule 3 — identical model, tools, budget and time. The baseline is the incumbent and must be given its best shot. |
| Delegation makes outcomes *worse* | **Accepted** | That is a finding, not a failure. It is the thing being measured. |
| One scenario is not enough to conclude | **Accepted** | It is enough to *proceed or stop*. §2.4 — practical evidence, not laboratory theatre. |
| Splitting the daemon destabilises working runs | **Guarded** | T6 lands after T4, so the thesis answer is not blocked by a refactor. |
| Three companies concurrently on one provider key | **Accepted** | Ran successfully once the reaper landed; revisit if starvation reappears. |
| No governance on effects | **Accepted** | Unchanged from sprint 01, same expiry: the first live provider. |

---

## Explicitly out of scope

Deferred because no run has demanded them, per `orgintel` §11.1 and `CLAUDE.md`
("observe before modelling"):

- hypothesis/experiment tables — Cosmon shipped branching evolutions without ever branching a hypothesis;
- the teamwork-pattern library (`orgintel` §6.3) — earn it from T4's run, do not seed it;
- actor identity packages (§5.1) — the journal is doing this job well enough to have caught a
  predecessor lying;
- the Authority Kernel proper (`authority_plane` §22 steps 1–3) — deferred with a live trigger;
- the cockpit's Work / People / Authority screens — Attention first (`owner-cockpit` §18 step 3);
- exploration machinery, the epistemic ontology, restore/snapshot reconciliation.

---

## Housekeeping carried from sprint 01

- `scratch/**/codex-home/` lost its ignore rule in the sprint-01 merge; the directory is now untracked
  and should be re-ignored or deleted with the rest of the codex-era scratch.
- The `restless-model-gateway` proxy path (~2,500 of 2,950 lines) is dead once metering moved to the
  ACP layer. `spend.rs` is load-bearing and stays. Delete the rest under T6.
