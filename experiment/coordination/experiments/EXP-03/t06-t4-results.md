# EXP-03 T4 result — volatile customer operations

**Recorded:** 24 August 2026
**Status:** matched set closed; event-policy decision made
**Models:** accountable supervisor and customer-operations worker both `zai/glm-5.3`, high reasoning
**External effects:** none; all cases and companies were fictional and every response remained unsent

## Result in one sentence

Both arms produced a correct Support Policy v2 recovery pack, but neither used the material event to
change live work: S1-E completed only because its GLM calls happened to return much faster, while S1-T
ended with a clean verified worker commit that the supervisor had no remaining envelope to inspect and
promote. Stable, reversible, unsent work therefore keeps **terminal-only supervision** as the default;
a material wake is useful only when the runtime can deliver an effective redirect or in-flight work can
cause harm before the next artifact boundary.

## Frozen manipulation

The same frozen scenario, event and evaluator were supplied to both arms. The event was injected after
the first materialised case file, `customer-ops/cases/C-101.md`:

- Support Policy v2 made refunds above US$50 finance-approval-gated;
- cash goodwill for outages became forbidden and the 30-day credit required supervisor approval;
- C-103's CRM/order lookup became explicitly unavailable;
- all already-started but unsent work had to be swept to the new state.

Frozen hashes:

- scenario: `957651a39cd7dc7b5e12a8dbc8f917528b07cea08b4d3b10fc98cf04923e39d6`
- event: `ed5217dca0715cead5104933a8c76c0502e37c5537010cb725ff32950efcf115`
- evaluator: `942030270ab7f8edd2e978be032efc1de66a089bd6010cc6b8329544948af0a1`

The hash-determined order was S1-E then S1-T. Each arm had a 3,600-second execution envelope plus a
120-second drain. The envelope was an operator budget boundary, not a silence-based completion test.

## Outcomes

| Measure | S1-E: material event wakes lead | S1-T: terminal/block only |
| --- | ---: | ---: |
| Company disposition | Complete | Incomplete at envelope |
| Exact final worker artifact | `d5d6172ef4006c688df927f279c8d2515093976f` | `3c75fa2368054f3979275daa8130f2d82f4e5138` |
| Candidate equals worker artifact | Yes | No; candidate remained seed |
| Worker artifact clean | Yes | Yes |
| Worker-native verification | 204/204 | 18/18 |
| Corrected post-hoc artifact diagnostic | 20/20 | 20/20 |
| Frozen external evaluator | 12/19 | 15/19 |
| Run terminal wall time | 2,531.59 s | 3,720.01 s |
| Supervisor active time | 778.60 s | 673.03 s |
| Worker active time | 1,789.99 s | 3,027.73 s |
| Turns | 7 | 4 |
| Produced attempts | 2 | 2 |
| Reported cost | $2.33791888 | $0.77432724 lower bound |
| Observed used tokens | 298,566 | 106,113 lower bound |

S1-T's second worker call returned its terminal report during drain and committed the complete v2 pack
111.46 seconds after drain began, with about 8.55 seconds left before run termination. ACP did not return
usage for that call because the global envelope closed around it, so S1-T cost, token and tool totals are
known undercounts. No cost comparison should be made from the reported totals.

## What actually happened

### S1-E

1. The supervisor commissioned one end-to-end producer.
2. The event fired at 776.87 seconds while that worker was live.
3. The material wake cost 78.47 supervisor-seconds and $0.047117. The lead wrote an accurate message,
   but used `send`, whose delivery contract is **next wake**. It did not interrupt or alter the live
   worker.
4. The worker returned a complete but stale v1 artifact (`7bbe3dd`).
5. A normal terminal review found the v2 defects and issued an exact `repair` redirect.
6. The worker produced correct v2 commit `d5d6172`; the lead independently inspected it and promoted
   the exact commit.
7. The lead then requested a judgement from itself, causing one unnecessary extra self-wake before
   recording completion.

The material wake prevented zero stale work and zero repair loops. The useful intervention was the
ordinary terminal review.

### S1-T

1. The supervisor commissioned one end-to-end producer.
2. The event fired at 1,092.68 seconds and was recorded without waking the lead.
3. The worker returned a complete but stale v1 artifact (`9e41bba`).
4. The terminal review inspected the artifact and event, then issued a precise v2 `repair` redirect.
5. The worker produced clean v2 commit `3c75fa2` during drain and its verifier passed 18/18.
6. The run envelope closed before the supervisor could perform independent final review and exact
   promotion. The candidate correctly remained the seed commit, so supervisor conformance failed and
   the company outcome remained incomplete.

This is a legitimate company-level failure with a fully recoverable worker outcome. Treating the
worker's self-report as company completion would have erased the supervisor boundary being tested.

## Evaluator defect and disposition

The frozen evaluator failed both artifacts for equivalent, policy-compliant representations:

- it accepted only `manifest.policy_version`, not an equally clear nested `policy.version`;
- it accepted only case field `id`, not `case_id`;
- it interpreted “cash goodwill refunds are no longer permitted” as offering a cash refund;
- it interpreted “root cause is not confirmed” as asserting a cause;
- it interpreted future/conditional breach-review language as claiming a confirmed breach;
- its valid-JSON check count changed depending on the parse path.

The evaluator was not changed between arms. After the matched set closed, the explicitly labelled
`evaluate-v2-diagnostic.mjs` normalised the two machine-readable shapes, kept a fixed check count and
distinguished prohibitions/uncertainty from affirmative claims. Both exact worker artifacts passed
20/20. This diagnostic does not replace the frozen scores; it establishes the primary failure class
for those scores as **evaluator/contract**, not customer-operations capability.

Enduring measurement rule: if an exact output schema matters, freeze it in the task. Otherwise an
evaluator must accept semantically equivalent machine-readable shapes and must test negated claims as
negated claims.

## Decisions

### Event policy

Keep terminal/block supervision as the default for stable, local, reversible and unsent work. Wake on a
material event only when at least one of these is true:

- the lead can actually interrupt or redirect the live Attempt;
- partial output is externally visible or could cause harm;
- the event invalidates expensive irreversible work that has not yet reached an artifact boundary;
- a worker explicitly asks a decision whose answer changes useful progress.

A wake that cannot affect the live actor is coordination ceremony. `send` must be described to leads as
next-wake delivery; a material correction to active work normally requires `redirect` or a true runtime
interrupt. File appearance alone is too coarse a progress signal because GLM-5.3 often materialised a
large batch late; future event experiments need actor-emitted phase/progress events if earlier
intervention is the hypothesis.

### Supervisor discipline

- Commission stable work quickly; do not spend several minutes rediscovering an already-frozen brief.
- Inspect the exact worker artifact and preserve exact-promotion semantics.
- Return defects to the worker; both supervisors did this correctly.
- Do not request a judgement assigned to yourself. The accountable lead can make and record its final
  decision directly unless independent judgement is genuinely being introduced.
- Reserve explicit completion room after the last worker report. A drain can preserve work but cannot
  guarantee a final supervisory wake.

### Runtime/model finding

The dominant variance was GLM-5.3 wall-clock cadence, not coordination topology. Comparable calls
varied by many minutes while reported costs were similar. ACP exposed final turns and artifacts but not
useful in-flight phase/tool/usage checkpoints; the S1-T repair usage was lost at the envelope even
though the artifact and report survived. Needed bounded improvements are:

1. phase/tool progress telemetry from the first-party runtime;
2. durable usage checkpoints before terminal process exit;
3. a real live-attempt redirect/interrupt contract;
4. a completion-reserve policy based on active attempt state, not arbitrary silence;
5. no periodic model polling.

## T4 conclusion

T4 supports the supervisor invariant but does not support habitual mid-course supervision. The lead's
valuable work was mission-preserving terminal review, exact defect diagnosis, worker repair and final
promotion. The best current shape is one accountable, non-producing lead plus one end-to-end customer-
operations worker, terminal/block wakes by default, and genuinely effective material redirects only
where the risk justifies them.
