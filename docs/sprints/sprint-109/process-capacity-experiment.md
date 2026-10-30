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
