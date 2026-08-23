# Workload R-LH-01 — Lumaara release evidence decision

Status: frozen for matched run

## Frozen success contract and native target

The exact contract and full synthetic source corpus are `scenario.md`. The native target is the
founder-readable Markdown decision memo at `research/lumaara-release-decision.md`.

## Starting artifact

- Cosmon seed: `514b7b3d0a65e093af608b08ca142344412181f4`.
- Scenario SHA-256: `9abf51439f580d8b0dce1ce802236c14b494e5083d10280b49297e8efc09a438`.
- Evaluator SHA-256: `238229bacd4a2414888dc7080b8f77e4683f56210beabdc0b2c48665ebecb29a`.
- No corpus claim is real-company or market evidence.

## Structural hypothesis

The 32 records divide into four independently useful evidence dossiers plus a structured traceability
ledger. One Staff analyst can produce two complete cited dossiers and the first ledger pass while the
lead analyses the complementary regions and retains cross-source weighting, tensions and the final
memo. This is intended as large parallel breadth with no shared mutable implementation state and a
substantial serial synthesis tail.

## Independent classifications

The first draft was independently classified by fresh GPT-5.6 Sol and Terra sessions as a 3–6-hour,
lead-dominant memo with limited overlap and not credibly beyond one effective strong-lead session. That
disagreement with the intended R-LH role was accepted. The contract now requires four independently
useful dossiers and a ledger in addition to the lead-owned memo.

Fresh GPT-5.6 judges then classified the revised pack independently:

- Sol: 8–14 focused hours (up to 12–18 with semantic repair), 20–40 tool interactions, high
  saturation/separability, moderate coupling, high overlap and medium-high repair cost.
- Terra: 4–7 focused hours, high saturation, medium-high separability, medium coupling, high overlap
  and medium-high repair cost.

Both judged the four regional dossiers independently useful research artifacts rather than artificial
file decomposition. Both retained “single lead with optional extraction assistance” as the strongest
alternative because the corpus is fixed and compact. The cleanest Staff seam is the full accessibility
and usability dossier plus proposed A01–A08 ledger rows; the lead retains evidence normalization,
cross-source weighting, recommendation, release sequence, final ledger and memo.

## Arm allocation

- Negative control: untouched seed failed 34/36 fixed checks; the two passes were absence-safe checks
  for unknown citations and placeholders.
- Order seed: `EXP-01:E02:R-LH-01:gpt56:2026-08-23`.
- Order SHA-256: `a71f55314b7e8d40ee89c2f61acfac06a419e2c2ca7b3e3693ccc2ba7c8da744`.
- Odd first-32-bit parity selects **B1 → B0**.
- Lead: `gpt-5.6-sol`, medium reasoning. B1 Staff: `research-analyst` on `gpt-5.6-terra`,
  low/runtime-default reasoning.
- Both arms receive identical scenario/evaluator bytes, seed, tools, 14,400-second envelope,
  120-second drain, no actor timeout and USD 6 nominal ceiling.

## Post-run result

The structural hypothesis was falsified in the ordinary B1 implementation. Although the ledger was an
independent artifact seam and 136.4 actor-seconds overlapped, the Work context carried the identifiers
but not the frozen corpus. The worker substituted unrelated repository evidence, and the lead rejected
and rebuilt the whole ledger. Both arms then failed the external evaluator. See `runs.md`.
