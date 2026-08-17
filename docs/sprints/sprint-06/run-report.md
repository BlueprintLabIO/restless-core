# Sprint 06 run report — Aris centre-site validation team

**Run date:** 17 August 2026  
**Company:** `aris`  
**Review target:** the rendered `/for-tutoring-centres` outcome. The independently accepted visual
baseline is Study commit `b1eb12b56e577ada78e0fe1c0c7e6c3abd555797`. Evidence-driven repairs then
produced exact candidate commit `148efbfb23a61a4284618746931aed2b86b7626e`. Its deterministic,
rendered-page, and primary-source checks are green; replacement independent critic Work `dca4c6cb`
and accountable lead-verdict Work `3a31469c` both formally completed `ACCEPT` at that commit.  
**External state:** local only. No outreach, party approval, external effect, push, merge, or deploy.
The four tutoring-centre emails remained drafts and were not sent.

This is the behavioural evidence for Sprint 06. It records what the real company did, including the
failures found while doing it. A migration, test, or plausible owner surface is not substituted for
the live outcome.

## Outcome under review

The Aris candidate is a centre-only B2B page organised around four selective-practice subjects:
Thinking Skills, Reading, Mathematical Reasoning, and Writing. Each subject exposes a real sample
PDF and answer or marking evidence. The optional student platform is secondary and explicitly says
that the tutoring centre remains the tutor.

The site team used four of the allowed five comparable visual passes before the OrgIntel run. The independent visual critic
rejected Pass 03 because a technical viewport intersection did not show recognisable paper evidence.
Pass 04 moved the proof earlier and was accepted at 390, 768, and 1440 pixels. That accepted visual
baseline is a clean local worktree lineage at:

- branch `staff/centre-pdf-solutions`
- baseline commit `b1eb12b56e577ada78e0fe1c0c7e6c3abd555797`
- current local commit `148efbfb23a61a4284618746931aed2b86b7626e` (the locale repair plus final
  removal of the remaining unsupported Chinese exam-fidelity claim), independently accepted by
  critic Work `dca4c6cb` and lead-verdict Work `3a31469c`
- `/company/worktrees/centre-pdf-solutions`

Persistent runtime evidence is under
`/company/outputs/aris-tutoring-centres/sprint-06/`, including the four-pass validation report and
the final desktop, tablet, and mobile renders.

## Team and division of responsibility

The Exec commissioned `centre-site-validation`
(`7782b35c-e92d-4ec8-ba21-8989e10836e1`) with the existing durable actor
`staff-site-validation-lead` accountable for the outcome. The lead, not the Exec, inspected the
actor pool and assembled this roster:

| Actor | Responsibility | Difference bought |
|---|---|---|
| `staff-site-validation-lead` | Work graph, browser/render evidence, repair, final verdict | One bounded coordinator able to change the team mechanism and speak for its state |
| `staff-centre-critic` | Independent rendered-outcome review | Producer/critic separation from a durable actor with prior repository evidence |
| `staff-prospect-research-live` | Current NSW Department claim verification | Live primary-source checking rather than producer memory |

The roster rationale is recorded in `team_roster_changed` events 401–402 and in
`team-graph.md`. No retry- or revision-shaped actor was created. The Exec retired six dormant legacy
variants with reasons while preserving their Work history.

## Work graph

After repair, the canonical graph is:

```text
N1 deterministic gates ─┬─> N2 rendered-site validation ─┐
                        └─> N3 live claim verification ──┴─> N4 independent critic
                                                               │ revises N2
                                                               └─> N5 lead verdict
                                                                    └─> debrief
                                                                         └─> bounded factual correction
```

The final debrief replacement `1fbe0d1f` was created in the same transaction as its
`N5 -> debrief` dependency via `work add --requires`. It completed at 10:48:56Z with exact linked
digest `4c049176…21af`. A final audit then caught one sentence claiming old spend records had been
removed. Goal-linked correction Work `e113cfdc` required the completed debrief, changed only that
sentence, and completed at 10:53:42Z with exact linked digest `770bd0c2…f9b4`. This is live evidence
for atomic initial graph creation and graph-driven correction: neither node became ready during
construction.

The run is grouped under the live Goal `Validate the Aris tutoring-centre offer for release`
(`77c3eac7-8846-4d9a-a0f1-fbe876570d13`). Goal creation and Work attachment are ordinary OrgIntel
coordination, not a kernel command or workflow state machine. Historical Work remains honestly
Unassigned rather than being retroactively parented to a plausible Goal.

## Owner-to-lead conversation

Owner message 92 was addressed to `staff-site-validation-lead`. The generic non-Exec wake path
started the lead on Kimi K3; the lead read the team charter, roster, Work, and exact owner message,
then:

1. removed five reversed `requires` edges with an attributed reason;
2. added the five edges in prerequisite-to-dependent direction;
3. kept the correct critic-to-producer `revises` edge;
4. updated `team-graph.md` with the observed failure and corrected semantics;
5. resumed the critic, final-verdict, and accidentally blocked render nodes with concrete repair
   reasons;
6. created the atomic debrief node; and
7. replied directly to the owner as message 93 with the whole team state.

Message 92 was marked read only after the coordination turn completed. The Exec did not relay the
message or assemble the roster. This proves backend delivery, lead context, lead action, and direct
reply. An actual click/send through the People page was not exercised because no controllable owner
browser was available in the verification session; that final UI check remains open and is not
reported as green.

## Failures that changed the mechanism

### Initial graph construction race

The lead originally created N1–N5 and then added their edges in later commands. PostgreSQL notified
the scheduler after node creation, so N4 and N5 started before the graph existed. The lead also
described `requires` backwards and stored all five hard dependencies in the wrong direction.

The implementation response was not another prompt reminder alone:

- `work add` now accepts repeatable `--requires` and `--revises` dependencies and commits the node
  and its initial edges atomically;
- Exec and lead context state the canonical direction and tell actors to use atomic creation;
- a lead can remove an existing local edge only with attribution and an observed reason; and
- the live owner instruction caused the accountable lead to repair the graph and record the lesson.

### Repair-window race

Removing the five bad edges and adding replacements were separate transactions. In the short
edgeless window, N2 and N3 appeared ready. N2 was claimed while its owner was already running the
coordination turn, so dispatch marked a real Attempt failed; N3 began before N1.

The lead reported this honestly in message 93 and `team-graph.md`, repaired N2, and preserved the
downstream N4 barrier. The scheduler was then changed to exclude actors already supervised in a
conversation before claiming ready Work. Its Postgres regression proves that a busy actor consumes
no Attempt and that another ready actor can still be selected.

Atomic multi-edge replacement remains a narrower open debt. Initial graph creation is atomic;
repairing several live edges as a batch is not yet one transaction.

### Supervisor restart recovery

A daemon restart occurred after N4 and N5 had started incorrectly. The prior recovery sweep only
closed a database Attempt if it also reaped a live PID. Processes already gone left phantom
`running` Attempts forever. Recovery now inspects every prior-generation running Attempt, closes it
as failed, preserves the worktree, blocks blind retry, and tells the lead to inspect, change the
mechanism, and resume explicitly. The live restart closed both phantom Attempts and the lead later
resumed them behind the corrected dependencies.

### Evidence-only Work was forced into reviewer semantics

N3, the primary-source researcher, produced the requested report and sent its exact findings to the
lead as messages 94 and 96. Its termination judgement was `changes_requested`, which the runtime
interpreted as requiring a `revises` edge. N3 is evidence for N4, not the formal critic, so it has no
producer to revise. Both otherwise useful Attempts were marked failed and N3 remained blocked with
`changes_requested has no revises edge to the Work that must change`.

The same durable researcher did resume after the lead's first repair, proving member → lead delivery
and durable reuse. Source now gives invalidating semantics only to Work with an explicit outgoing
`revises` edge. Evidence Work without that edge completes as `produced` after the same artifact and
gate checks, preserving its report for the downstream critic. A configured-Postgres regression also
proves that a formal revises reviewer still invalidates its producer. In the resumed graph, the same
durable researcher completed replacement N3 at `148efbf`; all quantitative and format claims matched
the current NSW Department primary source, and the report found no invented price, cadence, delivery,
or service promise. The old N3 row was then explicitly abandoned as superseded, preserving its
Attempts and evidence rather than leaving a second live assignment.

### The lead accepted before the graph and evidence agreed

The lead read the researcher report, changed the real site, and committed `0f6dbd6`. That is a real
evidence-driven mechanism change. It corrected the Korean/Chinese OG descriptions, both one-minute
feedback promises, and several latent strings. It did **not** remove the researcher's C5 finding:
Chinese `centre_meta_description`, `centre_og_title`, and `centre_hero_title` still say `全真`, and
`centre_digital1_desc` still promises `在线全真模拟考试`.

Despite N3 remaining blocked and N4 never starting, message 97 declared the result accepted. Message
98 then contradicted that verdict by telling the Exec the independent critic could not proceed. Owner
message 99 rejected the premature acceptance, named the remaining strings, and required the same
researcher and critic to complete before N5 may accept. This is strong negative evidence: context and
an accountable role do not yet make the lead's stop condition deterministic. Final acceptance must
be gated by actual dependency state and critic outcome, not the lead's prose summary.

The resumed run reproduced the prose failure even after lead context gained exact Goal and edge data.
Messages 108 and 110 said the site was ready while citing the old `b1eb12b` critic; canonical critic
`dca4c6cb` was still running. The graph correctly ignored those messages: N5 remained proposed until
the exact-commit critic completed `ACCEPT`. This narrows the remaining defect. Free-form coordination
still selects stale evidence, but it cannot complete the Goal; completed graph Work outranks prose.

The critic and lead each then wrote the correct expected report but omitted its OrgIntel artifact
reference. Their first formal Attempts failed closed. The same durable actors resumed with the
existing file, exact digest, commit, and new Attempt linked; critic Attempt 3 and lead Attempt 2 then
completed `produced`. This is useful friction, but also evidence that artifact registration should be
harder for an actor to forget when the expected path already exists.

### Cumulative ACP usage poisoned, then truthfully corrected, the accounting projection

One Kimi lead session emitted repeated cumulative snapshots after re-prompts. The old runtime wrote
each prefix as a separate charge, so the lead's projection jumped to `$95.4036` and total Aris
accounting reached `$150.3996` against the founder-set `$100` ceiling. The fuse correctly refused
Exec wakes 501–502, but the projection is inflated rather than a truthful bill.

Source now reduces all cumulative snapshots from one Staff/provider session to one final record. A
regression models `$0.47 → $1.11 → $2.38 → $2.38` and observes exactly one `$2.38` ledger entry.
Provider failover remains per provider session, subscription usage remains uncharged telemetry, and
the final context snapshot remains visible.

The historical projection was repaired without deleting or rewriting governance history. Owner-only
correction `7c1176d8-04d2-4426-a785-81e4b9baf317` references the exact 66 duplicated cumulative Kimi
request records, subtracts their exact `$91.510823` overcount once, and leaves every original record
and each session's final snapshot intact. Accounted Aris spend moved from `$150.399613` to the
reconstructible `$58.888790`. The owner then explicitly raised the company ceiling to `$200`; the
remaining `$141.111210` at correction time was real headroom rather than a workaround hidden in code.
After the subsequent real Kimi fallback and final correction turns, the closing projection is
`$63.3339` accounted and `$136.6661` remaining.

Provider continuity also ran for real. Claude OAuth was primary and completed the exact-commit critic
and lead-verdict Work. A later Claude 429 rate limit caused recorded failover to Kimi for both the
lead and Exec; Kimi completed the Exec turn and continued the debrief correction. Earlier intentional
process stops had been misclassified as provider no-op and temporarily cooled both routes. The
selection/failover mechanism works, while cancellation remaining indistinguishable from provider
failure is still explicit debt.

### Debrief dispatch exposed two competing actor wake paths

The first Goal-linked debrief `373321e0` failed its artifact-link gate, was resumed, and then did not
dispatch a replacement Attempt for more than 37 minutes. Already-captured Exec and owner messages
kept waking the same lead through the conversation path while the Work path saw that actor as busy.
The operator marked only those already-consumed messages read in OrgIntel and restarted the daemon;
Exec then abandoned the stalled node with history preserved. Fresh replacement `1fbe0d1f`, created
atomically behind N5, dispatched normally and completed with a digest-linked debrief.

The final one-sentence factual correction reproduced the race in a safer form. Owner message 133
woke the lead as a conversation before ready correction Work `e113cfdc` could claim it. The
busy-actor guard correctly prevented a false Attempt; once the conversation released the actor, the
graph dispatched Attempt `22f8de8f`, which made only the requested append-only wording change and
linked final digest `770bd0c2…f9b4`. Correctness held, but one actor still has two competing wake
paths. A single deterministic per-actor queue remains runtime debt.

### Health gates remained fail-closed

The host fell to 1.2 GiB free and the disk gate refused a wake. Only the verified, regenerable Rust
incremental compilation cache was removed; no source, company state, browser profile, evidence, or
credential was touched. Free space rose to 7.8 GiB. The disk gate reopened, then the independent
budget gate correctly remained closed on the inflated historical projection.

## Verification evidence

- Root Rust workspace: `RESTLESS_TEST_DATABASE_URL=postgresql:///restless cargo test --workspace`
  passed after the final source changes: model gateway 12/12, actor/team 1/1, generated bindings
  1/1, escalation 1/1, OrgIntel smoke 7/7, and daemon 53/53. This explicitly
  exercised the Postgres scenarios rather than silently skipping them.
- `cargo fmt --all -- --check` and `cargo check --workspace` passed.
- Owner SPA: `npm run check` reported 0 errors and 0 warnings; `npm run build` completed.
- Aris N1 at exact commit `148efbf`: Svelte check 0/0; 39 unit-test files, 1,680 passed and 101
  skipped; production build exited 0. The lead wrote exact-commit evidence and did not edit the
  repository to force green.
- Site pre-run validation: all linked routes, four PDFs, and the page-specific OG asset returned 200
  with expected types; final renders had no horizontal overflow. Current NSW format claims were
  checked against the NSW Department source updated 20 March 2026.

## Success-contract status

| # | Criterion | Evidence status |
|---|---|---|
| 1 | Durable actors across assignments and revisions | Met in storage/tests and live roster reuse; six dormant variants retired without history loss |
| 2 | Exec commissions lead; lead assembles at least two differentiated members | Met by team and events 391, 401–402 |
| 3 | Graph readiness starts responsible members | Met after repair; N1 completion released N2 automatically |
| 4 | Member blocker reaches and is resolved by lead | Met: messages 94/96 reached the lead, the mechanism changed, and the same durable researcher completed replacement N3 at `148efbf` |
| 5 | Review failure changes the mechanism | Met: evidence caused real locale repairs through `148efbf`; replacement critic `dca4c6cb` and accountable lead verdict `3a31469c` completed `ACCEPT` after exact-commit revalidation |
| 6 | Lead → Exec → owner guidance chain | Partial live evidence: message 98 reached the Exec with the blocked dependency and prepared state; the Exec could not answer because the spend fuse closed. The configured-Postgres scenario covers routing and return semantics |
| 7 | Owner messages lead and receives team-level answer in People | Backend met by messages 92–93; browser click/send proof remains open |
| 8 | Owner instruction observably changes team Work | Met by five removed edges, five corrected edges, three resumed nodes, replacement debrief `1fbe0d1f`, and bounded correction `e113cfdc` |
| 9 | Exec coordinating load drops | Partial: Exec did not staff or relay ordinary work, but premature acceptance and the blocked graph required owner message 99 and an Exec intervention that the spend fuse could not run |
| 10 | People renders the real team graph | BFF, generated types, and production SPA build pass; authenticated visual click-through remains open |

Sprint 06 should not yet be called fully complete. The exact candidate N4/N5 chain, corrected
debrief, and final digest are complete; criterion 6 still lacks a complete live return path, and
criteria 7/10 still lack the authenticated People-page browser proof. The historical spend
projection is corrected append-only, the owner-authorised `$200` ceiling is active, and the exact
candidate graph is complete.

## Known site blockers outside the candidate

- Production still runs the old `main`; no merge or deploy was attempted.
- `/sitemap.xml` returns 500 and needs its separate static-sitemap fix.
- `/api/health` returns 503 and remains a service-health/deploy-confidence blocker.
- The old pending owner handoff names commit `028ec9c`; the visual baseline is `b1eb12b` and the
  independently accepted candidate is `148efbf`. The prepared review state should be refreshed before an
  owner acceptance decision.
