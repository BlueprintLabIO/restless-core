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
