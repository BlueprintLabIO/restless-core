# Sprint 91 — owner feedback: one decision and clear capability context

Follow-up to the owner's populated-company review on 17 October. Preserve one authoritative decision per owner action, make missing live telemetry truthful, resolve person-header alignment, implement the small Installed hierarchy and distinguish available know-how from recorded use. The Company overview must describe current state rather than contradict its attention indicator. Main, local installation and cloud deployment are separate observed facts.

- [x] [One canonical Inbox](sprint-91/t01-inbox.md) decision with supporting context; no title/prose matching that hides an unrelated question.
- [x] [Missing live activity](sprint-91/t02-people.md) is not presented as a failed reply; genuine failures retain recovery.
- [x] [A coherent person header](sprint-91/t02-people.md) on desktop and phone without the extra white card.
- [x] [Installed](sprint-91/t03-apps.md): All, Apps, Know-how and conditional Needs setup; searchable, empty groups hidden, existing deep links retained.
- [x] [Company-authored know-how](sprint-91/t03-apps.md) appears in Installed; availability and recorded application remain distinct and inspectable by agent.
- [x] [Company overview](sprint-91/t04-overview-release.md) uses real current context and consistent state; no decorative empty dashboard.
- [ ] [Patched browser](sprint-91/t05-browser-release.md) and unchanged security gate for the hosted release.
- [ ] [Verified main](sprint-91/t04-overview-release.md), supported local upgrade, read-only real-company acceptance and exact cloud release disposition.

Stories US-295–US-301 own acceptance. Record observed evidence here as each slice passes; test counts alone do not imply completion.


## Observed acceptance so far

- Full product-quality run: 21/21, including the authored-method/native application path, missing live session, hierarchical detail/Back and complete-setup overview under healthy/degraded/failed controlled reads. Two affected hierarchy journeys passed again after preserving detail selection. First-failure traces are retained; a new profile initialization bug and inconsistent Exec display name were repaired before release.
- Six affected routes at 1440/390: 12/12 clipping/overflow/raw-copy/page-error captures passed. Built-in-browser desktop/phone review supplements automation.
- Motion: opening/return gaps 0px, streaming/final width 770px, 276 samples. This is scripted interaction evidence, not autonomous business quality.
- Owner API suite: 127 passed, one intentionally ignored long-running visual server. Company skills: 3/3 real database scenarios, including company observation, assignment precedence and exact applied version. Activity: 6/6. Source checks: zero errors/warnings; production build passed.
- Attention: 16 passed, one explicitly ignored dedicated Runtime review scenario. The three-context regression verifies the newest two details stay inside one item; another human’s direct Room cannot enter the owner’s queue. The old lead-only filter is removed, with one canonical Room read for all active agents. Earlier safety/harness failures are not counted green: a new test omitted its owner fixture, the native Sheets test rejected a database URL without its required _test suffix, and a warm dependency tree lacked the pinned accessibility tool. Those fixtures/dependencies were corrected, and the affected checks passed.
- Read-only Cloud evidence: Core product artifacts built at the earlier product revision; Cloud images built at 3172afd, but Compose the Cloud release and Deploy to Northflank were skipped. The latest separate multiplayer CI run failed before tests because its required database-file setting was unset; the local owner/member boundary suite passed. No live hosted deployment is claimed from image-build success.


## Local installation — 17 October

- Installed product source: `a2209c4bab8d4561516849e9f012257ad5a71eb9`; stable release `1f84b56d9fe604931f0d`, previous `d892fc447eecb3b38d05`. Installer reported ready, no rollback; installed status reported source match, health 200, active installed binary and draining false.
- Required release connection gate actually ran: 23 passed, zero failed, one optional skills-compatibility suite skipped. Separate native company-authored-method and exact-application checks above passed. The first two activations deferred at the 30-second active-work boundary without interrupting a process. The final supported promotion was invoked with `--force`; the installer emitted no forced-replacement warning and completed ready.
- Read-only Sydney Resale acceptance: all attention sources available, zero pending items, the original handoff still resolved with the owner's exact answer. No dismissal or approval mutation was needed. Work continued across installation (196 to 200 Work, 289 to 293 artifact references at observation).
- Real Installed detail showed nine methods, availability for 15 agents, and 13 recorded agent/method applications. Code review displayed Blossom and Bilbo with exact digest/date. Built-in-browser desktop/390px review confirmed the person header and neutral missing-activity state; Company showed its authored purpose and a health action matching its attention status.
- [Connections CI](https://github.com/BlueprintLabIO/restless-core/actions/runs/38914091642) and [product artifacts](https://github.com/BlueprintLabIO/restless-core/actions/runs/38914091653) passed for the installed product revision.
- Hosted rollout is now explicitly authorised by the owner. [Immutable Core release](https://github.com/BlueprintLabIO/restless-core/actions/runs/38916854059) was dispatched for the same source; its result and the downstream Cloud deployment remain pending. The hosted browser is signed out; database access is not evidence of an authenticated owner journey.

The local company Runtime was also reconciled through the owner recovery API after a final image check found it still on c9fbc870f0c0. The supported recovery replaced the container while keeping the volume; the subsequent probe matched current/target image ID a43e180304b495da8f67d6829b1da3cab2d12af99891e971d8b78b1c10d4430f, with coordination, supervisor and browser/desktop available. The first immediate startup probe was degraded; readiness is based on the later completed probe, not the replacement acknowledgement.
## Hosted release blocker and browser patch

Immutable run 38916854059 qualified a2209c4b but publication failed its unchanged High/Critical scan: the pinned Chrome 155.0.8059.39 has fixes in 155.0.8059.79. No sealed release or hosted deployment was produced. Both patched archives were downloaded from Google's per-version Chrome for Testing manifest and hashed independently; the AMD64 binary reported the patched version and rendered about:blank in an isolated Runtime filesystem (network disabled, exit 0). ARM64 archive verification is not an ARM execution claim. The exact canonical image build, rescan and rollout remain required.

The first local image build also failed to resolve Ubuntu package hosts. An isolated probe confirmed the default resolver fails while an explicit public resolver works. This is a build environment finding, not a waived checksum or scan.