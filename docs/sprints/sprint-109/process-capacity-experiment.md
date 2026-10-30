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
