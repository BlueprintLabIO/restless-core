# EXP-07: Greenfield Codex versus Restless site

**Status:** Frozen; arms not started  
**Date:** 26 August 2026  
**Depends on:** EXP-06 process evidence and the owner's rejection of a shared incumbent as a useful
creative comparison.

## Outcome

Produce two genuinely independent public-site proposals for Restless, one through the current
Restless company path and one through a single Codex producer. Both begin from empty Git repositories
containing only the same product-truth and evidence pack. Host the resulting sites behind neutral A/B
labels for blind owner review before revealing arm identity or process evidence.

This run measures greenfield brief interpretation, creative direction, writing, implementation and
closure. It does not reuse or repair either EXP-06 candidate.

## Treatment

```text
Arm C: identical greenfield pack -> one Codex producer -> complete hosted candidate

Arm R: identical greenfield pack -> Exec -> non-producing accountable lead
                                      -> Staff production -> lead judgement
                                      -> complete hosted candidate
```

Both arms use GPT-5.6 Sol at medium effort with no fallback. The producing actors may choose any
appropriate web stack, page structure, typography, palette, interaction model and visual signature.
Neither arm can inspect the existing `site/`, an EXP-06 artifact, the peer workspace or the peer's
hosted output.

## Greenfield boundary

Each arm receives a fresh repository whose initial commit contains only:

- `BRIEF.md`;
- `PRODUCT-TRUTH.md`;
- `EVIDENCE.md`; and
- a minimal `.gitignore`.

There is no package manifest, framework scaffold, component library, design token, logo asset, copy,
route implementation, lockfile or deployment file. Public internet access may be used for current
source-first research and mature dependencies, but the current Restless repository and published
site are excluded. This makes divergence possible without giving either arm a different aesthetic
brief.

## Validity and completion

Before productive cognition, freeze the exact input hashes, neutral arm labels, order and rubric.
Each valid candidate must:

1. implement every required route from the brief;
2. build from a clean install and include a production Dockerfile exposing port 80;
3. serve every route with no desktop or mobile overflow;
4. support keyboard focus and reduced motion;
5. contain no em dash, fabricated evidence or unsupported product claim;
6. include an arm-local evidence note and one final clean commit;
7. undergo the same independent objective probe and at most one matched repair packet; and
8. be hosted under a neutral public URL only after both arms finish.

Elapsed time is an operating observation, not semantic completion. A 75-minute non-progress safety
envelope may stop an arm and is recorded as operator termination rather than ordinary task failure.

## Evaluation

The owner receives only the two native hosted sites labelled Candidate A and Candidate B. Process,
topology, cost, timing, commit authorship and identity remain hidden until the owner answers `A`, `B`,
`tie` or `neither`.

The weighted blind rubric scores immediate clarity, product truth, writing, visual direction,
information architecture, interaction quality and evidence. Process evidence explains the outcome
and breaks only a near tie. One run selects a proposal for this task; it does not establish a universal
solo-versus-team law.

## Stop boundary

Do not merge either result into `dev`, replace the existing public site, reveal arm identity, publish a
winner claim or change the Restless coordination invariants before the owner locks the blind decision.

