# EXP-08 — callback and native review-path preflight

**Status:** `product-invalid` — 26 August 2026. The callback, native ReviewTarget and owner-review
path passed; the required human-readable verification output did not.

**Classification:** non-counted isolated `_test` product-path check.

This check is the narrow response to Exp-08 validity gate 6. It does not begin an EXP-08 arm, create a
playbook, or contribute quality, timing, cost, or owner-attention data to the eventual comparison.

## Fixed envelope

| Item | Value |
| --- | --- |
| Company | `exp08_review_path_preflight_test` |
| Model | `openai-codex/gpt-5.6-sol`, no fallback |
| Model ceiling | USD 3, company-wide |
| Authority | local internal work only; no effects, deployment, publishing, contact, purchase, or credential expansion |
| Candidate | one throwaway static page in the company Runtime, not any Restless site or EXP-08 workload |

The USD 3 ceiling is a bounded route-probe resource, not the later matched-arm envelope. The risk that
it proves too small for a three-actor path is **guarded**: exhaustion is recorded as a product-path
preflight failure rather than treated as an arm or organisational loss.

## Required observed outcome

The owner directive asks the current Restless product path to produce one tiny page with the marker
`EXP08_REVIEW_PATH_OK`. A valid result must show all of the following:

1. Exec delegates the request to one non-producing accountable lead; the lead commissions at least
   one attributable Staff producer.
2. Staff creates the page, commits the candidate, and links the exact loopback URL as a
   `review_target` artifact.
3. The declared `review-target-live-probe` observes the URL after Staff returns and records the
   marker, instead of accepting a prose claim or a path outside the company Runtime.
4. The lead inspects the exact target without making production edits, then prepares one owner
   `outcome_review` handoff.
5. The owner projection exposes that handoff and target; the test may then close it by an explicit
   deterministic owner acceptance.
6. Work, Attempt, artifact, gate, handoff, model-spend, and no-effect evidence remain inspectable
   through the normal owner APIs.

An absent callback, unobservable URL, missing gate output, missing prepared owner review, ambiguous
ownership, or spend exhaustion is `product-invalid` for this preflight. It must not be attributed to
the eventual playbook treatment.

## Frozen owner directive

> This is an isolated Exp-08 product-path readiness probe, not a web-production arm. Do not publish,
> deploy, contact anyone, use credentials, or inspect any existing Restless site. Dispatch through the
> normal Exec → non-producing accountable lead → attributable Staff path. The Staff producer must
> create and commit a tiny static HTML page under `/company/projects/exp08-review-path/` that visibly
> contains exactly `EXP08_REVIEW_PATH_OK`, serve it from an uncredentialed loopback HTTP URL that
> remains live for review, link that exact URL as the Work's `review_target`, and record a
> `review-target-live-probe` gate that observes HTTP 200 and the marker after the producer returns.
> The lead must inspect the exact target without editing it and prepare one owner `outcome_review`
> handoff. Keep all production attributable to Staff. Return truthful evidence or a blocker; no other
> outcome is required.

## Closeout

### Observed run

| Item | Observation |
| --- | --- |
| Config | `exp08_review_path_preflight_test`; SHA-256 `a1f534d54c305471e6aa9098d968bed7b145853950716d0fbdc77d55775bcb27` |
| Work / Attempt | Work `d7889472-0fee-46b6-b96b-b1780678bc8d`; Staff Attempt `4fa73bad-ee0c-4d62-a680-8214b8e8edbb`, state `produced` |
| Accountable path | Exec delegated to non-producing lead `review-delivery` (Mara Venn), which commissioned Staff `staticweb-craft` (Ilan Roe). The Staff Attempt owns production; the lead's review wake reports no edit. |
| Native candidate | `http://127.0.0.1:38008/`, linked as `review_target` artifact `ef66e253-ef79-4042-ad77-08cf3b6ad9df`; committed at `831f4246a351f8408519b73b3ae40fa7df9312e9` |
| Direct inspection | The live page returned HTTP 200 and exactly the committed 131-byte HTML, whose only body text is `EXP08_REVIEW_PATH_OK`; page SHA-256 `8270a056ea9b947cf4dfa06b97071ea36145e5d763aaa5cf88232e7c70fa9f9f` |
| Gate | `review-target-live-probe`, gate `21edd01b-9e90-4497-b50b-5bec824bb796`, run `c7276800-2608-4319-8951-cc924db55c1a`: passed with exit 0 after the Attempt returned |
| Owner review | Lead prepared `outcome_review` handoff `e48acb8e-8ae9-4f9c-8d20-c08d50efacdd`; the normal owner projection exposed its live target and the explicit deterministic acceptance closed the Work at `2026-08-26T12:00:56.482700Z` |
| Authority / spend | `restless receipts` returned `[]`; accounted model spend was USD 0 against the USD 3 ceiling. Event telemetry identifies subscription billing and labels its list-price estimates noncanonical. |

### Failure classification

The declared probe command was:

```sh
sh -c 'set -eu; body=$(curl -fsS http://127.0.0.1:38008/); printf %s "$body" | grep -Fq EXP08_REVIEW_PATH_OK'
```

Its exact command and exit 0 establish that it checked the live marker. However, `gate_runs` retained
an empty `output_excerpt` and the SHA-256 digest of empty output. The Staff evidence file and the
later direct inspection preserve the marker, but neither is gate output. That misses required outcome
3 and the stated rule that missing gate output is product-invalid. It is therefore not acceptable to
call this complete verification-output readiness or to attribute it to either EXP-08 treatment.

The result does establish that the ordinary callback, attributable Staff production, native target,
post-return probe, lead review, owner projection and explicit owner-decision path work together. A
fresh, separately frozen verification-output probe is required before this validity gate can pass.

The run record preserves the company name, exact config digest, request, Work graph, Attempts,
artifacts, gate result, attention item, owner decision, spend and receipts. The company may be stopped
afterwards, but its evidence is retained. No result may be used to freeze or promote an EXP-08
playbook until Sprint 20 reaches its own terminal evidence report.
