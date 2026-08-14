# Sprint 02 run report

**Status:** in progress — T4's comparison is executing.
**Question the sprint exists to answer:** does an Exec-led organisation beat one
strong agent on the same mission, given the same model, tools, budget and time?

---

## What landed

| Ticket | Commit | How it was verified |
|---|---|---|
| T2 · `restless spawn` as a tool | `a66f689` | T9's staff machinery ran for the first time: refusals returned synchronously, worktree created, staff commit `2e42a90` on `staff/probe-staff` |
| T7 · Party-level repeat detection | `3343b18` | Greg's real scenario — flagged across two honest keys, **not** flagged on replay |
| T8 · Organisational health signals | `206c0b2` | caught its own false positive against real receipt data before shipping |
| T3 · Three org modes + harness | `ddfb545`, `2b77eda` | mode gating verified in both directions |
| Purge · model gateway | `4f20054` | **2,999 deletions**; 102 spend records migrated and verified |

## Findings so far

### Sprint 01 was running `minimal_team`, unlabelled

The three-mode design did not have to invent its independent variable. Staff
have always been handed a task string and nothing else — no mission, no idea
what else is in flight, no way to raise a blocker. That is `minimal_team` by
the evaluation spec's own definition. So sprint 01 was not running a degraded
OrgIntel; it was running the middle baseline without knowing it, and reporting
the result as the product.

`orgintel` mode is the first time a worker is told what company it works for.

### Event-driven wakeups fire — proven by accident

A throwaway one-line-comment probe completed, its commitment completed, and the
scheduler woke the Exec:

```
scheduler wake company="cosmon"
  reason="event: commitment completed by staff-probe-staff: ..."
```

Sprint 01 listed this as an open question ("do event-driven wakeups actually
fire, or does the Exec go silent after a dependent result lands?"). They fire.
The staff→Exec trigger chain works end to end.

### Unknown is not failure — the same bug, twice, in one day

`classify_turn` fell through from its error path to its consumption check,
reporting a 20-minute work-turn boundary as "the model never ran, check
provider credit". Fixed in the morning.

By the afternoon the organisational health signals had grown a second, subtly
different definition of "failed" — an allowlist of success words that
classified `deployed` and `refunded` as failures. It would have accused a
healthy company of repeating a failed approach every time it deployed.

Both are the same error: **treating unknown as negative.** The fix is now
structural — one three-state `Outcome`, shared by reconciliation and health,
pinned by a test over every status word both companies have actually emitted.
A second definition of a shared predicate is the duplicate-ownership problem
the cross-layer contract forbids, appearing inside a single service.

### The company had the evidence and did not draw the conclusion

Aris's own journal, from its own run:

> *DURABLE LOSS: P1 Priya — price vs incumbent Bond/CGP stack; concedes quality,
> not value.*
> *P8 Simon — CEM/Devon; wants a CEM product (does not exist).*

A price rejection against an entrenched incumbent, and a segment mismatch. That
is the raw material for "we are selling to the wrong buyer" — and Aris recorded
both faithfully, then carried on selling to parents. It never asked whether the
segment was wrong.

The gap is not knowledge: the same model class produced a full channel analysis
when asked directly. The gap is that **nothing in the system revises strategy
from accumulated evidence.** The company executes, reports honestly, and audits
itself — but never reconsiders.

That is §3.2 self-exploration and §3.4 self-evolution, the two capabilities
`orgintel` §1.1 says OrgIntel *is*, and the two we have built nothing for. It
also sharpens where the human's involvement is by design and where it is the
product failing:

- owner supplies **identity, sign-off, taste, irreducible judgement** → working
  as intended;
- owner supplies **strategy, segment choice, channel selection** → the product
  not yet doing its job.

Worth stating plainly: we have never once asked a company to choose its own
strategy. Every run handed it a mission and measured execution. That is a gap in
what has been *tested*, not yet a verdict on the product.

### Unreachable concepts are not dead code

The purge scan found `add_goal`, `add_decision` and `add_artifact_ref`
uncalled. They are storage for three concepts §3.1 assigns to OrgIntel as
authoritative, with **no write path for any actor**. Recorded as a gap rather
than deleted — purging them would silently narrow the ontology.

### Operating evidence from every run so far

Numbers from the spend spool and event streams, not from any agent's account.

| | calls/turns | total | produced |
|---|---|---|---|
| sonnet-4 via the proxy | 91 calls | $4.76 | a partial Cosmon and a **false** verification claim |
| glm-5.2 via ACP | 13 turns | $18.26 | 3D creature-collector (12 species, 6 evolutions, combat, 2 biomes, trainers, mini-boss), a sales loop with 9 charges, an operating loop with 90 receipts |

Average glm turn: **96,214 tokens, $1.40**.

**Context does not grow without bound.** Per-turn utilisation of the 1M window:

```
thymelake   79k (7%)  → 130k (13%)
aris        63k (6%)  → 133k (13%) → 70k (7%)
```

It rises and falls with the size of the plan plus the *latest* journal entry —
aris's 133k turn was its long reconciliation journal, and the next turn dropped
back to 70k. Shipping only the latest journal is thin institutional memory, and
it is also exactly what bounds context. That is a trade-off to make knowingly,
not a defect to fix.

**Every blocked wake in two sprints was a platform defect, not a stuck company.**
Aris's two blocks were the effect surface having no discovery, and the health
gate's own misclassification of a work-turn boundary. Cosmon's were the same
misclassification. Both are fixed. No company has yet been blocked by its own
business problem — which is either the gate earning its keep by catching our
bugs, or evidence that the companies are more robust than the platform. Probably
both.

**The party-repeat guard has fired once**, on the deliberate test. Nothing
organic has tripped it yet, so it is verified but not yet load-bearing.

---

## T4 — the comparison

**Scenario:** [`lumaara-biome`](../../scenarios/lumaara-biome.md) v1 — add Prism
Caverns plus a trainer beat to the Lumaara slice, leaving it playable.

Held identical: model `zai/glm-5.2`, $15 ceiling, image, starting commit
(`514b7b3` groundwork atop `a01bd22`), wake reason, capabilities.

### Predictions, recorded before the run

- `single_agent` completes the contract — every sprint-01 artifact came from it.
- `minimal_team` is **worse** than `single_agent`: context-free workers collide
  and integration costs more than it saves.
- `orgintel` beats both on elapsed time, not on cost.

### Results

| Mode | Termination | Tools | Staff | Cost | Elapsed | Owner acceptance |
|---|---|---|---|---|---|---|
| `single_agent` | `done` | 192 | 0 | **$6.35** | **37m** | **PASS** — contract met in full |
| `minimal_team` | `done` | 137 | **0** | $4.49 | 46m | **PASS** — contract met |
| `orgintel` | PENDING | | | | | |

Acceptance is manual, against the scenario's success contract. Numbers do not
substitute for loading the build (`evaluation-dogfood` §21.2, §25 rule 2).

#### `single_agent` — verified, not relayed

Commit `b3d236e`, 22 files, +1090/−90. It claimed "23/23 green", which is the
exact shape of claim this project has already been burned by, so the verifier
was re-run independently rather than believed:

```
[PASS] #10 bonding refused in trainer battle
[PASS] #11 trainer sends out next creature on a down
[PASS] #14 warden battle is a scaled mini-boss :: Lv8 218hp
[PASS] #15 warden defeat unlocks the gate (flag, warden removed, disc lit)
[PASS] #16 gate teleports player into the cavern :: biome=cavern
[PASS] #23 zero errors across the whole run
23/23 steps passed; errors observed: 0
```

The claim held. Note the verifier could not run until its HTTP server was
restarted by hand — the reaper had correctly killed it at the wake boundary,
which is the process-leak fix working as designed.

**This sets a high bar.** The baseline did not merely finish; it delivered
trainer battles with real mechanics, a scaled mini-boss gating traversal, and a
full biome transition with atmosphere and bounds changes, for $6.35 in 37
minutes with zero owner interventions.

#### `minimal_team` — the middle arm collapsed into the baseline

`done`, commit `d75ae16`, 20 files, +837/−62. Verified independently: **18/18, 0
errors** — boss battle with bonding disabled, cavern unlock, portal transition,
cavern-specific creatures, bond offered in-cavern, exit portal.

**It did not delegate.** `restless spawn` was available to it — that is the only
thing separating this mode from `single_agent` — and the Exec never called it.

That is the sprint's most important result so far, and it is not the one the
harness was built to measure. Sprint 02 diagnosed three runs of
`spawn_requests: []` as a *plumbing* failure: delegation was an end-of-turn
envelope field rather than a tool, so an agent working naturally wrote briefs
instead. T2 fixed that, and was verified working. Given the tool, at the moment
of decision, on a task the Exec had itself previously decomposed into two
parallel workstreams — it still chose to do the work alone.

So the middle arm is not a distinct configuration. It is `single_agent` with an
unused affordance, and the two runs differ only by noise:

| | tools | cost | elapsed | lines |
|---|---|---|---|---|
| `single_agent` | 192 | $6.35 | 37m | +1090/−90 |
| `minimal_team` | 137 | $4.49 | 46m | +837/−62 |

Cheaper, slower, less work, same verdict. Nothing here separates them.

Two readings remain open until `orgintel` lands. Either delegation genuinely is
not worth its overhead at this size of task — a real and useful finding — or
the Exec will not reach for it unless the context makes the case, in which case
`orgintel` mode (which tells a worker what company it works for) may be the
thing that tips it. If `orgintel` also declines to delegate, the honest reading
is the first.

#### `orgintel` — invalidated by provider credit, not by performance

The third arm blocked at $2.15 with "exec produced no parseable termination
decision twice". The cause was not the mode:

```
429 [1113][Insufficient balance or no resource package. Please recharge.]
```

The zai account ran out mid-run. The same failure then took the two chained
follow-up runs with it, in seconds.

**The comparison was therefore re-run in full on `moonshot/kimi-k3`**, not
patched by re-running one arm. Adding a Kimi `orgintel` to two glm arms would
have violated `evaluation-dogfood` §25 rule 3 — baselines must receive identical
models — and produced a number that looked like an answer. The glm results are
archived under `lumaara-biome-results-glm/` as a valid two-arm comparison; the
Kimi run is the one with three.

#### Three defects the credit exhaustion exposed

Worth more than the run it cost, and none reachable from a test:

1. **Provider errors arriving as message *content* bypassed the health gate.**
   The agent runtime streams the upstream body through, so a 429 arrived as
   assistant text: the turn succeeded, tokens were consumed, and the gate saw
   nothing wrong. Three companies then blocked with "no parseable termination
   decision" — **F1 from sprint 01 in a new costume**, six weeks of machinery
   later. The gate reads transport; this one came through as speech. The
   termination parser now runs its raw text through the same deterministic
   classifier before blaming the model.

2. **A poisoned company could not be un-poisoned.** An unaccountable turn
   poisons fail-closed, which is correct — but a credit exhaustion produced
   usage with no cost and bricked two healthy companies permanently, for an
   outage they did not cause. `restless clear-poison` appends a cancelling
   record; both records stay in the append-only spool so the incident is still
   legible. Aris and Thymelake restored to $1.97 and $2.78.

3. **A provider's default host is not always the one a plan is served from.**
   A Kimi For Coding key 401s against `api.moonshot.ai` and works against
   `api.kimi.com/coding/v1` — indistinguishable from a dead key without a
   base-URL override. It cost four probes and one wrong conclusion earlier in
   the sprint ("the key is dead").

### Kimi re-run — `single_agent` (verification caught a subtly false claim)

`done`, 158 tools, **62m**, **$12.55**, commit `69cb6a6`, 25 files, +1366/−69.

It reported *"67/67 harness checks green, zero errors."* Running all four
harnesses independently:

```
verify-battle.mjs          12 pass, 1 FAIL
verify-combat-extra.mjs     7 pass, 0 fail
verify-loop4.mjs           19 pass, 0 fail
verify-roster-evolution.mjs 29 pass, 0 fail
TOTAL                      67 pass, 1 fail
```

**Correction — this finding was withdrawn on inspection.** The initial reading
was "67 is the pass count, not the score; the company elided a regression."
That accusation does not survive looking at the failure: it is a `TimeoutError`
waiting for *fonts to load* in headless Chromium, and it was produced while
another mode's wake was running on the same machine and competing for CPU.

In other words the one failure is very likely **my measurement artifact, not the
company's regression** — I ran a browser-based harness under load I had created,
then blamed the agent for the result. Re-verify on an idle machine before
claiming anything. The lesson is the one this project keeps relearning, applied
to me this time: *a claim is only as good as the conditions it was measured
under*, and I did not check mine before writing it down.

What stands unaltered is the value of re-running claims rather than relaying
them. It has changed the meaning of a claim three times today — twice in the
company's disfavour, once, here, in mine.

It also claimed to have *"left the game running on ports 8124/8231."* It had
not: the container held only `tini` and `sleep infinity`. The reaper had killed
both servers at the wake boundary. The agent's account of the world was wrong
and the platform's was right — which is the argument for reconciliation in one
sentence.

### The health gate is right in principle and keeps being wired wrong

Four times in one day the system misattributed an environmental failure to the
agent, and the same generic check was involved in three of them:

| | What happened | Reported as |
|---|---|---|
| morning | `classify_turn` fell through its error path to its consumption check | "the model never ran — check provider credit" |
| morning | a 20-minute wall-clock bound killed a live turn | same |
| afternoon | org-health grew a second definition of "failed"; `deployed` counted as a failure | would have accused a healthy company of repeating a failed approach |
| evening | the watchdog halted a 50-minute turn after 122s of quiet; the no-op check **overwrote its verdict** | "the turn consumed no tokens — the model never ran" |

Each fix was correct in isolation. The pattern says the design is wrong: the
no-op check has **three call sites** and should have one chokepoint. A predicate
that means "this turn produced nothing" must not be evaluated by whoever happens
to be holding the outcome — a halt, an error, and a normal completion each know
something the generic check does not, and each has now silently lost it once.

**Carry into sprint 03:** one function owns turn classification, and every path
routes through it with its own evidence attached. Do not patch a fourth door.

### What the day's invalidated runs actually cost

Three comparison attempts were thrown away — roughly two hours of compute — and
in every case the cause was our instrumentation rather than the companies:
a daemon restarted under a live wake, a browser harness run under CPU load I
had created, and a liveness threshold tuned against one model applied to
another.

That is worth stating plainly rather than burying, because it is the sprint's
most repeated lesson in a different costume: **the measurement apparatus is part
of the system under test.** The test suite passed green throughout all three
failures.

### Reading

PENDING — `minimal_team` and `orgintel` still running. If `orgintel` does not beat `single_agent` on any dimension, the honest
reading is that organisational intelligence does not earn its overhead at this
size of work — recorded as the finding, not reinterpreted as success
(§25 rule 10).
