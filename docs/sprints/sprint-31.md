# Sprint 31 — Establish the source-owned Company Identity kernel

**Status:** Implemented and locally verified; matched model-quality rerun remains provider-blocked

**Date:** 31 August 2026

**Programme:** [Company Identity](./company-identity-programme.md)

**Depends on:** Sprint 28 semantic records and Sprint 29 outcome inheritance. It may be specified while
Sprint 30 runs, but implementation must not destabilise Sprint 30's exact Work and supervision changes.

## Why this sprint exists

Restless already carries product facts, approved pages, rejected drafts, design assets and founder
judgements, but they are scattered across repositories, prompts, conversations and dogfood reports.
Agents can retrieve different subsets and manufacture different “brand” interpretations. A larger
system prompt would make that inconsistency harder to inspect, not solve it.

## Outcome

One `_test` company has a durable, owner-governed Company Identity release whose current truth and
evidence can be inspected, changed, versioned and compiled into a bounded outcome-specific Identity
Brief. Two outcomes created after restart use the same released identity without copying prompt prose
or relying on browser state.

The first release establishes the common kernel and company truth. Voice, visual and culture evidence
are admitted as typed pillars but remain intentionally thin until Sprints 32–34 prove their native
semantics.

## Product contract

An identity release contains:

- exact company and immutable release identity;
- effective-from time and predecessor, when any;
- current verified company-truth claims and vocabulary;
- attributed positive and negative evidence references;
- typed voice, visual and culture observations where already approved;
- explicit scope and expiry for any exception;
- the owner/Authority decision that promoted it; and
- a compact change account from its predecessor.

Draft observations and proposals are not effective identity. Assets and commissioned outcomes bind to
the release they actually used. A later release does not rewrite their history.

## Success contract

1. Fresh companies can have no released identity without pretending that defaults are authored truth.
2. A legacy company can adopt its first release without changing its existing artifacts.
3. Only the authenticated owner/Authority path can promote, supersede or deliberately except identity.
4. Every truth claim has provenance, status and a concrete locator or is visibly an attributed belief.
5. Conflicting claims fail closed until resolved; the compiler never chooses the more fluent one.
6. Runtime compiles a deterministic, size-bounded brief for the same release, outcome and channel.
7. The brief contains only relevant evidence and clearly separates fact, belief, expression guidance
   and exception.
8. Restart, handoff and a second client preserve the exact release and selection source.
9. A later truth correction marks affected claims and discoverable assets stale without deleting them.
10. No generic preference table, arbitrary key/value policy engine, project entity, quality score or
    second Work lifecycle is introduced.

## Slice per layer

**Authority.** Own release promotion, supersession and bounded exceptions using the existing owner
decision boundary.

**OrgIntel.** Retain typed proposals, evidence, immutable releases, lineage and outcome-to-release
binding. Reuse existing authored record and artifact semantics where they fit.

**Runtime.** Resolve evidence and compile one bounded Identity Brief. Unknown or unavailable evidence
stays unknown; it is not summarised into certainty.

**Exec / Staff.** Receive the effective release and brief locator. They cannot promote their own output.

**Cockpit.** Show current release, truth, evidence, pending proposals, conflicts and release history.
The cockpit does not become a CMS or a free-form brand-book editor.

## Salvage

No unverified `docs/SALVAGE.md` lift is adopted. Product-language and quality evidence from Sprints 23,
28 and 29 enters only if T0 revalidates its exact source, current truth and authority.

## Ticket index

Status lives only here.

| Status | Ticket | Outcome |
| --- | --- | --- |
| [x] | [S31-T0](./sprint-31/t0-corpus-and-grammar.md) | Freeze identity evidence and evaluation grammar |
| [x] | [S31-T1](./sprint-31/t1-canonical-contract.md) | Add one canonical release and proposal contract |
| [x] | [S31-T2](./sprint-31/t2-owner-promotion.md) | Make promotion and exceptions owner-governed |
| [x] | [S31-T3](./sprint-31/t3-bounded-compiler.md) | Compile relevant identity into bounded Staff context |
| [x] | [S31-T4](./sprint-31/t4-owner-surface.md) | Expose truth, evidence, lineage and conflicts sanely |
| [x] | [S31-T5](./sprint-31/t5-dogfood-and-purge.md) | Prove restart continuity, stale truth and no policy sprawl |

Expected order: **T0 → T1/T2 → T3/T4 → T5**.

Implementation and verification evidence is recorded in
[`docs/dogfood/company-identity/s31-run-report.md`](../dogfood/company-identity/s31-run-report.md).
The source, persistence, compiler, owner-action, stale-binding and responsive-surface contracts pass.
The additional matched copy-quality comparison is explicitly provider-blocked and must not be read as
a quality win or loss.

## Terminal decision

- **Pass:** the integrated `_test` company produces two identity-bound outcomes across restart and
  correctly handles one conflicting and one corrected truth claim.
- **Revise once:** repair one bounded source, compilation or projection defect without widening scope.
- **Stop negative:** if identity cannot remain source-owned and bounded without becoming prompt soup or
  a generic policy engine, preserve the corpus and do not activate Sprint 32.
