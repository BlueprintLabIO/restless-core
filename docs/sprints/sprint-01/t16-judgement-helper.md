# T16 · Judgement helper — make the model call the easy path

**Layer:** Kernel-adjacent — it is a thin call over T2's gateway, used from everywhere.
**Serves:** `LLM_CURE.md` Part 4 names this the highest-leverage mitigation available and states that **friction on the judgement path is a bug.** We wrote that, then shipped fifteen tickets none of which build it. This ticket closes that gap.
**Makes deletable:** Every heuristic that would otherwise be written in its place.
**Depends on:** T2.

## The failure this prevents

An agent reaches for `contains()` or a score threshold partly because "call the model" is vague — which model, what prompt, where does the key come from, how do I parse the answer. Regex is three lines and the model call is thirty. **The failure dies on ergonomics, not on discipline.** Prohibitions have already been tried; see `LLM_CURE.md` Part 3.

## Build

One call, no ceremony:

```rust
judge!("is this commitment blocked, or just quiet?", ctx) -> Verdict
```

- Routes through the T2 gateway, so it is spend-accounted and key-isolated like any other model call.
- Two shapes cover the cases in the register: **one-of-N** (returns a variant from a fixed set — the judgement + enumerable quadrant) and **open** (returns text).
- Structured output for the one-of-N shape, so the caller gets a typed value and never parses prose.
- A failure is a failure — no silent fallback to a heuristic when the model is unavailable. Falling back to a keyword check on error is the exact failure this ticket exists to prevent, and it would hide itself.

## Not in scope

Caching, batching, a prompt-template registry, an evaluation harness. If judgement calls turn out to be hot enough to need any of that, the run will say so.

## Acceptance

The judgement calls named in the sprint spec's register are implemented **through this helper and not around it.** Specifically, all three of:

- T4's turn termination — continue / blocked / done / abandon;
- T9's "is this staff member stalled, or just thinking?";
- T8's simulated persona behaviour.

Grep the sprint's diff for the smell family — `contains()`, keyword lists, score thresholds, enum classification of free text over content. Any hit is either justified as genuinely deterministic or converted.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
