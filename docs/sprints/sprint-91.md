# Sprint 91 — owner feedback: one decision and clear capability context

Follow-up to the owner's populated-company review on 17 October. Preserve one authoritative decision per owner action, make missing live telemetry truthful, resolve person-header alignment, implement the small Installed hierarchy and distinguish available know-how from recorded use. The Company overview must describe current state rather than contradict its attention indicator. Main, local installation and cloud deployment are separate observed facts.

- [x] [One canonical Inbox](sprint-91/t01-inbox.md) decision with supporting context; no title/prose matching that hides an unrelated question.
- [x] [Missing live activity](sprint-91/t02-people.md) is not presented as a failed reply; genuine failures retain recovery.
- [x] [A coherent person header](sprint-91/t02-people.md) on desktop and phone without the extra white card.
- [x] [Installed](sprint-91/t03-apps.md): All, Apps, Know-how and conditional Needs setup; searchable, empty groups hidden, existing deep links retained.
- [x] [Company-authored know-how](sprint-91/t03-apps.md) appears in Installed; availability and recorded application remain distinct and inspectable by agent.
- [x] [Company overview](sprint-91/t04-overview-release.md) uses real current context and consistent state; no decorative empty dashboard.
- [ ] [Verified main](sprint-91/t04-overview-release.md), supported local upgrade, read-only real-company acceptance and exact cloud release disposition.

Stories US-295–US-301 own acceptance. Record observed evidence here as each slice passes; test counts alone do not imply completion.


## Observed acceptance so far

- Full product-quality run: 21/21, including the authored-method/native application path, missing live session, hierarchical detail/Back and complete-setup overview under healthy/degraded/failed controlled reads. Two affected hierarchy journeys passed again after preserving detail selection. First-failure traces are retained; a new profile initialization bug and inconsistent Exec display name were repaired before release.
- Six affected routes at 1440/390: 12/12 clipping/overflow/raw-copy/page-error captures passed. Built-in-browser desktop/phone review supplements automation.
- Motion: opening/return gaps 0px, streaming/final width 770px, 276 samples. This is scripted interaction evidence, not autonomous business quality.
- Owner API suite: 127 passed, one intentionally ignored long-running visual server. Company skills: 3/3 real database scenarios, including company observation, assignment precedence and exact applied version. Activity: 6/6. Source checks: zero errors/warnings; production build passed.
- Attention: 16 passed, one explicitly ignored dedicated Runtime review scenario. The three-context regression verifies the newest two details stay inside one item; another human’s direct Room cannot enter the owner’s queue. The old lead-only filter is removed, with one canonical Room read for all active agents. Earlier safety/harness failures are not counted green: a new test omitted its owner fixture, the native Sheets test rejected a database URL without its required _test suffix, and a warm dependency tree lacked the pinned accessibility tool. Those fixtures/dependencies were corrected, and the affected checks passed.
- Read-only Cloud evidence: Core product artifacts built at the earlier product revision; Cloud images built at 3172afd, but Compose the Cloud release and Deploy to Northflank were skipped. The latest separate multiplayer CI run failed before tests because its required database-file setting was unset; the local owner/member boundary suite passed. No live hosted deployment is claimed from image-build success.
