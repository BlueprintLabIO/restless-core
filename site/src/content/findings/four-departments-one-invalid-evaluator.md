---
title: "Four departments worked. The evaluator did not."
deck: "The product produced 90 exact units across four outcomes. A malformed review response stopped the company comparison."
publishedAt: 2026-08-26
order: 3
readTime: "5 min"
run: "EXP-05"
finding: "Product truth and evaluation truth need separate terminal states. A broken judge cannot erase observed work or approve it."
status: "Accepted"
---

The final EXP-05 run put four independent outcomes into one company window: sales, support,
monitoring and invoice reconciliation. Each outcome had one accountable, non-producing lead and one
Staff producer.

The product path closed 90 of 90 exact units. Four Staff Attempts overlapped. Three model calls ran at
the same time. The bounded projection found no cross-department context leakage. Exec had returned to
availability before the fourth request arrived.

Then the evaluator omitted a required field.

## Stop without laundering the result

The support review packet required an explicit decision. The model returned useful prose without that
field. The harness allowed one bounded retry. The second response made the same omission.

The branch stopped as evaluation-infrastructure-invalid.

Two tempting responses would have produced a cleaner report and weaker evidence. We could have parsed
the surrounding prose and guessed the decision. We could also have replayed production until the
judge returned valid JSON. Both would let the evaluator change history after seeing the result.

Instead, the run preserved two facts at once:

- the company produced the exact artifacts;
- the semantic company comparison has no valid terminal judgment.

The first fact remains useful. The second prevents us from calling the organisation better than its
baseline.

## Failure types belong in the result

An accepted outcome, a rejected outcome and an invalid evaluator are different states. Compressing
them into “failed” loses the next action.

An outcome failure should change the product or the work. An evaluator failure should repair the
measurement boundary. A provider failure may leave both product and evaluation unknown. Each state
needs its own evidence and restart policy.

This separation also protects negative results. The support terminal arm closed 80 of 96 cases. That
was a counted outcome failure, not a harness incident. Replaying it would erase the causal result.

## The run still found a product bug

The organisation was wide enough to handle four outcomes. The owner-facing dispatch path was not.
The fourth request took 68.68 seconds to dispatch, and the command remained unavailable for 69.43
seconds while the new lead oriented.

Exec eventually returned before the request arrived, so the organisational boundary was correct. The
software placed durability at the wrong synchronous boundary. Owner request acceptance should return
after durable responsibility exists. Lead orientation can continue independently.

That distinction is now a concrete implementation target. It came from the full-company run, not a
speculative responsiveness requirement.

## What changes next

The evaluator should use constrained output for the small enumerable decision envelope. That repair
belongs to evaluation infrastructure. It should not become a production coordination primitive.

The product needs two narrow changes:

1. return owner-facing availability after durable dispatch, before lead orientation completes;
2. deliver exact terminal failure evidence to the accountable lead so no outcome waits on a callback
   that can never arrive.

After those changes, the informative test is a larger dogfood release with real events and process
replacement. Another synthetic company cell would mostly test our ability to operate the harness.
