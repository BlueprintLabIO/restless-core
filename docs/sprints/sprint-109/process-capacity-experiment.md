# Experimental process-capacity admission

This local Core branch accompanies the separate Cloud workload task-cap
prototype. It reads `pids.current` and `pids.max`, waits before finite limits,
and leaves ready Work, agent turns and queued heavy jobs with their existing
schedulers. Resources reports a separate process-capacity wait reason. Missing
or malformed counters fail admission; an older host with `pids.max=max` retains
its existing behavior. A finite process limit also applies to individual Runtime
containers, including one whose memory limit is unbounded.

The process headroom is an experimental ten percent or 32 tasks, capped at half
the limit; it is not yet qualified for real Chrome or multiple simultaneous
agent starts. Runtime unit tests passed (48, four live tests skipped), the actor
reservation test passed, the owner crate compiled and frontend checks passed.
These checks do not establish that the scheduler and all browser/startup paths
actually wait and resume under a real process shortage. Role and visual checks,
real mixed-workload measurement, host upgrades, backlog behavior, and a full
release qualification remain required. Do not deploy this batch alone.

Code inspection found additional launch paths outside the current Work and actor
admission: starting or reconciling a Runtime and starting/replacing its browser.
In particular, reconciliation can stop and remove an existing container before a
replacement is started. The release must preserve the running container while
process capacity is unavailable and coordinate simultaneous starts across
companies; otherwise a resource shortage can turn a healthy workload into an
outage. The prepared real-pressure job fixture remains unrun; it now restores its
own temporary cap before cleanup and uses an append-only marker to detect a
repeated effect. WSL swap was nearly full while the daemon build was stopped.

The local branch now refuses a Runtime or browser start when the shared process
reading is already busy, and checks before retiring the existing Runtime or
browser during replacement. Runtime/browser lifecycle starts use one account
mutex. This does not reserve headroom against independent agent or heavy-job
starts, and a change in pressure after the check can still make replacement
fail. The new startup guard has compiled but has not yet passed the prepared
live pressure smoke or a concurrency/rollback test. Retain the old runtime under
all failed replacements and prove safe recovery before release.

Queued heavy-job starts now take the same account lifecycle lock as Runtime and
browser starts, then re-read capacity before Docker start returns. The owner
crate compiles with this change. This does not yet coordinate independent agent
starts, guarantee a Docker start cancelled after timeout, or establish that a
replacement can always roll back. The real-pressure fixture remains unrun and
this is not a production isolation claim.

The account lifecycle lock is now taken only for an actual Runtime or browser
start/replacement, not for the already-running no-op path; the browser acquires
it separately after Runtime startup to avoid recursive locking. This reduces
head-of-line blocking but has only passed an owner-crate build check. It has not
yet passed live concurrency, cancellation, upgrade or user-interface tests.
Independent ACP agent startups remain outside the boundary, and replacing a live
container still needs a proved rollback when capacity changes after admission.

A local real-pressure smoke on 31 October ran the actual daemon built from Core
`1aca5df709a781250b616d69842d4254dde1e7a0` (binary SHA-256
`34d702809c030501bc9f850bb68268238df66e95be0f524179f2556db17faddd`)
with the Cloud task-cap installer and one disposable company. The temporary
owner workload pool held 35 tasks and was narrowed to 59. Owner principal,
Inbox and Work APIs remained available; an actual heavy job stayed in its
capacity queue without its effect, then ran exactly once after restoring task
capacity. The pool recorded `pids.events: max 0`, showing admission avoided a
kernel process rejection in this scenario. Cleanup reported `[]` and the test
slice was removed. No model provider was called. The owner API used a stub
cockpit, and this is one job rather than a simultaneous agent/browser, multi-owner
or production test. The full resource release gate and rollback proof remain open.

An expanded local smoke against the same daemon and isolated owner pool used a
second existing local image to force Runtime reconciliation, then saved a changed
browser site list while process admission was busy. The previous Runtime and
browser container IDs both remained running and unchanged; owner APIs remained
available, the queued job later ran once, `pids.events` stayed `max 0`, and the
fixture reported clean plane and slice removal. No paid model was called.
This proves preservation at the instant of pressure, not automatic application
afterward. The settings route saves the site list even if browser refresh fails;
its current code only logs that failure, and a dedicated safe retry and visible
pending state have not been established. The automatic release roll-forward,
concurrent actor starts and rollback after pressure changes remain open.

The local branch now records a durable pending browser refresh before saving
home sites, retries it on the existing maintenance cadence when the company is
quiet, and reports the pending result to the owner. It does not wake a stopped
computer, and an active browser lease blocks the deferred restart. The same
company lifecycle lock orders this save against refresh and startup. A live
single-company pressure fixture changed the sites while the pool was busy,
restarted the plane with the marker still present, restored capacity, and saw
maintenance replace the browser with the saved setting; the old browser stayed
running while busy. The queued job ran once, owner APIs stayed available, no
kernel PID rejection occurred, and fixture cleanup succeeded. This used a stub
cockpit and no model provider. The final marker cleanup on company destruction
and the restored not-found response were compile-checked after that smoke, not
re-run in it. Real browser-lease races, user-interface visual checks, full
role coverage, actor-start races and rollout remain open.

A further live test exposed an unsafe path: automatic `up` could replace the
browser while a synthetic owner lease was active, despite the maintenance
quiet check. The first run failed this assertion and cleaned up. The local fix
now takes the browser control lock without waiting in the reverse order and
rechecks the lease at both browser replacement and Runtime reconciliation. It
checks process headroom before attempting the browser diagnostic. A second
fixture revision waited until the owner API observed the synthetic lease, kept
it active through a full maintenance interval, then released it. With daemon
binary SHA-256 `3d478c9688785561b787f614c12dd9a5fcb6020e2027e8f270b5efc7b405a746`,
the browser remained the same while held, applied the saved sites after release,
the queued job ran once, owner APIs stayed available, `pids.events` was `max 0`,
and cleanup reported no problems. The lease was injected in a disposable
company; this is not a real browser attach, live UI, concurrent take/release
race, production upgrade or full resource gate.

After that smoke, code review found that a stopped Runtime cannot answer the
browser-control probe. The local change skips that probe for an already-stopped
Runtime; the owner crate compiled, but this stopped-Runtime path needs its own
live test before release.

The previously untested stopped-Runtime path now passed a disposable local
smoke against Core `1cde1ea55c5da8478205bfcd93805b6296b5e576` (daemon
SHA-256 `3442a32f7e78ebd7cf072d5e43f839a9951b202e82ada763c12dea3d49039c39`).
A stopped company reconciled to a distinct existing local image, retained a
file in its company volume, and returned a healthy owner principal response.
The test used no model provider and removed its plane, slice and temporary image
tag. This checks a single stopped company with available capacity; it does not
prove pressured upgrades, active owner attachment, simultaneous agents, full
Cloud rollout or production.

The owner machines API now exposes whether a saved home-site change is still
waiting to be applied, so the status survives a page reload. A local smoke with
daemon SHA-256 `1ab3c1fb0374668215365d74cf8d2b4f401486e68a240b8831ee710d2ad8d77a` observed the pending status before and after a plane
restart and its clearance after application. It also preserved the browser
through a synthetic owner lease and resumed a queued job exactly once; cleanup
reported no problems and removed the temporary slice. Frontend static checks
passed. The owner-facing status has not yet been visually tested, and this does
not establish the production resource guarantees listed above.

A further launch-boundary inspection confirms why this prototype is not yet a
release gate: actor capacity is observed and reserved in `staff.rs` before
Work, conversation and Exec start their asynchronous turns, but the actual
local ACP process is spawned later in `acp.rs` without the lifecycle lock.
Two agent starts, or an agent and a browser/job start, can therefore both pass
a headroom observation before their process counts are visible. The lock is
in-process and does not coordinate overlapping daemon generations either. A
fix must preserve each existing Work/turn retry path when admission is deferred
and cover cancellation and daemon replacement; simply returning an error after
a Work Attempt has been claimed could falsely fail work. This is source
inspection, not a reproduced concurrency test.

The launch boundary is broader than the ACP child itself. Local Codex sessions
use a separate Docker exec in `codex.rs`, and both harnesses run preparatory
Docker execs before their long-lived child is spawned. A guard placed only
around the ACP spawn would therefore leave Codex and preparation uncovered.
Conversational Exec, Staff conversation and claimed Work all enter through
different scheduling paths; a capacity wait after claim must retain the
claimed turn and honor cancellation, rather than becoming a failed attempt.
Any cross-daemon coordination must also survive a daemon dying during startup,
without freeing a reservation while its accepted Docker operation is still
creating processes. These are source-level findings, not proof that a proposed
reservation size is sufficient or that a particular coordination design works.

The daemon does already hold `SingletonGuard` on `machine/plane.lock` for its
entire lifetime, before scheduler startup. Two daemon processes using the same
state root cannot run concurrently, so the earlier concern about overlapping
generations needs that qualification. This does not serialize distinct state
roots sharing a workload pool, and it does not cancel a Docker operation already
accepted when the old daemon exits. The replacement daemon can therefore see
an operation still in progress even after acquiring the singleton lock. A
restart test must cover that case rather than treating the lock as a complete
account-wide reservation.

A concurrent-submission local smoke against daemon SHA-256 `85f9cc55175ac5824d6792bf0d7f229b0b703fc6989ab801be8afa572922b68f`
submitted two heavy jobs to one disposable company while its task count was 35
with a cap of 59. Both remained queued without effects, then each applied its
effect once after capacity was restored. Owner entry remained healthy, kernel
process rejections were zero, cleanup reported no problems and the temporary
slice was removed. This does not test agent starts, overlapping daemon versions,
multiple owners, or admission under sustained load after capacity returns.


After merging current main into the prototype, verify-process-pressure.py passed
against clean source 63666e424ff936351599a4035a1b1bed7ee46811 and daemon
SHA-256 0e54e6e6be885f3b4783b20e29bfd2f12f93cb08dc840f067337694c8b0d37b3.
Two jobs queued at 35 tasks with a cap of 59, each resumed once, and owner
entry remained healthy. The kernel reported zero process-cap rejections; the
disposable plane and slice were removed without reported cleanup errors. This
is the same bounded single-company heavy-job scenario, not a test of agent
launches, multiple owners, or the full resource gate.

Queued heavy-job admission now samples shared pressure after acquiring the
account lifecycle start guard, rather than using a sample taken before waiting
for it. This prevents a Runtime or browser start guarded by the same daemon
lock from making that memory observation stale before the job start. Actor
launches still do not share this guard, and Docker operations surviving daemon
exit still need reconciliation; this is not a complete account admission gate.
The daemon built successfully and verify-process-pressure.py passed with binary
SHA-256 b978287f9fac056e69b7563c6181137ee452cdb6c48900c24156a50cf3ded46c:
two jobs waited under process pressure and resumed once, owner entry was
healthy, the kernel reported zero task-cap rejections, and cleanup succeeded.
The fixture does not reproduce the stale-memory interleaving or test agent
launches, multiple owners, or production.

Restart inspection found a second admission boundary that must be addressed
with the launch race: `staff::sweep_orphans` invokes `reap_orphan_sessions`,
which returns zero both when no marked sessions exist and when the Docker
inspection fails. `reap_session` also ignores kill failures; the sweep removes
markers without verifying process disappearance, and startup proceeds after
the sweep. Thus an incomplete observation or a delayed accepted Docker exec
can be mistaken for a clean recovery before new work is admitted. The existing
`verify_session_reaped` helper verifies absence for ordinary turn cleanup but
is not used by this startup sweep. Fixing this requires a retryable, observed
per-owner recovery boundary and a test that injects inspection/kill failures
and late process creation; a global permanent pause would not satisfy owner
isolation. This is source inspection, not a reproduced production incident.

The agent orphan sweep now reports inspection and marker-read failures and
keeps a marker unless the exact session is observed gone. A disposable
company-image container test passed for an unavailable container, an invalid
marker retained for recovery, and an actual orphaned session reaped before
its marker was removed. The daemon build also passed. This improves recovery
visibility and preserves evidence, but staff startup currently only logs the
failure and still proceeds: per-owner admission and retry are not yet wired to
this result. The late Docker-start race remains open.

The prototype now holds the affected company when agent or gate orphan
inspection fails, retries that company's recovery in the background, blocks
its scans, agent admissions and queued heavy-job starts, and wakes scheduling
when recovery succeeds. Other companies retain admission and owner APIs remain
available. Failure to read interrupted Attempts also retains the hold. A
local two-company test using daemon SHA-256
5a193f895527d3c3ea5059a4ca21ce841fe41ad8739c5e81957e8546dc103ff9 injected an
invalid agent marker during restart: the affected company's job waited, the
other company's job ran, and the held job ran once after the marker was
removed; both owner APIs responded and fixture cleanup succeeded. The scoped
registry test passed on the earlier build before the additional database
error branch. This is two companies on one local plane, not multiple production
owners. It does not cover agents launching during pressure, late Docker starts,
gate kill verification, failed Attempt settlement, stopped or replaced
containers during retry, or a database error. Those remain release blockers.

Gate orphan recovery now retains invalid or unverified process-group markers
and checks an observed process listing before removal; an inspection failure
keeps that company on the existing retry hold. The two-company local restart
smoke passed with daemon SHA-256
3aa605db0872b0f18e7f9b32f3fc8e68507f59f61500197c419e5b6d427b21aa. It verified
a real orphaned gate process group was gone, a malformed gate marker kept its
company's queued job waiting after the agent marker was cleared, the other
company's job ran, the held job ran once after correction, both owner APIs
responded and cleanup succeeded. This does not test a signal denied by the
kernel, Docker accepting a delayed start across a restart, process-group ID
reuse, actual agent launch under pressure, multi-owner production, or failure
in Attempt settlement.

Startup recovery now installs per-company holds before reopening scheduling,
then runs at most four company sweeps concurrently. Memory observation and
credential-bearing effect cleanup are included in the affected company's sweep;
failed observations keep that company held for retry. Effect cleanup now checks
signals, verifies no process remains under the effect UID, and requires staging
cleanup to succeed. A local two-company restart smoke using daemon SHA-256
f52b9262f302705a608672982a445f4c78804c05c0f4d1749f9ae2df8312c8cd passed:
it killed a real orphan effect child and gate group, removed valid effect
staging, held one company for malformed effect, agent and gate records while
the other ran a job, then resumed the held job once. Both owner APIs responded
and cleanup succeeded. This does not test actual memory pressure in this
recovery path, provider or multi-owner production, effect calls initiated
outside actor admission, delayed Docker starts, failed Attempt settlement,
stopped/replaced containers during retry, or fairness when all four recovery
slots are busy; these remain release blockers.

The memory part of per-company restart recovery has now been exercised using
real cgroup counters and daemon SHA-256
f52b9262f302705a608672982a445f4c78804c05c0f4d1749f9ae2df8312c8cd. In a
disposable two-company local plane, one company's bounded 340 MiB synthetic
working set exceeded its own soft threshold while the shared pool and peer
remained below theirs. Its recovery and queued job waited; the peer's job ran
and both owner endpoints responded. After the synthetic load was removed,
recovery observed a deliberately invalid agent marker and continued to hold
until it was removed; the queued job then ran exactly once. Cleanup reported
no problems. This is not a production or multi-owner test and does not address
the remaining agent-start, delayed Docker-start or Attempt-settlement gaps.
