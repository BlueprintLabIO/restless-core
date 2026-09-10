# Sprint 44 — Prove the Open Company Runtime product claim

**Status:** Planned; final programme gate  
**Date:** 4 September 2026  
**Programme:** [Open Company Runtime](./open-company-runtime-programme.md)  
**Depends on:** Sprints 40–42, plus Sprint 43's recorded positive or negative disposition and Sprint
39's outstanding provider-backed three-harness qualification.

## Why this sprint exists

Managed autonomy plus native escape should make Restless a better default than operating Codex or
Claude directly for company work. Architecture and feature presence cannot establish that. Restless
could still add handoff friction, hide useful native capability, cost more, lose context or cause the
owner to abandon the managed path after every intervention.

This sprint compares the complete product against real direct-native baselines and settles the
positioning narrowly. It is an outcome test and deletion sprint, not a mechanism-building buffer.

## Outcome

A matched corpus of real company outcomes runs through three arms:

1. **Direct native:** the owner operates Codex or Claude in its normal supported client.
2. **Restless managed:** the company completes the outcome without native takeover where possible.
3. **Restless hybrid:** autonomous work reaches a useful intervention point, the owner takes native
   control and returns it, and the company completes the outcome.

Evidence records accepted output, owner time, interventions, latency, cost, capability gaps, recovery,
handoff loss and residue. A source-blind reviewer judges outcomes before seeing the arm. The founder
then approves one precise claim, narrows it or rejects it. Unsupported UI, abstractions and copy are
removed.

## Frozen comparison contract

1. **Same outcome, inputs and acceptance standard.** Arms may use native idioms but may not receive
   different hidden requirements or quality bars.
2. **Current supported builds.** Record exact Restless, Codex, Claude, adapter, model and client
   identities and authentication/billing route.
3. **No synthetic task score as product truth.** Native artifact review and company outcome acceptance
   decide utility.
4. **Owner attention is measured, not estimated.** Record active minutes, intervention count and
   blocking waits separately.
5. **Quality and convenience stay separate.** A faster rejected result does not beat a slower accepted
   one; equivalent accepted results expose time/cost differences.
6. **Provider advantages are allowed.** Direct arms use supported native features; the hybrid path must
   reach them through its claimed escape hatch.
7. **Restless advantages are allowed.** Managed/hybrid arms use durable roles, schedules, memory,
   delegation, Authority and recovery because those are the product.
8. **No hidden subsidy.** Human setup, manual reconciliation, unmetered subscription use and failed
   attempts are recorded.
9. **S39 live qualification closes here.** Any harness called supported passes the shared provider,
   secret-isolation, permission, cancellation and restart cases.
10. **The claim applies only to observed company work.** It never says Restless's model or harness is
    universally better.

## Counted outcomes

- one hard-goal Attention case with a deliberately broad search space and owner narrowing;
- one substantial repository change with tests, failure diagnosis and low-level native inspection;
- one non-code or mixed-media company artifact using browser/application state;
- one recurring or interrupted operation spanning restart;
- one governed external effect with exact provider reconciliation; and
- one no-takeover case where managed Restless should avoid unnecessary owner attention.

At least one outcome uses Codex and one uses Claude. At least one hybrid takeover is exact attachment or
resume, and any reconstruction is scored and described separately.

## Success contract

1. Every arm produces reviewable native artifacts and complete operational receipts or an explicit
   failure; missing evidence cannot be scored as success.
2. A fresh reviewer judges artifact usefulness and outcome acceptance without knowing the arm during
   first review.
3. Managed/hybrid Restless produces accepted outcomes with materially lower owner coordination burden
   than direct operation across the portfolio, not merely one cherry-picked case.
4. The hybrid path reaches important native capability without lost Work identity, workspace changes,
   decisions, effects or provider/session provenance.
5. No concurrent writer, cross-company state, duplicated canonical transcript, leaked credential or
   orphan process occurs.
6. Restart, client crash and abandoned takeover recover within the frozen bound without manual database
   or process repair.
7. Cost and latency overhead are recorded by source; unknown provider/subscription usage remains
   unknown and cannot support a savings claim.
8. Owners can always export or retain ordinary files, Git history and outcome evidence without keeping
   Restless running.
9. Every native-only gap receives one disposition: reachable through takeover, acceptable limitation,
   scheduled narrow repair or claim blocker.
10. Harness/app settings remain expert policy; no per-item picker or marketplace emerges from the
    comparison.
11. A founder-approved release note states supported harnesses, takeover relationship, optional
    workstation status and known limitations separately.
12. Unsupported mechanisms, prototype clients, raw-key routes, duplicate role fields and stronger
    marketing copy are deleted before exit.

## Positioning decisions

The strongest admissible claim after a pass is:

> Restless is the better default for running sustained company work: it preserves the full company
> around Codex and Claude, and lets the owner enter their native controls when needed.

A narrower pass may claim:

> Restless coordinates durable company work across Codex and Claude with governed native takeover.

Never claim that Restless's underlying model, every individual coding task or every native interaction
is superior. “Strictly better” remains internal strategy language unless the approved evidence and
legal review support a precise public comparison.

## Slice per layer

**Full product.** Run released paths without test-only bypasses. Kernel, OrgIntel, Runtime, owner UI,
native clients and external providers all contribute their normal evidence.

**Evaluation.** Freeze inputs and rubrics before runs, blind first artifact judgement, retain complete
operational observations and keep infrastructure-invalid runs separate from product failures.

**Positioning and release.** Bind every public claim to the accepted corpus, exact support disposition
and named limitations. Remove copy not supported by evidence.

## Salvage

- Reuse Sprint 39 T7's three-harness corpus and evidence requirements rather than running a weaker
  handshake-only qualification.
- Reuse EXP-17's source-blind matched-arm evaluation principles only after T0 revalidates them for
  owner-operated native sessions and company outcomes.
- Prior benchmark conclusions do not substitute for current builds, authentication routes or takeover
  evidence.

## Out of scope

- broad public benchmark leadership claims;
- ranking models or providers globally;
- changing defaults automatically from aggregate scores;
- an app store launch;
- adding mechanisms merely because one arm performs poorly;
- hiding infrastructure-invalid or negative outcomes; and
- expanding to multiplayer or fleet economics.

## Stop rules

Narrow or reject the positioning if direct native use remains materially easier for the counted jobs,
if takeover loses state or capability, if owner attention does not improve, or if cost cannot be
compared honestly. Do not repair the rubric after seeing results and do not publish an aggregate scalar
that conceals an outcome or security failure.

## Ticket index

| Status | Ticket | Outcome |
| --- | --- | --- |
| [ ] | [S44-T0](./sprint-44/t0-freeze-comparative-corpus.md) | Freeze arms, outcomes, rubric, builds and claim thresholds |
| [ ] | [S44-T1](./sprint-44/t1-release-and-isolation-preflight.md) | Prove live providers, clean environments and comparable inputs |
| [ ] | [S44-T2](./sprint-44/t2-run-company-outcomes.md) | Execute direct, managed and hybrid arms with native evidence |
| [ ] | [S44-T3](./sprint-44/t3-recovery-security-and-residue.md) | Exercise crash, restart, isolation, secrets and cleanup |
| [ ] | [S44-T4](./sprint-44/t4-attention-cost-and-capability-analysis.md) | Compare owner burden, cost, latency and native-only gaps honestly |
| [ ] | [S44-T5](./sprint-44/t5-independent-review-position-and-purge.md) | Settle the claim, release dispositions and delete unsupported machinery |

Expected order: **T0 → T1 → T2/T3 → T4 → T5**.

## Terminal decision

- **Strong pass:** approve the “better default” claim for the observed company-work scope.
- **Narrow pass:** approve governed multi-harness company coordination and native takeover only.
- **Revise once:** repair one bounded handoff or evidence defect, then rerun every affected arm.
- **Stop negative:** retain the useful underlying capabilities, reject the stronger positioning and
  return to the observed product friction.
