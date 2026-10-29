# Capacity admission and degraded-runtime reads — in progress

Owner priority: keep the current paid plan and reduce compute and AI use.
Layers: Runtime capacity observations, the existing OrgIntel Work scheduler, owner projections.

This is not a completed ticket or a deployed batch. Sprint 109 and all A0 acceptance remain open.

## Current implementation

- New productive Work is left unclaimed in OrgIntel while shared-pool or individual Runtime live memory is above
  the admission threshold. No new queue, Work state, model wake or meter is created.
- Live memory excludes inactive file cache; admission reserves the larger of ten percent or
  256 MiB below memory.high, capped at half the ceiling for small containers.
  Missing configured counters defer new Work rather than assuming zero use.
- The existing scheduler rechecks ready Work at its normal five-second overdue cadence.
- Resources projects the same admission reading with an observable wait reason.
- Company settings bound the full cached Runtime Doctor to three seconds and retain
  Authority/OrgIntel controls with an unavailable Runtime observation.
- Inbox bounds its cached browser-health probe to two seconds.
- Diagnostic execs check existing memory counters before starting, serialize each program,
  retain a 30-second retry window across cancellation, and use a remote six-second deadline
  in the known Runtime image. Lifecycle operations retain their existing deadlines.
- One cached Doctor refresh survives a shorter owner response deadline; overlapping reads
  do not queue background refresh tasks.
- Core-owned Documents disables the duplicate Docker health check and retains its actual
  HTTP/protocol readiness check. Existing sidecars migrate without replacing their database.
- Startup orphan recovery waits behind the existing recovery barrier for memory headroom.
- Expired Doctor observations retain their actual observation timestamp during one background
  refresh. Owner reads do not lose the last reading simply because their refresh interval is
  longer than the health cache's freshness interval.
- A failed persisted Inbox placeholder no longer holds the whole application's first reveal
  during its background retry. Attention uses ordinary refresh/manual retry, not stacked retries.
- Surface capture now waits for actual loading-screen dismissal rather than network idleness.

## Latest local evidence, 30 October Sydney time

- Six-program, 30-request pressure probes passed for both a full individual Runtime and
  a busy shared pool. No process growth or child OOM/max events; actual execution recovered
  after backoff. A deliberately slow remote process was terminated without leaving a child.
- Full Resources state checks passed at 1440/390, with zero-pixel first-reading and waiting
  shifts. Fresh pressure entry passed both widths without overflow or page errors.
- After diagnostic guards, Work c8c81b76-66a1-4324-b664-fed44bb4915b stayed unclaimed during
  the sustained 960 MiB load; model requests did not increase. It started 3.546 seconds after
  load removal and completed with its actual attached file. All child OOM counters were zero.
- A later pending-Work restart retained Work 456a1348-ff02-4680-a85f-e83645439c13 with no
  Attempt or new model request during the hold. It started 1.557 seconds after the recorded
  release and produced one attached artifact. All child OOM counters stayed zero and all OCI
  initialization processes cleared afterwards. Startup recovery still launched transient OCI
  processes during this hold; the new recovery admission check addresses this separately.
- Normal development startup Doctor passed after the cached refresh fix. The first per-container
  diagnostic mutex had blocked nested coordination callbacks; it was replaced with program lanes.
- Actual Documents configuration reports Healthcheck.Test=["NONE"], and its HTTP service is ready.
  Its process census remained one process and zero OCI initialization processes during pressure.
- Latest frontend checks and all three reveal regression tests pass. No real provider tokens were
  purchased for these protocol fixtures. They do not measure AI quality or dollar savings.

Final local checks:

- The final real-pressure diagnostic runs also call the actual storage-health probe. Individual
  and shared tests pass without additional processes or child OOM/max events; the slow remote
  process is reaped. Logs: /tmp/s109-observation-resource-live-{individual,shared}.log.
- Startup with the populated company held recovery while the shared pool was full. Fresh
  Resources entry passed at 1440/390. A strict unchanged-process census did not pass: the Runtime
  had four base processes and one transient fifth process. No child OOM kill occurred, and after
  release all transient processes were gone. Do not describe this as a zero-growth full-stack run.
- The last pending-Work restart started at 15:35:03.165879Z, 3.279 seconds after release at
  15:34:59.886767265Z. Work completed with one actual attached file. Model requests remained
  at eleven throughout the hold. This used the active scheduler, not the later disabled-scheduler
  fixture used to isolate startup and page probes.
- The complete self-hosted multiplayer journey passed, including owner-only compute readings,
  denied admin/member host access, three-way persisted document edits, role changes and revocation.
  Its runner removed its containers, volume, databases and roles afterwards.
- Reno-scale data (30 finished Work, long threads and library files) passed desktop/phone Resources
  captures and built-in-browser review. Three unrelated seeded open Work nodes were explicitly held
  for visual verification. The long chat opened and reopened at the latest message (zero-pixel gap).
- Streaming stalled with two conversations in the 768 MiB stress Runtime: live anonymous memory
  nearly filled that ceiling. This remains evidence for the broader actor-start/concurrency work.
  The comparison at a local 2 GiB Runtime ceiling passed: a new streamed reply supplied 396 samples,
  with streaming/final widths both 770 px and no tool-row flashes. The shared local test pool was
  6 GiB for the multiplayer/scale comparison; production and its paid plan were unchanged.
- The motion journey now requires a new reply and actual width samples; an older scripted reply
  cannot make a repeat look successful. It allows 30 seconds for the scripted tool/reply journey.
- Actual owner reads across cache expiry retained available Runtime observations in 0.255–0.315 s;
  the background refresh advanced the recorded observation timestamp. Evidence:
  /tmp/s109-observation-last-reading-report.json.
- Final frontend checks, formatting and reveal regressions pass. The local company was canonically
  destroyed, four dedicated infrastructure containers, three volumes and one network were removed,
  and the owned daemon/model/Vite processes, profiles, private database URL and test units were removed.
  Temporary browser tabs were closed and the viewport reset. Other resource debt was only reported.

Retained evidence: /tmp/s109-observation-full-after-release.json,
/tmp/s109-observation-final-after-release.json, /tmp/s109-observation-direct-held-surfaces/report.json,
/tmp/s109-observation-final-states/report.json, /tmp/s109-observation-lanes-individual.log,
/tmp/s109-observation-lanes-shared.log, /tmp/s109-observation-roles/journey.log,
/tmp/s109-observation-scale-motion-comparison.log. Clean-main qualification and production
deployment remain open.

## Earlier failed experiments, retained for diagnosis

Worktree: /home/yail/code/work/slot-3, branch codex/s109-capacity-wait, based on c409988e.
All new code remains unmerged and undeployed.

A disposable capacity_test company ran under a 2 GiB shared pool, with 768 MiB Runtime
and computer ceilings, a scripted Anthropic protocol server and the real daemon/CLI.

- The memory/cached-file/recovery unit scenario passed.
- Initial frontend checks passed; later reveal-regression tests passed (3).
- verify-resources passed waiting/available/error/unknown/recovery states at 1440/390;
  first-reading and memory-wait geometry movement were both zero.
- A smaller real load showed the wait reason in the built-in browser and completed one
  artifact-producing Work Attempt with the correctly attached file.
- That smaller-load run is NOT evidence of holding until operator release: its start timestamp
  preceded load removal by about forty seconds. Natural memory fluctuation crossed the threshold.
  An earlier conversational progress claim of sub-three-second post-release resume was corrected.
- A sustained 960 MiB allocator independently exceeded admission, eliminating that ambiguity.
  The third Work remained unclaimed for over ten minutes; the calibrated model log stayed at
  eleven requests over the sustained hold, including the earlier control and completed activity.
- Immediately before removal, Attempts were empty and the live admission reading was waiting.
  Load-stop timestamp: 2026-10-29T14:03:50.908510782Z.
  Attempt start: 2026-10-29T14:03:54.363569Z (3.455 seconds later).
- That third Attempt subsequently stalled and is NOT a successful useful-outcome smoke.
- Under sustained pressure, company settings returned 200 in 3.317 seconds with Runtime
  unavailable, instead of failing the entire company view.
- Cold Resources still failed at desktop/phone: Inbox browser-health reads blocked its initial
  navigation summary. Diagnostic output identified attention with status=error,
  fetchStatus=fetching and persisted placeholder data.
- The failed-placeholder regression failed before the fix and passed afterwards. The phone
  subsequently rendered actual controls, counters and the waiting reason in the built-in browser.
  Fresh cold-load capture still failed. Do not treat the in-place recovery as a complete cold smoke.
  The newly added two-second Inbox browser-probe bound still needs a rebuilt daemon and that test.
- Docker pause and external cgroup freeze experiments returned fast settings reads but did not
  produce the intended unavailable source assertion; those experiments are failed fault probes,
  not proof of the whole-doctor timeout.
- A direct cgroup process census ultimately found many runc initialization processes in the
  disposable Runtime, with local memory.events reporting 15 OOM kills and substantial OOM/max
  events. Its supervisor was gone; the container still appeared running.
  Parent-pool events had reported zero. Parent events alone do not prove no child OOM.
- Killing a timed-out Docker CLI does not prove its OCI startup is gone. Read-only Docker exec
  probes and insufficient per-runtime admission are the next investigation.
- Production was not changed. Latest read-only check: desired/applied 123/123, healthy,
  about 4.4 GiB available on the current 8 GB plan. No resize was applied.

## Release checklist and remaining resource scope

- [x] Individual/shared Work admission, diagnostic pressure/backoff/remote-deadline tests.
- [x] Sustained hold, useful artifact after timestamped release and pending-Work restart.
- [x] Child OOM/process census; finite transient full-stack process and failed comparison retained.
- [x] Cold/warm desktop/phone entry, state recovery, real-scale data, access roles and motion.
- [x] Canonical fixture destruction and exact owned-resource cleanup.
- [ ] Qualify a clean main commit, deploy signed Core/Cloud batch and verify production health.
- [ ] CPU/start concurrency and conversation launches, jobs/tools, editable per-company ceilings,
  histories/catalogue, wider idle reductions and matched AI cost/quality requirements remain open.

## Retained local artifacts

- /tmp/s109-wait-resource-states/report.json
- /tmp/s109-wait-sustained-surfaces/report.json (failed cold loads)
- /tmp/s109-wait-sustained-fixed-surfaces/report.json (still failed cold loads)
- /tmp/s109-wait-pressure3-at-release.json
- /tmp/s109-wait-work3-held-at-release.json
- /tmp/s109-wait-release3-at
- /tmp/s109-wait-final-graph.json
- /tmp/s109-wait-procs.json (actual child OOM/process census)
- /tmp/s109-wait-final-company-cleanup.log
- /tmp/s109-wait-cleanup.log
- /tmp/s109-wait-debt-after.log

Cleanup: canonical company destruction removed the Runtime, computer, company and attachment
volumes, cell and config. Four dedicated infrastructure containers, three volumes and one network
were removed. The private test/model profiles, test unit/drop-ins and private database URL were
removed; the model and stack processes were stopped. The temporary browser tab was closed and
the viewport override reset. Other profiles and caches were preserved.


Latest checks: cargo check -p restlessd and the full frontend check passed after the Inbox
browser-probe bound was added. That is compilation/type evidence, not a passing load smoke.
