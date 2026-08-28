# EXP-10 result — continuous product development

**Status:** Complete; founder taste review remains pending  
**Disposition:** `thin affordance`  
**Run:** `exp10_swift_continuous_r1_test` on 28 August 2026  
**Model:** `zai/glm-5.3`, medium effort, USD 25 ceiling, USD 2.624257 charged

## Decision

The existing Goal, team, Work, Attempt, Message, artifact, Git and terminal-callback substrate can
run repeated product-development cycles without repeated owner decomposition. It needed three small
repairs, not a new continuous-work abstraction:

1. a free-standing one-shot Schedule can atomically create a durable message for its accountable
   non-Exec actor, including in a freshly migrated company schema;
2. an interrupted ACP turn with observable text or tool activity resumes instead of being called a
   zero-token no-op; and
3. a mistaken deterministic Work gate can be retired with its history intact and replaced before
   the same Work resumes.

No `Mission` entity, heartbeat, recurring-workflow engine, poller or cron-owned feature loop is earned.
Continuous responsibility remains durable; execution remains bounded and event-driven.

This is not yet evidence for unattended product development as a default. The company produced useful
changes, recovered from a killed worker and stayed quiet, but bad gate and workspace specifications
caused four failed Attempts, one unnecessary replacement Work node and nearly as much lead spend as
Staff spend. The substrate is sounder than the run's operating efficiency.

## What happened

| Cycle | Native outcome | Coordination outcome | Result |
| --- | --- | --- | --- |
| P01 playtest | Raw telemetry was demoted, overlapping world labels were reduced, an objective banner, route chevrons and destination beacon were added; the positive host/client loop stayed green | One standing lead commissioned one Staff owner and accepted exact commit `a4751a20391a032fe3e7733fee9e5b37130a8f16` | Pass, after setup and gate churn |
| Duplicate P01 | No repository change | Exact source-key redelivery created zero Work and zero artifact | Pass |
| B01 regression | Injected parse failure at `5ed412cd595b47224eff3c999aa0ea76d811aee2` was removed; positive delivery and a new outside-zone negative release probe both passed | The exact worker was killed after useful state appeared; the same Work and worktree resumed 12.544 seconds later; clean final commit `f9f5e61ed733d8479cf2ae3078779c73db457317` promoted to `main` | Pass, with one gate-caused replacement Work |
| S01 review | No new product evidence existed | One durable one-shot Schedule survived daemon restart, woke the lead once, created no Work, did not wake Exec and did not reschedule itself | Pass |
| Quiet control | Repository remained clean at `f9f5e61` | Events, Work, schedules and model-attempt counts were identical across an 83-second interval | Pass |

The visual delta is inspectable in [`FOUNDER_REVIEW.md`](FOUNDER_REVIEW.md). It is clearly more
legible, but neither the lead nor this experiment can replace the founder's judgement of whether the
game is good enough to continue.

## Frozen success contract

| Criterion | Evidence | Verdict |
| --- | --- | --- |
| Exact baseline retained and freshly probed | Source `3dc502ae`; experiment evidence commit `7581205`; host/client positive probe passed | Pass |
| Standing lead, Exec outside routine relay | One Game Product lead across all cycles; two Exec wakes total, both during initial setup/erroneous capacity repair; zero routine cycle or schedule relay | Pass |
| Staff-attributable accepted changes | Two completed Work nodes, exact Attempts, artifacts, gates and commits | Pass |
| Native legibility improvement | Before/after captures and measured text-density reduction; mechanics preserved | Pass; taste unknown |
| Duplicate suppression | One exact P01 redelivery, zero duplicate Work | Pass |
| Executable repair and positive/negative closure | Failing parse log first; final validation, positive probe and negative probe all exit 0 | Pass |
| Kill recovery | Commit and dirty state preserved; same Work/worktree resumed in 12.544s; no owner implementation | Pass |
| Durable direct schedule | Schedule `84b4f86b` survived restart, fired once to the lead 4.408s after its due time; zero Exec wake | Pass |
| No evidence, no production | Scheduled turn created no Work, commit, artifact or successor Schedule | Pass |
| No external effects | No effect receipt, publication, purchase or outreach | Pass |
| Source-derived reporting | Work graph, events, messages, Schedule, Git and host spend ledger recorded below and in [`metrics.json`](metrics.json) | Pass |
| One architectural disposition | `thin affordance` | Pass |

## Cost, latency and churn

- Host spend ledger: 217 charged request records, USD 2.624257 total. Staff cost USD 1.314671,
  lead cost USD 1.213287 and Exec cost USD 0.096299.
- OrgIntel observed 25 model attempts: Exec 2, lead 16 and Staff 7. It recorded completed usage for
  23 turns; the two deliberately killed turns correctly lack final usage.
- P01 signal to lead acceptance: 1,782.226 seconds. This is dominated by the wrong seed path, the
  pre-repair false no-op/cooldown and two invalid gate declarations.
- Duplicate P01 judgement: 9.761 seconds and zero duplicate production.
- B01 failing signal to accepted promoted repair: 519.216 seconds.
- Killed worker to the next Attempt on the same Work: 12.544 seconds.
- Scheduled time to atomic fire: 4.408 seconds; to model-attempt start: 4.430 seconds.
- Work: 3 total, 2 completed and 1 abandoned. Attempts: 8 total, 2 produced, 2 blocked and 4 failed.

The lead cost 92.3% as much as Staff. That is the strongest negative result: preserving a supervisor
worked, but repeated gate repair turned supervision into expensive operational recovery. The next
product work should reduce the chance of specifying gates against the wrong execution context, not add
more agents or a richer work graph.

## What generalises

1. A standing accountable actor plus material events is sufficient continuity. An immortal model
   loop is unnecessary.
2. Time can trigger inspection, never justify production. One-shot schedules preserve model
   judgement about whether and when another review is useful.
3. Recovery classification must follow observable evidence. Missing final usage after text or tool
   activity is unknown/resumable, not zero/no-op.
4. Deterministic acceptance machinery is operational state and must be repairable. Retiring a bad
   gate preserves evidence without forcing healthy product work into a new node.
5. Exact workspace and integration-branch semantics are coordination truth. Natural leadership does
   not compensate for ambiguous execution context.
6. Quiet is measurable success. Once material evidence stopped, the company stopped.

## What does not generalise yet

- The signals were controlled injections into an isolated `_test` company, not authenticated real
  playtest, email, telemetry or issue-tracker events.
- One game and one lead do not validate continuous sales, research, marketing or operations.
- A single useful visual pass does not validate a reusable game-development team template.
- GLM-5.3's judgement and spend are one model/workload observation, not a universal staffing curve.
- Technical and legibility evidence cannot establish fun or continued investment; founder review is
  still the prepared last mile.

## Product decision

Keep the current small substrate and the three thin repairs. Do not add recurrence, polling, Mission,
heartbeat or autonomous-improvement state. Before calling this unattended product mode, dogfood one
real inbound signal path and improve gate declaration ergonomics so candidate-local checks are the
obvious default. The detailed failure dispositions are in [`FRICTIONS.md`](FRICTIONS.md).

## Postflight verification

- On final game commit `f9f5e61`, GDScript validation, the positive host/client delivery and the
  outside-zone negative release probe all passed again with both peers exiting cleanly; Git remained
  clean at tree `aa1ad3960e1468eb2f38d17946d597e96d9d0a5a`.
- The full CLI, OrgIntel and daemon suites passed against live Postgres: daemon 158 passed with five
  intentionally live-only tests ignored.
- Strict Clippy passed for OrgIntel, daemon and CLI targets.
- Final verification exposed and fixed one fresh-schema bootstrap defect: migration 0019 now seeds
  the reserved `daemon` sender before a direct Schedule can produce mail.
