# Sprint 113: faster teams

**Status: planned, not started (8 Nov 2026 AEDT). Part C needs an owner decision first.**

The owner's question was fair: team members look like sub-agents of the lead, so why are they
slower? PRs #709 to #715 halved team-work time without changing tokens, but missed both targets.
This sprint covers what is left: ready Work that sits behind a busy lead, the cost of starting each
turn, and the cost of each handoff.

## Where we are

Overhead bench (#708, #710), final merged main, seconds. Model latency in the bench is short and
modelled, so these numbers show overhead and overstate its share of real, minutes-long work.

| Measure | Now | Target |
| --- | --- | --- |
| Team work vs the sub-agent baseline (tasks d, e) | 2.48×, 3.52× | about 2× |
| Single tasks vs solo (a, b, c) | 1.35× to 2.23× | about 1.3× |
| Owner message to first model call | 3.2 to 3.4 | 1 or less |
| Result to Exec wake | 4.7 to 4.8 | 2 or less |
| Work to next Work | 4.6 | 2 or less |

Task a's 1.84× in the final table was taken on a loaded host. A quiet run gave 1.35×.

Turn start is about 3.2 s, mostly `omp`: about 1.3 s to initialize and 0.8 s to load the session.

## A. Ready Work is never starved by conversation

**Problem.** `claim_ready_work_excluding` skips any owner with a running Attempt, a live cognitive
lease, or a place in the registry's running actors. Each scan serves conversations before claiming
Work, so steady mail to a lead can hold that lead's own ready Work back indefinitely. On Reno,
"Restore the concept-site content" stayed proposed this way, and nothing said why.

**Build.**

1. Every ready Work that is not running shows why it is waiting: owner busy in conversation, no
   free actor slot, not enough memory, or waiting on another Work. Derive it from the same facts
   the scan uses, so the label cannot disagree with the scheduler. It is a sub-row under a
   fixed-height row, and the row does not grow.
2. Ready Work that has waited past an age limit (start at 2 minutes) takes the owner's next free
   turn ahead of Staff and lead mail. The owner's own messages still go first. Mail stays durable
   and unread, and wakes the actor when the Attempt ends.
3. No pre-emption. A running turn is never interrupted.

**Done when.** Under a scripted stream of mail to a lead, its ready Work starts within one turn end
after the age limit. An owner message still beats that Work. One actor never has two processes (the
migration 0039 unique index holds). No mail is lost or delivered twice. The waiting reason is
checked live in the browser on a local stack, including the row height.

## B. Keep each actor's agent process warm

**Problem.** Every turn starts a new `omp` process. `omp` reads its system prompt at launch
(ACP has no system-prompt field), so it cannot start before the context is ready.

**Build.**

1. **Spike first, one day at most.** Establish how each credential reaches the process today:
   `issue_actor_session`, `issue_tool_session` and `issue_model_session`. MCP servers travel through
   `session/new`, so the tool session probably moves per turn already. The model session is the
   open question. Measure idle memory per warm process as well.
2. Keep one process per actor between turns, keyed by actor and a hash of the launch contract:
   system prompt, tools, model and image. Any change to the hash retires the process.
3. Keep credentials turn-scoped. A warm process receives them at turn start and holds nothing valid
   between turns. The gateway refuses an expired turn's credential, as it does today.
4. Idle limit starts at 60 seconds. Warm processes count against the workload memory pool and are
   the first thing evicted under pressure. A company update drains them. A crashed warm process
   falls back to a cold start without failing the Attempt.

**Done when.** Owner message to first model call is 1 s or less on a warm actor, and cold starts are
no slower than today. A test shows a credential from an earlier turn is refused. Warm processes do
not push a real company past its memory limit, checked at Reno's scale. Tokens are unchanged.

**If the spike finds model credentials cannot move per turn without weakening the turn boundary,
stop and bring the options back to the owner.** Security wins over latency here.

## C. Cheaper handoffs for small jobs (owner decision needed)

**Correction.** The earlier suggestion was to let small jobs run as in-process sub-agents. Restless
deliberately forbids this: ADR 0003, the skill contract, `OMP_AGENT_TOOLS` and the test
`omp_cannot_create_invisible_staff`. A private sub-agent has no actor, Work, message or supervised
process, so the company cannot see who did the work.

**Recommended: keep one delegation canon and make it cheaper.** Every job stays a Work Attempt.

1. Coalesce result wakes: if a lead will wake soon anyway, deliver the results in that turn rather
   than paying for another.
2. Start the next Work in a chain as soon as its dependency completes, without waiting for a scan.
3. Add a measured guide for when a lead should do a job itself. Exec already does (4 Nov), so small
   asks need no handoff at all.

**Alternative, needs a new ADR:** visible lightweight sub-agents, each recorded as a Work Attempt
but running inside the lead's process. This is faster, but one crash takes down both, they share
one credential scope, and review independence is weaker. Not recommended unless A, B and C1 to C3
still miss the target.

**Done when.** Team work is about 2× the sub-agent baseline or better, result to Exec wake is 2 s or
less, and every job still has an attributable Work and Attempt.

## Order and proof

A first: it is the cheapest and it is the delay the owner actually saw. Then the B spike, B, and C.

Each part reports before and after on the overhead bench on a quiet host, plus token totals, which
must not rise. #715 is not exercised by the bench because its host has no workload pool, so use the
`actor start waits for capacity` log line on Reno to confirm the real reason for any remaining wait.

Out of scope: changing CPU or memory actor slots, and the `local-ci` shared cargo cache fix, which
is tracked separately.
