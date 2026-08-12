# LLM_CURE — failure modes and their cures

**Read this before designing or coding.** `ARCHITECTURE.md` says what to build. `CLAUDE.md` says how we
operate. This file says how we think, and how we avoid the specific ways coding agents reliably go
wrong on this repo.

These are observed failure modes of the agents actually working here, not generic advice. They are
recorded because muscle memory will otherwise reproduce them every session.

The whole file compresses to one line:

> **Name the risk and accept most of them. Classify the problem before choosing the tool. Branch, run,
> then purge to one canon.**

---

## Part 1 — The three frames

Diagnostics for design and review. Not gates, and not labels required on every function.

### Frame 1 — Name risks and give each a disposition

**Paranoia is what unnamed risk turns into.** An agent that cannot articulate a specific risk hedges
against all of them at once. That is where over-engineering and premature governance come from — not
from caring too much, but from never naming what it is you are worried about.

Name the risk. Then give it exactly one disposition:

| Disposition | Meaning | Cost |
|---|---|---|
| **Accepted** | We know. We are fine with it. No code. | Free — recorded, not ignored |
| **Pending fix** | Real, not now. Tracked in the friction backlog (§16.4). | A line in a backlog |
| **Guarded** | Mitigated but still possible: detection, recovery, limits, restart. | Code |
| **Invariant** | Structurally impossible. | Code, plus rigidity, plus it blocks valid work when state is stale |

**Default to accepted.** Escalation takes an argument, not a feeling.

**Invariant is reserved for irreversible harm.** That is the entire reason the kernel is small
(ARCHITECTURE.md §2.5, §2.7, §3.3). Prefer recovery over prevention: if restart, replan, Git, or
snapshot restore fixes it cheaply, it is guarded at most (§8 item 4, §16.6).

**Citations, not justifications.** New governance, state, or entities require naming the run where
their absence caused a real problem. A justification can be generated; an observed failure can be
checked in one glance.

**Dispositions are revisitable.** An accepted risk that actually fires gets promoted. A register that
is never revised rots into a list of things we once said were fine.

Accepted risks should carry their **expiry condition** where one exists — the event that changes the
disposition, not a project phase we choose. "Accepted because nothing real is connected" expires the
moment something real is connected.

This frame applies to all risk — technical, product, operational — not only safety.

### Frame 2 — Classify the problem before choosing the tool

Regex where judgement belongs, and a state machine where one does not fit, are **the same error:
misclassifying the problem.** The machinery is downstream of the misread. Arguing about the machinery
treats the symptom.

Classify on two axes:

- **Deterministic or judgement** — is the answer computable from the inputs by rule, or does it depend
  on meaning, context, and degree?
- **Enumerable or open-ended** — can the situations be listed in advance, or not?

| | Enumerable | Open-ended |
|---|---|---|
| **Deterministic** | State machine, enum, match — where a state machine is *correct*: container lifecycle, process supervision, effect status | Compute, do not enumerate: spend accounting, idempotency keys, Git operations |
| **Judgement** | **Model call returning one of N.** Nearly all the damage happens here | Model call, open output: directive → milestone, staffing, run reports |

**Judgement + enumerable is the cell that goes wrong.** "Is this commitment blocked, or just quiet?"
has a finite *output*, so it looks classifiable by rule. Reading finiteness off the output and
concluding the problem is deterministic is the single misread that produces both regex and the
spurious state machine — a tool matched to the output shape while missing the nature of the input.

**The smell is broader than regex: any function that takes content and returns a decision.** Regex,
`contains()`, keyword lists, score thresholds, enum classification of free text. Banning the word
"regex" alone just produces `contains()`.

**Cost is not a reason.** A model call in the hot path is acceptable. Correct judgement beats latency.

**Judgement paths are tested behaviourally, not exactly** (§16.8). "This path has no deterministic unit
test" is an expected state, not a gap to be closed by making the path deterministic. Do not convert an
open question into a closed one in order to assert a green test.

**Quadrants move.** After enough runs a stable pattern can make part of a judgement problem
deterministic, and rules that stop helping move back — §4.10's ladder runs in both directions:

```text
ad hoc → repeated pattern → playbook → tooling → invariant, and back again
```

A classification is a current reading, not a property.

### Frame 3 — Branch, gather evidence, then purge to one canon

Tunnelling and accumulation are **one loop failing at opposite ends.** Tunnelling skips the branch;
accumulation skips the purge.

```text
branch (build the alternatives) → run (let evidence decide) → purge (one canon, delete the rest)
```

**Branch.** For any non-obvious choice, name the candidate approaches *before* choosing one.
Alternatives listed after a decision are backfilled strawmen — the ordering is the mechanism, not the
listing. Then build the smallest runnable version of each in the scratchpad.

**Run.** Resolve by running, not by arguing. If you have been reasoning about which approach is better
without running anything, stop and run something.

**Purge.** Converge on one canon and delete the rest. Purge on evidence, not taste.

**The half-executed loop is worse than not running it.** Branching without purging leaves four ways to
do the same thing and no canon — every explored alternative surviving as a code path, a flag, an
adapter, a "we support both."

**Deletion is reversible; that is why the bar is low.** Git holds the deleted path. Treating deletion
as risky and addition as free is backwards — addition is the one that compounds. After a run, the
objective question is which code paths no company exercised. That is a data lookup, not a matter of
taste, and it is the version of "what is unnecessary" an agent can actually answer.

**Evidence adjudicates branches and updates canon.** A run can invalidate the question, not only pick a
winner. If a run contradicts `ARCHITECTURE.md` or a sprint spec, the run wins: record the contradiction
and change the document (§16.10). Never implement a spec you have watched fail.

---

## Part 2 — The failure modes, and why they happen

The mechanism matters. Knowing *why* a failure happens tells you which cure will actually bite.

| Failure mode | Why it happens | Cure |
|---|---|---|
| Over-indexing on safety and governance | Trained toward caution; governance code is easy to write and feels virtuous; the kernel section makes it read as legitimate work | Frame 1 |
| Reaching for state machines | Pattern completion. A state machine feels rigorous and gives the sense that all cases are handled | Frame 2 |
| Determinism where judgement belongs | **Testability-seeking.** Judgement is untestable and feels unsafe; determinism produces a green test. The agent converts an open question into a closed one in order to assert correctness | Frame 2 |
| Regex and keyword matching over content | Same as above, in its most visible form | Frame 2 |
| Adding instead of removing | The ask is always "do this task," and removal is never the literal ask. Knowing what is removable also needs run data the agent does not have | Frame 3 |
| Tunnelling on the first approach | **Autoregressive commitment.** Once a design paragraph is written, everything after is conditioned on it. This one is close to mechanical, which is why it needs a mechanical counter | Frame 3 |

Two of these diagnoses do most of the work:

**Testability-seeking** explains why regex survives every prohibition. The agent is not being lazy — it
is trying to prove the work is correct, and judgement offers no proof. Remove the incentive (say plainly
that judgement paths have no deterministic test and that this is fine) and the pull weakens.

**Autoregressive commitment** explains why "consider alternatives" never works when asked after the
fact. By then the context is already conditioned on the choice. The only reliable counters are ordering
(name candidates first) and fresh context (a different agent, without the sunk cost).

---

## Part 3 — What we already know does not work

Recorded so nobody retries it.

**Volume and emphasis.** This file replaced a block that said `NEVER SUBSTITUTE REGEX FOR REAL
JUDGEMENT` three times in capitals. It was a natural experiment, and the result was informative: it
bought *literal* compliance with the word "regex" while the underlying failure walked straight past it
as `contains()`, keyword lists, and score thresholds. Repetition targets the token, not the reasoning.
If these frames stop working, the fix is a better diagnosis — not louder text.

**Exhortation and values.** "Prefer simplicity," "complexity is weight," "deletion is progress." All
true, all already written down, none of them sufficient. A value with no trigger condition and no
cheaper named alternative does not change behaviour.

**Long prohibition lists.** They get skimmed, and they compete with each other for attention. Every
added rule dilutes the ones already there. This is why the cure is three frames rather than fifteen
rules — and why adding a fourth should be resisted unless it retires one.

**Asking the same context to reconsider.** It produces rationalisation of the existing design, not
simplification. Reconsideration needs a context that does not own the decision.

---

## Part 4 — Levers that are not rules

The two highest-leverage mitigations are not written rules at all, because written rules are the
weakest instrument available. They are commitments about the codebase and the workflow.

**Ergonomics beat discipline.** An agent reaches for regex partly because "call the model" is vague —
which model, what prompt, where does the key come from. Once the codebase has a one-line judgement
helper, the correct path is *easier* than the regex path, and the failure mode dies on affordance
rather than on willpower. Any judgement call that is awkward to make is a judgement call that will be
faked with a heuristic. **Treat friction on the judgement path as a bug.**

**A fresh-context simplification pass.** One agent whose only job is: *build this with half the
machinery.* Mechanically different from asking the authoring context to reconsider, because it has no
sunk cost in the design. This single move hits governance bloat, state machines, accumulation, and
tunnelling at once. ARCHITECTURE.md §16.9 gestures at it as a rotating human role; with agents it is
cheap enough to run every time.

---

## Self-check

Before adding machinery, and during review:

1. What is the risk, named specifically — and what is its disposition? Is accepted really not enough?
2. Which quadrant is this problem in? Did I read enumerability off the output instead of the input?
3. What else could this have been? Did I name the alternatives before choosing, or after?
4. What does this make deletable — and if nothing, why is that acceptable?
5. Has this actually been run, or do I only believe it works?
