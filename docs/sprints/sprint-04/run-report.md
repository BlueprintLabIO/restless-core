# Sprint 04 run report — Aris takes custody of Study

**Run date:** 15 August 2026  
**State:** complete to the owner authority gate; the PR is pushed but not yet opened or merged  
**Company:** `aris`  
**Repository:** `https://github.com/BlueprintLabIO/study`

## Outcome so far

Aris's Exec authored a tutoring-centre acquisition path in the real Study repository, a durable Kimi
critic independently reviewed the runnable branch, and the critique materially changed the result.
Exec reran the repository gates and shipped the final branch through the generic governed `repo.push`
effect. The kernel returned the prepared compare URL. The remaining merge is deliberately an owner
act, so Sprint 04 is not marked complete yet.

- Branch: `feat/tutoring-centre-offer`
- Final commit: `4eb334570070c12664ea5ad810eadb4b289ca4f8`
- Compare URL: <https://github.com/BlueprintLabIO/study/compare/main...feat/tutoring-centre-offer?expand=1>
- Final push receipt: `cc64cb0d-4ba1-4283-a010-44ffe157a26b`
- External evidence: `git ls-remote` returned the exact final SHA for the branch.

A separate documentation correction found during the critic follow-up was kept off the landing branch:

- Branch: `fix/pricing-doc-rate-limits`
- Commit: `4e18414ddd6e9232fbe55591ad814a1d774f9f65`
- Compare URL: <https://github.com/BlueprintLabIO/study/compare/main...fix/pricing-doc-rate-limits?expand=1>
- Push receipt: `968b7672-987c-4b2f-9752-dbd3ed7f9ab9`; idempotency replay returned the same receipt.

## What Aris changed

The branch adds `/for-tutoring-centres`, a homepage entry point, sitemap coverage and English, Korean
and Chinese centre-facing copy. The offer is a real February 2026 sample that centres may print and
teach, followed by an ongoing paid class-set licence.

The first producer pass reached commit `289f04c`. The independent durable critic then found that the
page promised free users step-by-step maths solutions even though the repository's own free-tier
limits suppress those solutions. It also found an ambiguous sample/class licence and a centre page
that sitemap/hreflang advertised in three locales while its copy was hard-coded English. Its
13,303-byte report at `/company/outputs/sprint-04-restless-critic.md` returned
`SHIP-AFTER-FIXES`.

Exec responded in `4eb3345` by:

- stating the actual free experience: writing AI feedback and maths answer keys, with full worked
  maths solutions on paid plans;
- making the free sample explicitly printable/teachable while the licence covers ongoing class sets;
- moving the page through the existing Paraglide message system in English, Korean and Chinese;
- correcting the test-day-format and auth-wall copy, shortening the title, and linking the centre CTA
  to a prepared email.

That is the observable producer–critic change required by AC6; the critic did not merely agree.

## Verification evidence

Aris ran the Study repository's gates before the initial commit and again after the critic's changes.
The final report to the owner records:

- unit suite: 39 files, **1680/1680 passed**;
- `svelte-check`: **0 errors, 0 warnings**;
- production build: green;
- served-HTML probes: corrected claims present and refuted claims absent;
- sample PDF and centre/practice routes: HTTP 200 in the built application.

The final branch is clean in the persistent company computer. Exact-secret scans over the container
environment and all of `/company` found zero copies of the host Git credential.

The critic also observed a separate production caveat: current `aris-academy.com/booklet/*` requests
return 500 and `/api/health` returns 503 while static PDFs remain available. This is not represented as
fixed by the branch. It must be re-probed after merge before outreach describes the interactive QR
funnel as live.

## Team and cost, from the owner CLI

No database console was used. `restless people`, `restless spend` and `restless receipts` showed:

| Actor | Role | Model | Recorded cost | Produced |
|---|---|---|---:|---|
| `exec` | Exec | `moonshot/kimi-k3` | $5.7057 through the pricing-doc push wake | Study branches, verification, corrective commit and outreach packaging |
| `staff-centre-critic-live` | Critic | `moonshot/kimi-k3` | $2.9006 | Independent runnable-path report and seven evidence-backed objections |
| `staff-prospect-research-live` | Researcher | `moonshot/kimi-k3` | $2.0356 | Four verified centre contacts from seven candidates checked |

The preserved ledger also contains unattributed spend from earlier pre-fix experiments, including an
old ZAI record. It is historical evidence, not the current model configuration: the reconciled daemon
and both actors in this proof use only the owner's configured Kimi endpoint.

`restless receipts --capability repo.push` shows one row for final commit `4eb3345`. Repeating the
same idempotency key produced `effect_replayed` and returned receipt `cc64cb0d...`; it did not create a
second effect.

The final owner read reported $19.7025 accounted against the $30 company ceiling, with $10.2975
remaining. The current daemon environment and every actor in this run used `moonshot/kimi-k3` only.

## Revenue-validation batch

The first open-ended Exec research attempt made 67 browser/tool calls across timed-out turns but wrote
no candidate evidence to the company volume. On resume, the Exec correctly refused to turn ephemeral
transcript data into claims and returned zero verified contacts. That exposed two concrete findings:

1. a turn timeout preserves files, not unwritten reasoning or arbitrary tool results; and
2. prospect research must checkpoint evidence per candidate rather than write only at the end.

The runtime message that claimed *all* work was on disk was corrected. A bounded durable researcher
then created its output before browsing and updated it after each of seven candidates. It qualified
four parties and rejected three with evidence:

| Centre | Canonical party | Evidence/fit |
|---|---|---|
| BrainTree Coaching Australia | `hello@braintreecoaching.com.au` | Published contact address; Selective/OC prep and mock-test library are its core offer |
| Global Education Academy | `enquiries@globaleducationacademy.com.au` | Published new-enquiries address; Year 5–6 Selective & Scholarship course |
| Pre-Uni New College | `info@newcollege.com.au` | Head-office address verified from its branch page and structured data; 20+ NSW OC/Selective branches |
| Matrix Education | `info@matrix.edu.au` | Published contact address; dedicated NSW Selective Test course across Sydney campuses |

Talent 100 was rejected because its own site publishes no email, Dr Du because its site is HSC-only,
and North Shore Coaching College because its site returned a database fatal error and no address
could be verified. The researcher wrote
`/company/outputs/sprint-04-prospect-research-live.md`; Exec independently rechecked all four accepted
addresses and wrote four tailored drafts to `/company/outputs/sprint-04-prospect-batch.md`.

No party was approved and no email effect ran. Three deterministic gates remain red: the centre page
is 404 until merge, the production booklet/health paths are 500/503, and first contact requires an
owner grant for each exact party. If the QR path remains unhealthy at send time, the prepared drafts
remove the QR paragraph and send the verified static PDF only.

## Runtime/CLI finding and correction

The run initially exposed an outdated in-container CLI and actor cleanup that could kill a concurrent
session. S04-T11 corrected the owner path without replacing the mature OCI/Docker Runtime Bridge:

- `restless attach -- <ordinary command>` is the generic Linux door;
- `restless doctor` reports source/image skew as `current`, `required` or `unknown`;
- `restless up --reconcile` rebuilds/replaces only when needed, preserves the company volume and
  refuses while a supervised actor is active;
- ACP sessions own marker files and cleanup reaps only the matching Linux session, not a broad PID
  difference or process-name match.

Live acceptance used only Restless owner commands. `doctor` first reported `required`, reconciliation
rebuilt and replaced the stopped container, and the final `doctor` reported matching source/image
digest `1553a9184766a356f599528958fe5aca2c019a00f4b7d8c4b598b250384cdabb` as `current`.
The marker `/company/scratch/s04-t11-volume-marker.txt`, Study commit `4eb3345`, and the critic report
all survived. A reconcile attempted during an earlier live Exec wake was refused with a typed
`conflict` naming the actor.

Host verification after the final implementation: `cargo test --workspace` passed 52 tests across
the workspace (including 42 daemon tests and the concurrent-session cleanup regression),
`cargo build --workspace` succeeded, and `git diff --check` was clean.

## Remaining owner gate

1. Open and merge the centre-offer compare URL if the diff is acceptable; optionally merge the
   separate pricing-document correction from its own compare URL.
2. Let the repository's existing deployment complete.
3. Re-probe the centre page, `/booklet/26-02-1` and `/api/health`; record production evidence before
   declaring the landing/QR funnel live.
4. Grant or decline first-contact authority for the four exact centre email parties. Once the page
   gate is green, Aris can send the prepared messages through `email.send` and record receipts.

Until those occur, S04-T4 and S04-T8 remain unchecked. No forge API, PR lifecycle or deployment
adapter is introduced merely to remove this deliberate human boundary.
