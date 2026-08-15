# Sprint 04 — Real custody, and a team worth having

**Status:** Run complete through the governed push. T4 and T8 are waiting on the owner to open and
merge the prepared PR, then on observation of the repository's existing production deployment.
**Date:** 15 August 2026 (rev 2)
**Spec refs:** `orgintel` §2.1 / §5.2 / §6.3 / §3.4, `authority-plane` §2.2 / §6.4 / §8.2 / §8.3,
`cross-layer-contract` §3.1 / §4.5, `company-runtime` §5.4, `ARCHITECTURE.md` §16.1 / §16.6 / §16.10

---

## Outcome

> **Aris owns a real production codebase — it reads it, changes it, verifies its own changes, and
> ships them through a governed push that the owner merges. And it does that work as a team of
> differentiated specialists, because by sprint 04 the work demonstrably does not fit in one head.**

> **Success contract:** a change **Aris authored**, verified against the repo's own test suite,
> reaches `github.com/BlueprintLabIO/study` as a pull request the owner merges — and **at least one
> specialist other than the Exec did part of the work**, with the owner able to say which part and
> what it cost.

### Who does what, and this is not negotiable

**Aris's Exec and Staff implement, critique, verify, ship and deploy the Study change.** Not the
founders, not a coding agent operating the repo directly. A human-authored diff pushed through the
rail would satisfy every acceptance criterion below and prove nothing — the whole claim of this sprint
is that *the company* does the work.

Founder work in this sprint is the rail only: the effect surface, the authority gate, the verification
capability, and the owner's reads. If the rail is finished and Aris cannot produce a shippable change,
**that is the sprint's finding**, and it is a more valuable one than a green checklist.

### Why these two gaps, and why together

Sprint 03 proved the rail: a real provider, a real inbox, a governed front door, for $0.79. What it
also proved is that Restless is not yet *doing work* — it is sending mail about work.

**Gap 1 — the company cannot ship.** Aris holds a 140MB SvelteKit product at `/company/repos/study`
and can commit to it locally, which is custody, not ownership. It cannot push, and it must not be
given a git credential: that is exactly the regression S03-T4 closed for email.

**Gap 2 — there is no team, and there never has been.** Zero organic delegation across three companies
and three sprints, because `staff.rs` gave every staff member the Exec's own model, the same broad
context and the label `"staff"`. **Delegating meant handing work to an undifferentiated copy.** A
rational Exec declines, and has, every time. S04-T5 has since landed durable roles, model attribution
and deliberately narrow briefs; whether those change the arithmetic is one of this sprint's open
questions.

**Runtime constraint — Aris is Kimi-only.** The owner has authorised and configured
`moonshot/kimi-k3`; no second provider credential exists. This is not a gap to route around. Sprint 4
tests critic independence through a distinct adversarial role and withheld drafting context. It does
not substitute a model or provider merely to satisfy a diversity-shaped metric.

They are the same gap because both are about the company being *able to act on something real*. A
codebase it cannot ship is theatre; a team of clones is a bigger context window with extra steps.

---

## What has already landed

Verified this session with stated inputs and observed output. Not claims.

| Ticket | Evidence |
|---|---|
| **S04-T10 · A principal on the wire** | Every request carries `owner` or `company/exec`; `dispatch()` gates on the principal, not the listener. Verified over a live socket: a missing principal, an unknown principal, and `company/exec` attempting `approve` are all refused with `error.kind = "authority"`, and the party did not reach company config. Ordinary agent coordination over TCP still works. 4 focused tests |
| **S04-T1 · Test/live split** | `restless up -c aris_test --from aris` clones mission and config with providers, credentials, standing approvals and `from_address` stripped, and copies the simulator personas. `restless down --destroy` removes container, volume, schema, spend spool, personas and config. All five S03-T7 acceptance criteria observed live, including create → run → destroy → recreate under the same name in one session |
| **Purge** | `OrgMode` deleted entirely — enum, config field, two dead branches in `staff.rs`, three test literals, and the stale key in every company file. `is_performed_by_daemon` deleted. 38 tests pass, 0 warnings |

**One real bug fixed along the way, and it explains a sprint-03 finding.** `call_world_model` built
its auth with `agent_auth` but dropped the `<PROVIDER>_BASE_URL` override that `exec.rs` forwards. A
Kimi For Coding key authenticates against `api.kimi.com/coding/v1` and 401s against the provider
default — probed, both hosts, 200 versus 401. So every *simulated* effect failed with "Invalid
Authentication" while the Exec worked fine. Sprint 03 recorded the world model 401ing twice and read
it as a dead key. It was two code paths building the same auth and one honouring it.

---

## The correction this rewrite exists to make

Rev 1 of this spec said, in T3: *"The daemon holds the GitHub credential and performs the push"*, and
its AC2 required a receipt with `provider: "github"`.

**That mandates a provider catalogue**, which the working model forbids outright, and which
`provider.rs`'s own doc comment argues against harder than the rule does — *"an adapter per provider
does not scale… We built the adapter first anyway, which got email working and got the architecture
backwards."* Implementing T3 as written produced `Provider::Github`, catalogue entry #2, sitting
directly beneath that comment.

The tension looked irreducible: an adapter violates *no catalogue*; self-reporting the push puts a git
credential in the container, which the risk register marks **Invariant**.

**It is not irreducible. Both horns come from assuming the company must open the pull request.**

### `repo.push` is a git push. The owner opens the PR.

Split what is actually two things:

| | Who | What it needs |
|---|---|---|
| **Push a branch to its own origin** | the kernel, host-side | a git credential — **generic**, not a vendor API |
| **Open a pull request** | the owner | a link and a click |

A git push over HTTPS is a *protocol*, not a provider. The same code path serves GitHub, GitLab, Gitea
and a bare remote on a VPS, because it never learns their APIs. There is no catalogue to grow: adding
a forge adds nothing.

The receipt carries the branch, the commit SHA, the remote, and a **ready-to-click compare URL** built
by string template from the remote — no API call, no second credential, no vendor client. The owner
clicks it, sees the diff, opens and merges the PR.

This is better on three counts beyond rule 7:

1. **It is the prepared last mile.** CLAUDE.md: *"preserve the prepared state and bring the exact
   browser session, link, or bounded confirmation to the CEO — never hand the surrounding workflow
   back as instructions."* A compare URL is precisely that. A PR opened by a bot and merged by a human
   is a weaker version of the same act.
2. **The authority boundary gets sharper, not softer.** T4's claim was that merging is the human
   authority act. Under rev 1 the company opened the PR and the owner merged; now the owner opens
   *and* merges. Nothing about the company's reach into GitHub needs to be trusted.
3. **The credential never enters the container** — the Invariant holds, unchanged. The branch leaves
   as a `git bundle`; the host pushes it.

**Cost, named honestly:** the owner does one extra click, and the PR body is theirs to write rather
than Aris's. Accepted. If that click becomes friction across several sprints, the Agent Proxy
(`authority-plane §8.3`) is the right answer, not an adapter.

**Consequence for code already written:** `Provider::Github`, `github_push`'s PR-opening half, and its
two GitHub API helpers are **reverted**. The bundle-out-and-push half survives as the generic path.

---

## Acceptance criteria

Headless, with stated inputs and observed output. Nothing is green until it has run.

**Evidence ranks:** provider/external evidence outranks executable evidence, which outranks review,
artifacts and agent narrative. A criterion satisfied only by an agent saying it happened is not
satisfied.

1. **Aris verifies before it commits.** It installs the repo's dependencies and runs the repo's own
   suite (`bun test:unit`, `bun check`), and reports the result *as observed output*, not as a claim.
   A red suite it did not cause is a finding it records, not a blocker it invents.
2. **A change ships as a pushed branch with a receipt.** `repo.push` produces a receipt carrying the
   branch, the commit SHA, the remote, and a compare URL. **Provider evidence:** the branch is visible
   on `github.com/BlueprintLabIO/study` — confirmed against GitHub, not against our own record.
3. **No git credential exists inside the container.** Exact-secret grep over container environment and
   the whole `/company` volume returns zero, exactly as S03-T4 verified for Resend.
4. **A repeated push with the same idempotency key does not push twice** — it replays the stored
   receipt (`cross-layer §4.5`).
5. **At least two actors with different roles did the work using the configured Kimi model**, visible
   as durable actor rows and as cost reported per role. The critic receives the artifact and
   acceptance criteria without the producer's drafting context; role and context provide the
   independent angle in this Kimi-only runtime.
6. **The critic changed something.** A producer–critic pass on the change produces at least one
   specific objection that alters it, with the diff observable. A critic that only agrees is a
   **failed demonstration and a recorded finding**, not a pass.
7. **The owner can tell who did what.** For the shipped change: which role, which model, what it cost,
   and what it produced — answerable from OrgIntel without reading a log.
8. **An agent cannot grant itself authority.** ✅ *Landed (T10)* — `restless approve` from inside the
   container is refused with `error.kind = "authority"` and the party does not reach company config.
9. **The owner follows the run without `psql`.** AC7's four questions are answered from `restless`
   alone. The *complete* control surface — company creation, credential verbs, the coverage test — is
   sprint 05, so it is designed against a real shipped change rather than guessed at.
10. **The change is Aris's.** The PR diff is authored by Aris's Exec and Staff. Git history and the
    OrgIntel record agree on who wrote what. A founder-authored diff fails this criterion outright.
11. **The owner operates the Company Runtime through `restless`, not raw Docker commands.** Runtime
    inspection uses the generic `restless attach -- <command>` door; image/version skew is diagnosed
    with `restless doctor` and repaired with `restless up --reconcile`, preserving the company volume.
    Direct Docker remains the Runtime Bridge implementation and an allowed out-of-band verification
    tool, but it does not appear in the normal owner operating transcript.

---

## Tickets

| ✓ | Ticket | Layer | Evidence (observed friction) | Depends |
|---|---|---|---|---|
| [x] | **S04-T1 · Test/live split** (carried S03-T7) | Runtime / OrgIntel | We contaminated a live company's beliefs with a synthetic webhook because `aris` was where the real provider was | — |
| [x] | **S04-T10 · A principal on the wire** | Authority | `restless approve` — the human authority act — was callable from inside the container. `main.rs:215` accepted this with expiry *"before any real external effect"*; S03 sent real email | — |
| [x] | **S04-T5 · Roles, models and persistent actors** (carried S03-T9) | OrgIntel | Zero organic delegation in three sprints, because staff were clones | — |
| [x] | **S04-T11 · CLI-first runtime reconciliation** | Runtime / Owner surface | The Sprint 4 dogfood needed raw Docker to discover an outdated in-container CLI, replace it, inspect two actor processes and pause one so the other survived. Live re-run: `doctor` detected the stale image; `up --reconcile` rebuilt/replaced it while preserving the volume and Study commit; the in-container CLI matched; a reconcile during a Kimi Exec wake was refused with a typed conflict. Zero raw Docker in the operating transcript | T10 |
| [x] | **S04-T2 · Verify-before-commit** | Runtime | Aris observed 39/39 unit files and 1680 passing tests, `svelte-check` with 0 errors/0 warnings, a green production build and four HTTP smoke probes before commit `289f04c`; after the durable critic changed the diff, Exec reran 1680/1680 tests, 0/0 `svelte-check`, the production build and served-HTML probes before final commit `4eb3345` | T1, T11 |
| [x] | **S04-T3 · `repo.push` as a generic governed effect** | Authority (effect service) | Generic Git receipt `cc64cb0d-4ba1-4283-a010-44ffe157a26b` pushed `feat/tutoring-centre-offer` at final commit `4eb3345`; replay returned the same receipt, external `ls-remote` observed the exact SHA, and exact-secret scans found zero git credential occurrences in runtime env and `/company` | T2, T10 |
| [ ] | **S04-T4 · The owner opens and merges** | Owner surface | Merging is the human authority act. Under the corrected T3 the owner opens the PR too, from the receipt's compare URL. **No approvals table, no lifecycle, no forge API** | T3 |
| [x] | **S04-T6 · Producer–critic on the change** | OrgIntel | A real durable `critic` actor received repository/runnable-path context but no producer reasoning. Its `SHIP-AFTER-FIXES` report found an inaccurate free-tier worked-solutions promise and two merge-worthy inconsistencies; Exec changed four files and pushed corrective commit `4eb3345` | T5 |
| [x] | **S04-T9 · Owner reads: receipts, people, spend** | Owner surface | `restless people`, `receipts` and `spend` identify Exec and critic roles, configured Kimi models, per-actor cost, the final Git outcome and one stored receipt despite idempotency replay. No `psql` was used to operate or assess the run | T5 |
| [ ] | **S04-T8 · The run + report** | All | The success contract is one shipped change authored by a team. Everything above is machinery until that happens | T2, T3, T6, T9 |

**S04-T7 (real `web.deploy`) is cut from this sprint.** It is the same catalogue problem wearing a
different hat: a real deploy provider is adapter #3. The landing page reaches production when the
owner merges the PR, which is a deploy the repository already knows how to do. If a preview URL turns
out to be load-bearing for the critic's judgement, it returns as its own ticket with the Agent Proxy
question answered first.

**If only three land, they are T2, T3 and T6** — verify, ship, and a second mind that changed
something. T9 is below that line but is not new scope; AC5 and AC7 already demand it.

### Notes per ticket

**T2** is the cheapest ticket and the one that decides whether ownership is real. `node_modules` is
present (79 entries) and `bun 1.3.14`, `node 24.19.0` and `git 2.39.5` are in the image; `gh` is not,
and under the corrected T3 it does not need to be. The scripts exist: `test:unit`, `check`, `build`.
**The rule that matters: a red suite Aris did not cause is a finding, not a blocker.** Sprint 02 nearly
lost a day to a font-load timeout under CPU contention that was read as a regression.

**T11** is the dogfood correction the owner surfaced mid-run. Docker stays as the mature V0 Runtime
Bridge mechanism (`company-runtime` §2.1); it stops being a command the owner must know. `attach`
accepts an arbitrary ordinary command instead of growing `restless ps`, `restless files`, and one API
per Linux primitive. `doctor` reports version skew honestly, including `unknown`; `up --reconcile`
rebuilds the source-labelled image and replaces only the container, preserving the volume. A live
supervised actor makes reconciliation refuse until the owner explicitly lets it finish or stops the
runtime. **Deletes:** the manual `docker build` / `docker exec` repair transcript as normal operation.

**T3** is the sprint's structural piece, re-shaped. The company's path is unchanged —
`restless effect repo.push --args '{"repo":"study","branch":"…"}'`. The daemon bundles the branch out
of the container, pushes it from the host with a credential that never crosses the boundary, and
writes a receipt carrying branch, SHA, remote and compare URL. No forge API, no vendor client, no new
`Provider` variant beyond a generic `Git`. **Deletes:** the assumption that work leaves the company
only as prose, and `Provider::Github` along with it.

**T4** invents nothing. The compare URL is a string built from the remote; the owner's browser does
the rest. This is the ticket that got *smaller* by taking the working model seriously.

**T6** is the demonstration with teeth. Producer drafts, critic objects with no access to the drafting
context, producer revises. §6.3's stated best fit is "hidden errors, subjective quality,
external-facing output". A yes-man critic is a recorded failure, not a pass.

**T9** is the read half of a CLI that has only ever grown a write half. The source change is small and
load-bearing: the spend ledger gains an actor, and the role is joined from OrgIntel at read time
rather than copied, so OrgIntel stays the single writer of role.

---

## Risk register

| Risk | Disposition | Why |
|---|---|---|
| **Aris breaks a real production repo** | **Guarded** | It never writes to the default branch — refused in the effect before anything is pushed. T2 gates non-trivial commits behind a green suite. The owner opens and merges. Worst case is an abandoned branch |
| A git credential leaks into the runtime | **Invariant** | The daemon pushes; the branch leaves as a bundle. AC3 grep-verifies zero occurrences on the volume and in the container environment. Reopening this is the one thing this sprint must not do |
| **A provider catalogue grows** | **Invariant** (new) | Rev 1 violated this and produced `Provider::Github`. The corrected T3 adds a generic `Git` transport and T7 is cut. A future ticket proposing a third vendor adapter must first answer why `authority-plane §8.3`'s Agent Proxy is not the answer |
| `repo.push` pushes twice on retry | **Guarded** | AC4: idempotency key replays the stored receipt |
| Role-specific turns blow the budget | **Guarded** | The company ceiling is unchanged and is the real fuse; producer and critic both use the configured Kimi model. AC7 reports cost per role |
| **The critic is a yes-man** | **Accepted (finding)** | AC6 makes it a recorded failure rather than a silent pass |
| **Delegation still does not happen** | **Accepted (finding)** | T5 has landed durable roles, model attribution and narrow briefs. If the Exec declines *even now*, with a codebase too large to hold, that is a strong result about the product thesis and it reshapes sprint 05 |
| **A founder writes the change instead of Aris** | **Invariant** (new) | AC10. It is the easiest way to get a green sprint and a worthless one. Git history and the OrgIntel record must agree that Aris wrote it |
| Runtime reconciliation kills useful live work | **Guarded** | `up --reconcile` refuses while the daemon supervises an Exec or Staff session. The owner either waits or uses the existing explicit `down`; the volume and Git work survive replacement |
| A timeout is mistaken for a durable checkpoint | **Guarded** | The prospect run proved browser/tool evidence is ephemeral until written to `/company`. The runtime now says only files already written are preserved; bounded research tasks checkpoint after each candidate. No workflow engine is introduced |
| We contaminate a live company again | **Closed** | T1 landed. `aris_test` resolves every provider to the simulator structurally, verified by test and by live run |
| The daemon's credential posture is weaker than the backend it plans to adopt | **Accepted (recorded)** | `credential.rs:60` resolves `env:` from the daemon's environment, so keys live in whatever shell starts `restlessd`. Infisical's CLI uses the OS keyring. Not this sprint's fix; recorded so §8's adoption is not mistaken for a no-op |
| The repo is too large to work in usefully | **Accepted** | 140MB, unknown to the Exec. T2 is partly a probe of this |

---

## What we are trying to learn

- Does a company given a real codebase, real verification and a real way to ship **actually ship**, or
  does it stall on something we cannot see from here?
- **Does an Exec delegate once delegation is worth it?** Three sprints say it will not delegate to
  clones. T5 changed the arithmetic; this is the first honest test.
- Does producer–critic improve the output measurably, or is it two model calls where one would do?
- Is a receipt the right record for a push, or does code want something a receipt cannot express
  (review state, CI status, merge conflicts)?
- **Where does the owner still reach for `psql`?** Record every instance. That list is sprint 05's
  input, and it is worth more than a surface designed before the work existed to observe.
- Does the owner mind opening the PR themselves? If the extra click is invisible, the Agent Proxy can
  stay deferred indefinitely.

## Explicitly out of scope

- **The Authority Kernel proper** — capability grants, policy language, the `§5`/`§6` engine.
- **The Infisical Agent Proxy** — named as the right long-term answer to the catalogue problem, and
  deferred with a live trigger (a third vendor integration wanting an adapter).
- **CI in the company runtime** — Aris runs the repo's suite locally.
- **Real `web.deploy`** — cut, see above.
- **More than two specialists at once** — the cap stays two.
- **The owner SPA** — S03-T8's wire contract stays carried.

## Carried

- **S03-T8 (owner wire contract)** — partially discharged: `error` is now `{kind, message}` and the
  authority refusal is typed (T10). Still carried: `BlockKind` flattening on the other paths, writes
  returning state instead of `{accepted, status}`, and idempotency on §4.5's five classes.
- **The complete control surface and the attention queue** — sprint 05 (`S05-T1`, `S05-T2`).
- **The reply leg** — blocked on one owner MX record for `reply.blueprintlab.io`.
- **The owner's CEM verdict is answered but not closed** — v2 exists as a draft.
