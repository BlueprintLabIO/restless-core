# Coordination lab results

Date: 2026-08-21
Model: `anthropic/claude-sonnet-4-5` for every actor
Seed: Cosmon commit `514b7b3d0a65e093af608b08ca142344412181f4`
Owner input during runs: none
Production Restless code or company state changed: none

## Outcome

The seven model-facing commands are sufficient for the coordination patterns observed in this
scenario:

- `send`
- `commission`
- `redirect`
- `report`
- `request_judgement`
- `decide`
- `schedule`

The experiment did **not** reveal a missing eighth organisational command. It revealed that commands
alone do not establish execution boundaries. The 10x change is to combine the small command surface
with enforced Work leases, callback-driven turn boundaries, a single-writer coordinator, and a
single-writer integration path.

The v1 change was materially better than v0: three Staff Attempts started while the original Exec
turn was still active, the Exec was unable to implement project code, bounded turns enabled recovery,
and one independent review completed through a clean commission -> Attempt -> commit -> gate ->
report -> wake path. The run still failed overall because ownership, terminal callbacks, integration,
and persistence were not enforced strongly enough.

## Experimental change

v0 treated the initial Exec ACP turn as a scheduler lock. Although `commission` durably created Work,
the scheduler could not claim it until the Exec turn returned. Exec instead entered the Staff
worktree and implemented the delegated outcome itself.

v1 made one conceptual change: **Work ownership became an execution lease**.

- Exec received read/search project tools only.
- The scheduler continued dispatching while the Exec turn was active.
- A `commission` wake could immediately create the named Staff Attempt.
- Actor turns had an eight-minute upper bound.

The file and integration sides of that lease were not yet enforced for Staff. That omission produced
the most useful v1 failures.

## Run comparison

| Measure | v0 | v1 |
| --- | ---: | ---: |
| Accounted model cost | $1.69 | $9.09 |
| Actor turns | 1 | 15 |
| Exec tool calls before first usable callback | 78 | Staff launched after 3 commissions |
| Work records | 1 | 7 |
| Attempts | 0 | 9 |
| Produced Attempts | 0 | 1 |
| Unknown Attempts | 0 | 8 |
| Clean governed artifacts | 0 | 2 references from one review Work |
| Valid direct JSONL records | 644 | 3,357 |
| Corrupt direct JSONL records | 0 | 37 |
| Run-complete decision | no | no |

Combined accounted model cost was **$10.78**, below the approved $30 combined ceiling.

v0 was controller-stopped after its dominant failure was established. Its sole Exec turn ran for
more than ten minutes, made 78 tool calls, spent $1.69, commissioned one trainer Work item, never
yielded, created no Staff Attempt, and then wrote 613 staged lines plus further fixes into the Staff
worktree.

v1 ended when the scheduler observed:

```text
sqlite3.DatabaseError: database disk image is malformed
```

After all cross-runtime writers exited, `PRAGMA quick_check` returned `ok`. The likely failure is the
storage topology: the host scheduler and container MCP processes opened the same WAL database through
a macOS-to-Linux bind mount. The database recovered, but that topology is disqualified. Direct
multi-process JSONL appends independently produced 37 malformed records.

## What worked

### Immediate asynchronous dispatch

In v1 Exec commissioned three Work items in roughly the opening 90 seconds:

- Prism Caverns and mini-boss -> `world-content`
- trainer and rival battles -> `gameplay-systems`
- spacecraft and story presentation -> `experience-presentation`

All three Staff Attempts began while the originating Exec turn was still active. This validates the
event-driven scheduling direction and disproves the v0 assumption that an actor turn must complete
before its emitted wakes can be handled.

### A complete generic callback path

The independent critic successfully:

1. claimed Work `work-766f6dc773`;
2. evaluated the runnable seed and the directive;
3. committed `CRITIC_ASSESSMENT.md` at
   `6cf69d67ef110569c9c8fe888f790c37bfd21c04`;
4. passed the declared `test -f CRITIC_ASSESSMENT.md` gate;
5. called `report(outcome_met)` with evidence;
6. caused the Attempt to become `produced`, Work to become `completed`, artifact references to be
   stored, and Exec to be woken.

The assessment found a strong technical base but only about 15-20% of the requested 60-90 minute
commercial slice: one of three biomes, no trainers/rival/bosses in the reviewed seed, no story
chapter, no working exploration-ability loop, and no spacecraft.

### Useful production existed despite coordination failure

The independently materialised candidates were real:

- trainer candidate: **16/16** headless browser checks passed;
- spacecraft candidate: **14/14** headless browser checks passed;
- critic artifact: committed, clean, and gate-valid.

This is important: the problem was not model inability to build. It was failure to terminalise,
connect, and integrate otherwise useful work.

### Repair was the right generic recovery primitive

After an integration Attempt ended `unknown`, Exec used `redirect(repair)` on the same trainer Work.
Revision increased, the worktree was preserved, concrete feedback was attached, and a new Attempt
resumed. No special retry or resume command was needed.

## Failure modes

### 1. A command mutation did not imply a turn boundary

v0 showed the basic failure. Exec commissioned Work, remained inside the same open-ended ACP turn,
looked for synchronous worker activity, and implemented the outcome itself. The scheduler was waiting
for that turn to finish.

v1 allowed dispatch during the Exec turn, but Exec still polled canonical state, sent status messages
to Staff already inside fixed contexts, abandoned running Work after about two minutes, and opened an
owner judgement asserting that workers were stalled while their worktrees were actively changing.

Required behavior: a coordinator wake is a bounded reaction to events, not a place to wait. Once it
has delegated all useful Work, it quiesces until a callback. `send` to a running actor is queued for a
future wake and must not imply live conversational delivery.

### 2. Work ownership was metadata, not a complete runtime boundary

Exec's read-only v1 posture prevented the v0 self-implementation failure. Staff, however, could still
read and write outside their claimed worktrees. Recovery workers therefore mutated the shared root
repository and refs.

Required behavior: an Attempt lease binds actor, Work revision, worktree, and write scope. The
persistent company Runtime remains; this is not a disposable per-turn sandbox. The process simply
receives a persistent Work workspace and cannot write the integration checkout or another Work's
workspace.

### 3. Terminal callbacks were cooperative and frequently omitted

Three initial producers and two integration producers ended or timed out after substantial tool use
without `report`. Some left dirty worktrees; one made a meaningful spacecraft commit but left
generated package files untracked. The harness correctly marked these Attempts `unknown`, but a large
amount of useful work became difficult to consume.

Required behavior: before an Attempt process may exit, the runtime performs one preparation check. If
the Attempt is still running, it issues a bounded final continuation containing exact observed state:
dirty files, current commit, failed gates, and the requirement to commit/clean/report or truthfully
report blocked. A second failure remains `unknown`; the substrate must never invent success.

### 4. Running leases had invalid transition semantics

Exec could immediately `redirect(abandon)` Work with a running Attempt. The worker continued, because
there was no cancellation acknowledgement or lease expiry. A later terminal callback could then
overwrite the Work status.

Required behavior: `redirect` remains the generic command, but its semantics depend on state. Against
a running Attempt, repair/reassign/abandon first requests cancellation and records the pending
transition. It becomes effective only when the worker acknowledges, the process exits, or the lease
expires. Stale callbacks carry actor + Attempt + Work revision and cannot mutate a newer revision.

### 5. Recovery lost graph provenance

Exec commissioned new “integration” Work with no `requires` or `revises` edges, referring to old
worktree paths only in prose. Staff often recreated files rather than consuming an explicit commit
artifact. This increased cost and ambiguity.

Required behavior: if an outcome is a continuation, use `redirect(repair)` on the same Work. If it is
integration, commission new Work with explicit `requires` edges to produced artifacts. The expected
artifact describes the result, not an ad hoc absolute-path transfer protocol.

### 6. Integration had multiple writers and no canonical convergence

Trainer and spacecraft recovery Work both said “merged to main.” The two Staff actors concurrently
wrote the shared repository:

- `master` became trainer merge `ef336bdff10b6e5f68a3671213157eca547f6816`;
- the root checkout's detached `HEAD` became spacecraft merge
  `6cd3666fa44906355231323f1ae3d82d754a7dfa`;
- their merge base remained the seed `514b7b3...`;
- `HEAD` lacked `js/trainer.js`, while `master` lacked `js/spacecraft.js`.

Both candidates passed independently, but there was no combined candidate.

Required behavior: producers end at clean commits. Exec then commissions one dependent integration
Work with all producer Work in `requires`. A single integrator holds the integration-branch lease,
resolves conflicts, runs combined gates, and reports one reviewable commit. Producers never merge.

### 7. The persistence and telemetry topology was unsafe

The host and container opened the same bind-mounted SQLite WAL directly. Concurrent actor processes
also appended to one JSONL file without a serialising writer. The result was a scheduler-stopping
SQLite read error and malformed JSONL.

Required behavior: one coordinator daemon owns the database and outbox on one native filesystem. MCP
clients call it over a local socket/HTTP boundary. The same daemon serialises trace events. SQLite is
still an ordinary mutable store; this does not introduce an append-everything ledger or a workflow
engine.

### 8. Idempotency was not real

The server generated a fresh command ID for every MCP invocation. Retransmitting the same logical
request therefore created a new mutation. The preflight only proved that a duplicated database key
was unique, not that a client could safely retry a command.

Required behavior: every mutation accepts a caller-generated idempotency key scoped to company,
actor, and Attempt. The coordinator stores request hash + result atomically and returns the original
result on a retry. A reused key with different input is rejected.

### 9. Owner judgement was used to compensate for coordination confusion

Exec requested owner direction despite the no-mid-run contract and despite the choice being
reversible. It then recorded a contradictory direct-implementation decision without resolving the
open judgement.

Required behavior: context assembly clearly distinguishes reversible operating decisions from
irreducible owner judgement. `decide` should resolve or supersede an open request on the same subject;
an unresolved request cannot coexist silently with a contrary organisational decision.

### 10. Accounting and executable escape needed substrate guards

Before counted v0, Exec invoked a nested `omp` vision call from the shell, bypassing actor accounting
and the all-Sonnet condition. The run was stopped at zero counted harness cost. The lab then shadowed
nested agent CLIs, rejected nested-model ACP permission requests, and added bounded actor leases.

Required behavior: model credentials and agent executables are not inherited by ordinary tool
children. Live spend is accounted by the gateway, not inferred only after a turn returns. Per-turn
leases limit overshoot but do not replace a shared run-level spend reservation.

## 10x harness design

The next scratch harness should keep the seven commands and replace their substrate:

### 1. Single-writer coordination service

- One process owns SQLite, transactions, the outbox, command idempotency, and compactable operational
  events.
- Actor MCP servers become thin clients over a Unix socket or loopback HTTP endpoint.
- No process inside the company container opens the database file.
- One trace sink receives chronological tool/text/coordination events and serialises them.
- Startup reconciles undelivered outbox rows and running leases after a crash.

### 2. Explicit execution leases

- `commission` creates Work and workspace atomically, then a substrate claim creates an Attempt lease.
- Lease identity is `{actor, work, revision, attempt, workspace}`.
- All Attempt-scoped commands validate that identity.
- Exec receives a read-only organisational posture.
- Staff receive write access only to their persistent Work workspace.
- One separately commissioned integrator receives the integration branch lease.
- Lease cancellation/expiry makes late callbacks stale rather than authoritative.

### 3. Event-driven actor lifecycle

```text
durable event(s)
  -> assemble bounded delta + canonical references
  -> one actor lease
  -> mutations and/or production
  -> mandatory quiescence or terminal callback
  -> durable outbox wakes affected actors
```

The scheduler continues while turns run. A coordinator does not poll delegated Work. A message to an
active actor is inbox state for its next wake. Schedules are only for genuinely time-driven events.

### 4. Terminal preparation without fake completion

On attempted process exit with a running Attempt:

1. observe Git status, current commit, declared gates, and callback state;
2. provide one bounded finalisation continuation;
3. require `report(outcome_met|blocked|abandoned)`;
4. if it still exits, mark `unknown` and wake the accountable coordinator.

The coordinator, not the model, runs declared gates. `outcome_met` remains a claim until gates and
independent review support it.

### 5. Single integration Work

```text
producer A report(commit A) --\
producer B report(commit B) ----> integration Work -> combined gates -> review Work
producer C report(commit C) --/
```

This uses ordinary `commission`, `report`, and `requires` edges. It needs no universal artifact state
machine and no bespoke merge command.

### 6. Comparison contract

Before another full commercial-brief run, exercise deterministic fault cases:

- duplicate command delivery returns the original result;
- stale callback after repair is rejected;
- agent exits dirty and receives one finalisation continuation;
- coordinator restarts with running Attempts and undelivered outbox;
- three commissions dispatch concurrently without trace/database corruption;
- one actor cannot hold two active execution leases accidentally;
- two producers cannot write the integration checkout;
- integration cannot start until all required Work is produced;
- spend reservation prevents concurrent turns overshooting the run ceiling;
- no owner judgement is created for reversible operating choices.

Then rerun the same seed and directive. Measure:

- commission-to-Attempt latency;
- proportion of Attempts with terminal callbacks;
- clean committed artifact rate;
- gate-pass and independent-review rate;
- stale/duplicate mutation count;
- integration convergence to one candidate commit;
- owner interruptions;
- cost and wall time per produced outcome.

## Recommendation

Do not add these scratch structures to production OrgIntel yet. Build the single-writer, lease-enforced
v2 lab first and rerun the deterministic fault suite plus this scenario. If it converges to one
integrated candidate with no persistence errors and materially higher terminal-report rate, record the
proven command semantics in `docs/specs/orgintel.md` and schedule the smallest vertical production
slice. The evidence supports the seven-command surface; it does not yet support this implementation.
