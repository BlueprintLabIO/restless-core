# EXP-14 results: a tighter game-development loop, not final acceptance

## Verdict

EXP-14 materially improved Swift Arrival and removed several expensive sources of game-development
churn. The final candidate is `bd32f71967e91a67b3a28156c8fb287e52a6d51d`. It passes project load,
positive host/client delivery, route-zero rejection, parcel drop/recovery and seat re-entry gates.

The experiment did **not** establish the required final acceptance. No fresh source-blind player
completed either of the two journeys on the final candidate before both admitted model routes became
quota-bound. The honest product label is therefore: mechanically verified delivery slice with strong
positive player evidence on its immediate predecessor, but not yet independently certified playable
or fun.

## What the loop achieved

1. A Runtime specialist built a five-command `native-session` seam: `launch`, `observe`, `act`,
   `export` and `stop`. Launch returned one opaque exact CLIENT handle. Players no longer enumerated
   windows, guessed titles or used a second attachment channel.
2. The first product candidate fixed route-end exit and delivery and added a repeatable host/client
   gate runner. All five declared gates passed.
3. A fresh source-blind visual player completed pickup, deliberate drop, recovery, loaded drive,
   route 40/40, exit, actionable unload and host-owned delivery in 194 seconds on candidate
   `d90a451b81ba0265715414b818384079935a3d34`.
4. A separate fresh source-blind negative player entered the marked destination zone on foot at route
   0/40. Delivery was correctly refused but no visible explanation appeared. This was a real product
   defect that deterministic state checks alone had not exposed.
5. The one authorised evidence-driven repair added a high-contrast six-second `ACTION BLOCKED` panel
   and actionable truck-journey guidance. The final candidate then passed all five deterministic gates
   again, including the exact feedback assertion.
6. The final source-blind negative player produced a rejection frame, but had not entered the marked
   zone. The accountable semantic review correctly overruled the player's `outcome_met` label instead
   of treating agent prose as evidence.

## Acceptance matrix

| Evidence | Result | Meaning |
|---|---:|---|
| Godot parse/load | Pass | Candidate is runnable |
| Positive host/client delivery | Pass | Authoritative completion works mechanically |
| Route-zero rejection | Pass | Shortcut cannot complete delivery |
| Drop and recovery | Pass | Ordinary mistake is recoverable |
| Early exit and seat re-entry | Pass | Truck interaction recovers naturally |
| Opaque launch-to-target handle | Pass | Player attachment is deterministic |
| Fresh complete journey on predecessor | Pass, 1 | Strong positive evidence, but not final-candidate acceptance |
| Correct in-zone blind rejection on predecessor | Pass as diagnosis | Found missing visible feedback |
| Correct in-zone blind rejection on final candidate | Not established | Agent tested the wrong spatial condition |
| Two fresh final-candidate journeys | 0 of 2 | Blocked by model capacity |
| Fun, feel or release readiness | Not tested | Requires founder/human taste and broader gameplay |

## The 10x loop shape

The strongest design is not a larger team. It is a shorter evidence path:

`immutable candidate -> cheap mechanical gates -> one exact-handle blind player -> smallest failure packet -> one repair -> restart final blind gate`

This shape generalises beyond games. Replace rendered pixels with the domain's native outcome and keep
the same ordering: cheap deterministic checks first, sparse independent judgement second, human taste
last.

The durable structural improvements are:

- **Identity at launch.** A launch operation returns the exact surface it created. Discovery and
  attachment are one atomic capability.
- **One native outcome per Work.** Positive journey, negative shortcut and product repair are separate
  outcomes. Combining them caused a valid completed journey to be labelled a failed task.
- **Sparse judgement.** Mechanical gates run on every candidate. Vision is spent only at semantic
  milestones and stops at the first conclusive blocker.
- **Semantic supervision.** A lead checks whether evidence proves the requested condition. Agent pass
  labels never outrank the pixels and state trace.
- **One refusal, then sleep.** Provider failures become a shared durable cooldown. No actor-local retry
  storm, polling loop or timeout cascade is allowed.
- **One runtime generation.** Host gateway and company agent now pin OMP `18.0.10`; native protocol
  skew is an image-build failure, not a production-time mystery.
- **Validated admission.** HTTP 200 is insufficient. A model route is admitted only when the body is a
  valid completion/error envelope for the selected protocol and exact model.
- **Bounded evidence.** Each player may retain at most 12 decisive frames. Raw captures are deleted
  after compact text/JSON synthesis.

## Provider terminal fact

The protocol-correct BigModel coding route for exact `glm-5.3-flash` returned provider code `1310`
with a reset timestamp of 30 August 2026. Exact GPT-5.6 Terra and Sol on the configured GPT route both
reported the key's seven-day allowance exhausted. After the runtime version repair, Restless made one
GLM attempt, recorded one shared quota cooldown and remained quiet.

## Decision

Keep the final candidate and the loop architecture. Do not claim two-run playability. When capacity is
available, resume only the three missing clean-room outcomes: correct in-zone final negative, final
journey one and final journey two. No further product repair is authorised unless one of those fresh
runs supplies a new decisive failure packet.
