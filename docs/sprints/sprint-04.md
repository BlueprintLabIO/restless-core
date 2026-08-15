# Sprint 04 — Real custody, and a team worth having

**Status:** Draft for founder alignment
**Date:** 15 August 2026
**Spec refs:** `orgintel` §2.1 / §5.2 / §6.3 / §3.4, `authority-plane` §2.2 / §6.4 / §8.2,
`cross-layer-contract` §3.1 / §4.5, `company-runtime` §5.4, `ARCHITECTURE.md` §16.1 / §16.6

---

## Outcome

> **Aris owns a real production codebase — it reads it, changes it, verifies its own changes, and
> ships them through a governed push that the owner merges. And it does that work as a team of
> differentiated specialists, because by sprint 04 the work demonstrably does not fit in one head.**

> **Success contract:** a change Aris authored, verified against the repo's own test suite, reaches
> `github.com/BlueprintLabIO/study` as a pull request with a receipt — and **at least one specialist
> other than the Exec did part of the work**, with the owner able to say which part and what it cost.

### Why these two, and why together

Sprint 03 proved the rail: a real provider, a real inbox, a governed front door, for $0.79. What it
also proved is that Restless is not yet *doing work* — it is sending mail about work. Two gaps stand
between here and a harness that does real work, and they turn out to be the same gap seen twice.

**Gap 1 — the company cannot ship.** Aris now holds a 140MB SvelteKit product at
`/company/repos/study` and can commit to it locally, which is custody, not ownership. It cannot push,
and it must not be given a git credential: that is exactly the regression S03-T4 closed for email, and
it would reopen within a week.

**Gap 2 — there is no team, and there never has been.** Not "it declined this time":

```
aris       exec, owner, world      ← no staff, ever
cosmon     (none)
thymelake  exec, owner             ← no staff, ever
```

Zero organic delegation across three companies and three sprints. The one staff process that ever ran
was a probe we triggered. The cause is not reluctance, it is arithmetic — `staff.rs:182` gives every
staff member the Exec's own model, `staff.rs:166` labels them all `"staff"`, and `context.rs` has zero
`org_mode` branches. **Delegating means handing work to a copy of yourself with less context.** A
rational Exec declines, and has, every time.

They are the same gap because both are about the company being *able to act on something real*. A
codebase it cannot ship is theatre; a team of clones is a bigger context window with extra steps.

---

## The evidence that these are now earned, not speculative

`ARCHITECTURE.md §16.1` says grow machinery only after repeated real scenarios reveal the same need.
Both tickets clear that bar on sprint-03 evidence rather than on principle.

**Specialisation earned it in one exchange.** The owner's verdict on the CEM sample was that the
artifact was strong and the packaging failed — *"what is a CEM? If I have to ask, a parent has to
ask."* Writing a correct 11+ question and writing an email a parent opens on a phone are visibly
different jobs, done well and badly by the same actor in the same wake. Add the codebase and there is
a third: someone who knows `exercise-engine` and `anvil`.

**The rewrite proves the loop but not the shape.** Told plainly, Aris cut 17,240 chars to 4,898,
dropped the acronym, and led with the parent. It also noticed, unprompted, that it had never persisted
the outbound body and so could not review its own send against the verdict — and fixed that. The
correction loop works. What it does not show is that one actor should hold both jobs.

**Context is now a hard constraint, not an aesthetic one.** A 140MB product will not fit alongside a
sales loop, a hypothesis file and an effect ledger. This is the first time the argument for a team is
*this work does not fit in one head* rather than *teams are good*, and that is the only kind of
argument that should create machinery here.

---

## Acceptance criteria

Headless, with stated inputs and observed output. Nothing is green until it has run.

1. **Aris verifies before it commits.** It installs the repo's dependencies and runs the repo's own
   suite (`vitest`, `svelte-check`), and reports the result *as observed output*, not as a claim. A
   red suite it did not cause is a finding it records, not a blocker it invents.
2. **A change ships as a pull request with a receipt.** `repo.push` produces a receipt with
   `provider: "github"`, a PR URL in the outcome, and the branch name. The owner merges; the company
   never writes to `main`.
3. **No git credential exists inside the container.** Exact-secret grep over container env and the
   whole `/company` volume returns zero, exactly as S03-T4 verified for Resend.
4. **A repeated push with the same idempotency key does not open a second PR** — it replays the
   stored receipt (`cross-layer §4.5` lists consequential external effects as requiring stable
   identity).
5. **At least two actors with different roles and different models did the work**, visible as actor
   rows whose `kind` is not `"staff"`, and as cost reported per role.
6. **The critic changed something.** A producer–critic pass on parent-facing copy produces at least
   one specific objection that alters the artifact, with the diff observable. A critic that only
   agrees is a **failed demonstration and a recorded finding**, not a pass.
7. **The owner can tell who did what.** For the shipped change: which role, which model, what it cost,
   and what it produced — answerable from OrgIntel without reading a log.

---

## Tickets

| ✓ | Ticket | Layer | Evidence (observed friction) | Depends |
|---|---|---|---|---|
| [ ] | **S04-T1 · Test/live split** (carried S03-T7) | Runtime / OrgIntel | We contaminated a live company's beliefs with a synthetic webhook because `aris` was where the real provider was. `_test` companies are the fix for an incident, not a convenience | — |
| [ ] | **S04-T2 · Verify-before-commit** | Runtime | Aris holds a 140MB product with `node_modules` empty. Any change it makes today is unverified, and it spent three wakes reasoning about a "404 landing page" that came from a *simulated* `web.deploy` | T1 |
| [ ] | **S04-T3 · `repo.push` as a governed effect** | Authority (effect service) | The company can commit and cannot ship. A token in the container reopens the S03-T4 regression; push is a consequential external action with a party and an outcome, so it belongs on the effect surface | T2 |
| [ ] | **S04-T4 · PR-as-approval** | Authority / Owner surface | S03-T5 proved per-party approval for email. Code needs the same boundary and already has the native form: the company opens a PR, the owner merges. No policy language (`§6.5`) | T3 |
| [ ] | **S04-T5 · Roles, models and persistent actors** (carried S03-T9) | OrgIntel | Zero organic delegation in three sprints, because staff are clones. `§6.3` teamwork patterns is Core contract and item 12 of the V0 acceptance list; none of the five exist | T1 |
| [ ] | **S04-T6 · Producer–critic on parent-facing copy** | OrgIntel | The owner's verdict split one wake's output into a strong artifact and failed packaging — two jobs, one actor, one of them done badly | T5 |
| [ ] | **S04-T7 · Real `web.deploy`** | Authority (effect service) | The landing page lives in the repo now. `web.deploy` is still simulated and its world model 401'd twice this sprint, so the company has been reasoning about a page that does not exist | T3 |
| [ ] | **S04-T8 · The run + report** | All | The success contract is one shipped PR authored by a team. Everything above is machinery until that happens | T1–T7 |

**If only three land, they are T2, T3 and T5** — verify, ship, and a second kind of worker. That is the
irreducible claim: *the company does real work on a real thing, and more than one mind does it.*

### Notes per ticket

**T1** goes first, and this is not negotiable in the way it was last sprint. We injected a synthetic
reply into a live company and it became "the strongest single demand signal so far" in a real
hypothesis. Nothing touches a live company until throwaways exist. `--destroy` must clear the spend
spool as well as container, volume and schema — the sprint-02 comparison died of exactly that gap.

**T2** is the cheapest ticket and the one that decides whether ownership is real. `bun install`, then
the repo's own `vitest` and `svelte-check`. It is also how Aris discovers what is actually in the
repo versus what it has been assuming for three wakes. **The rule that matters: a red suite Aris did
not cause is a finding, not a blocker.** Sprint 02 nearly lost a day to a font-load timeout under CPU
contention that was read as a regression.

**T3** is the sprint's structural piece. The daemon holds the GitHub credential and performs the push;
the company's path is `restless effect repo.push --args '{"repo":"study","branch":"…","title":"…"}'`,
identical in shape to `email.send`. The receipt carries the PR URL. **Deletes:** the assumption that
work leaves the company only as prose.

**T4** reuses T5's boundary rather than inventing one. A PR *is* an approval request with a native UI
the owner already knows, and merging is the human authority act. No approvals table, no lifecycle.

**T5** is S03-T9 with the scope it should always have had: `SpawnRequest` gains an optional `model`,
`add_actor` carries the real role instead of `"staff"`, and roles are files under
`/company/org/roles/`. Actors persist across wakes (`§2.1`: durable actors, replaceable sessions) —
today they are one-task processes, which is why nothing accumulates. **Deletes:** `OrgMode` entirely.
Modes were three names for one configuration; patterns replace them.

**T6** is the demonstration with teeth. Producer drafts, critic objects with no access to the drafting
context, producer revises. §6.3's stated best fit is "hidden errors, subjective quality,
external-facing output" — the CEM email exactly. A yes-man critic is a recorded failure.

**T7** closes the loop the owner opened by handing over the repo: the landing page stops being
simulated. Same dispatch pattern as S03-T1, one config entry.

---

## Risk register

| Risk | Disposition | Why |
|---|---|---|
| **Aris breaks a real production repo** | **Guarded** | It never writes to `main`; it opens PRs. T2 gates non-trivial commits behind a green suite. The owner merges. Worst case is a bad branch and a closed PR |
| A GitHub credential leaks into the runtime | **Invariant** | The daemon performs the push; AC3 grep-verifies zero occurrences on the volume. This is the S03-T4 boundary, and reopening it is the one thing this sprint must not do |
| `repo.push` double-opens PRs on retry | **Guarded** | AC4: idempotency key replays the stored receipt. `cross-layer §4.5` names consequential external effects as requiring stable identity |
| Per-role models blow the budget | **Guarded** | The company ceiling is unchanged and is the real fuse; a role names a model, it does not get a wallet. AC7 reports cost per role so "was the critic worth it" is answerable in pounds |
| **The critic is a yes-man** | **Accepted (finding)** | AC6 makes it a recorded failure rather than a silent pass. If it happens, it tells us the pattern needs different context or a different model — which is what `§3.4` improvement records are for |
| Delegation still does not happen | **Accepted (finding)** | If the Exec declines *even with* real specialists and a codebase too large to hold, that is a strong result about the product thesis and it reshapes sprint 05. Better learned here than assumed |
| We contaminate a live company again | **Pending fix** | T1. It has happened twice — the cosmon schema reset and the synthetic webhook — and both times because the live company was the convenient one |
| The repo is too large to work in usefully | **Accepted** | 140MB, unknown to the Exec. T2 is partly a probe of this. If context turns out to be the binding constraint, that is itself the argument for T5 and should be recorded as such |

---

## What we are trying to learn

- Does a company given a real codebase, real verification and a real way to ship **actually ship**, or
  does it stall on something we cannot see from here?
- **Does an Exec delegate once delegation is worth it?** Three sprints say it will not delegate to
  clones. This is the first honest test of whether the OrgIntel thesis holds when the arithmetic
  changes.
- Does producer–critic improve external-facing output measurably, or is it two model calls where one
  would do?
- Is a receipt the right record for a push, or does code want something a receipt cannot express
  (review state, CI status, merge conflicts)?
- What does the owner actually want to see about a team? AC7 asks for role, model, cost and output —
  that is a guess, and the run should correct it.

## Explicitly out of scope

- **The Authority Kernel proper** — capability grants, policy language, the `§5`/`§6` engine. Approval
  stays a typed check plus a PR.
- **CI in the company runtime** — Aris runs the repo's suite locally. Hosted CI, matrix builds and
  deploy pipelines are the repo's own concern.
- **More than two specialists at once** — the cap stays two. Prove the pattern before widening it.
- **Multi-repo custody** — one repo, one company.
- **The owner SPA** — S03-T8's wire contract stays carried; wiring `web/` is not this sprint.

## Carried

- **S03-T8 (owner wire contract)** — unbuilt. Still the cheapest-now/expensive-later item, and T5's
  roles will add owner-facing nouns that want naming before three clients hardcode them.
- **The reply leg** — blocked on one owner MX record for `reply.blueprintlab.io`. Outbound is proven;
  inbound is verified only by signed replay.
- **The owner's CEM verdict is answered but not closed** — v2 exists as a draft. Sending it is T6's
  natural first artifact.
