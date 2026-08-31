# Sprint 28 implementation run report

**Run date:** 31 August 2026  
**State:** Implementation, automated evidence and browser verification complete; blinded-human
acceptance open.

## Delivered

- Every projected Attention action now declares its semantic role, immediate consequence and next
  observable state beside the existing source operation id. The cockpit does not infer these from
  authored prose.
- Attention queue rows use the authored owner headline and name the exact ask. Focused views separate
  changed facts, impact, uncertainty, recommendation, real controls, waiting consequence and source
  evidence. Each control exposes its effect without hover.
- The shared owner-writing instructions reach Exec and lead authoring paths. They assume no technical
  context, give each semantic field one job and keep optional message receipts out of ordinary chat.
- Owner-brief writes reject objective structural defects: blank required/optional values, multiline
  headlines and exact prose duplication across distinct semantic roles. No score, word cap, keyword
  classifier, anonymous rewrite or browser-time model call was added.
- Work opens with its authored human paragraph while retaining the exact execution contract unchanged
  inside a collapsed technical disclosure. Artifact purpose and availability are foregrounded over
  raw URI. New automatic artifacts are named from the readable Work title; the exact source
  relationship identifies and repairs the presentation of older auto-labels without rewriting
  genuinely authored labels.
- Consequential agent messages may declare `outcome`, `nextStep` and `ownerNeed`. The UI renders only
  fields actually present; free-form message prose remains the primary communication.
- The frozen `_test` corpus covers bounded approval, native outcome review, an irreducible human
  browser step, one post-decision causal continuation, a completed Work contract and important linked
  outputs. Multi-select was deliberately not built: none of these source records exposes
  independently composable choices.

## Automated and live evidence

| Check                                                          | Result                                                                                                        |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `npm run verify:reader`                                        | Pass: Attention, Work, artifacts, continuation and varied conversation                                        |
| `npm run check`                                                | Pass: 0 Svelte errors and 0 warnings; type policy pass                                                        |
| `npm run build`                                                | Pass: static production build generated                                                                       |
| Focused `restless-orgintel` owner-brief validation tests       | Pass: 3/3                                                                                                     |
| Focused `restlessd` action, intent, bindings and context tests | Pass                                                                                                          |
| Live Vite → fixture-proxy Attention read                       | Pass: all 3 categories, 8 actions and the continuation retain source ids, roles, consequences and next states |
| Desktop browser walkthrough at 1440×900                        | Pass: full approval/review decisions and visible consequences fit the focused view                            |
| Mobile browser walkthrough at 390×844                          | Pass: Attention and Work preserve reading order with no page-level horizontal overflow                        |
| Keyboard walkthrough                                           | Pass: logical queue → source context → action → conversation → provenance → evidence order                    |
| Decision-history and Work accessibility snapshots              | Pass: causal roles, observed result, owner, human summary and linked-output purpose are exposed               |
| Ordinary/consequential message walkthrough                     | Pass: exactly 1 semantic block on the consequential reply; ordinary conversation has none                     |
| Scoped formatting and `git diff --check`                       | Pass                                                                                                          |

The full `restlessd` suite ran 239 tests: 229 passed, 8 were explicitly ignored and 2 failed in
pre-existing work outside Sprint 28. The failures are:

- release schema identity is `21` while the uncommitted migration set reaches `22`; and
- `staff::tests::one_actor_keeps_one_organisational_posture_across_wake_types` expects capitalised
  “Never” while its current prompt contains lower-case “never”.

The full `restless-orgintel` suite is likewise blocked by the pre-existing regenerated ScheduleRow
contract not matching the current checked-in binding. Sprint 28's focused owner-brief tests pass.

## Source-fidelity audit

- Approval exposes only `grant` and `decline`; neither prose nor chat grants authority.
- Outcome review keeps inspection, acceptance, revision feedback and conversation distinct.
- Opening an external provider page explicitly says that opening does not complete the human step.
- Decision history distinguishes recorded choice, released Work, current state, responsible actor and
  observed outcome. Pending provider results remain pending, and internal Attempt numbers no longer
  appear in its primary narrative.
- Evidence and exact Work contracts remain available without being rewritten for the primary view.
- Ordinary conversation renders only its natural authored reply. A consequential reply exposes
  Outcome, Next and Needs you in reading order without the message crossing an authority or review
  boundary.
- A live read of the existing `_test` company confirmed that real model-authored Work carries a
  readable opening before its long execution contract. It also exposed legacy auto-generated
  artifact labels that copied the whole expected-output contract; the source and compatibility paths
  now render these as “Output from: [Work title]” with a plain evidence note.

## Open acceptance gate

Desktop, 390px mobile, keyboard order and accessibility-tree structure are signed off on the frozen
source-backed fixture. These checks establish that the intended language and hierarchy actually
reach the rendered product; they do not prove human comprehension.

One gate remains open: an uncoached low-context person has not yet supplied the blinded account.
Neither automated structure nor implementation-team reading is reported as proof that the authored
output is easy to understand.

Run [`blinded-reader-exercise.md`](blinded-reader-exercise.md) when a browser and an uncoached reader
are available. Any wrong account is a product failure to revise, not a passing result with notes.

## Residual decisions

- Keep free-form prose for explanation and uncertainty; the corpus includes information that would be
  harmed by a rigid short template.
- Keep the current Work two-paragraph boundary. It is deterministic at the declared blank line and
  avoids adding a second summary lifecycle.
- Do not build generic cards, a universal presentation entity or multi-select until a real source
  record earns them.
