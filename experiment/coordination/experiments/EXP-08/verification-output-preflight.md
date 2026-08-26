# EXP-08 — verification-output preflight

**Status:** passed — 26 August 2026. This clears EXP-08 validity gate 6 only; P1 remains
`product-invalid`, and the Sprint 20 and EXP-07 source-evidence entry gates remain independent.

**Classification:** non-counted isolated `_test` product-path check, following the product-invalid
result of [P1](review-path-preflight.md).

P1 proved the ordinary supervised native-review path, but its silent boolean gate had no retained
human-readable stdout. This follow-up tests the narrower question left open: can the same product
path retain the exact verification marker in the gate-run output that reaches accountable review?
It is not an EXP-08 arm and produces no playbook, quality, cost, repair or owner-attention datum.

## Fixed envelope

| Item | Value |
| --- | --- |
| Company | `exp08_verification_output_preflight_test` |
| Model | `openai-codex/gpt-5.6-sol`, no fallback |
| Model ceiling | USD 3, company-wide |
| Authority | local internal work only; no effects, deployment, publishing, contact, purchase, or credential expansion |
| Candidate | one throwaway static page in the company Runtime, unrelated to P1, any Restless site, or an EXP-08 workload |

The USD 3 ceiling is a bounded route-probe resource, not the later matched-arm envelope. Exhaustion,
missing callback, absent native target, missing owner review, an output-less gate, or ambiguous
production ownership is `product-invalid`; it must never be attributed to a playbook treatment.

## Required observed outcome

1. Exec delegates to one non-producing accountable lead, which commissions attributable Staff
   production.
2. Staff commits a page containing `EXP08_GATE_OUTPUT_OK`, serves it on the exact loopback target, and
   links that URL as the Work's native `review_target`.
3. The declared post-return gate must emit `EXP08_GATE_OUTPUT_OK` to stdout *after* fetching the live
   target and before it succeeds. The owner-visible `gate_runs.output_excerpt` must contain that exact
   marker; exit status alone does not satisfy this check.
4. The lead reviews the exact target without production edits and prepares one ordinary
   `outcome_review` handoff. The owner projection exposes it and an explicit deterministic acceptance
   may close the test.
5. Work, Attempt, artifact, gate command and output, handoff, spend, and receipts remain inspectable
   through the normal owner APIs.

## Frozen owner directive

> This is an isolated EXP-08 verification-output readiness probe, not a web-production arm. Do not
> publish, deploy, contact anyone, use credentials, or inspect an existing Restless site or another
> experiment's candidate. Dispatch through normal Exec → non-producing accountable lead → attributable
> Staff. The Staff producer must create and commit a tiny static HTML page under
> `/company/projects/exp08-gate-output/` whose visible body text is exactly `EXP08_GATE_OUTPUT_OK`, and
> serve that committed page at exactly `http://127.0.0.1:38009/` using an uncredentialed loopback HTTP
> service that remains live for review. Link that exact URL as the Work's `review_target`. Declare and
> run a `review-target-live-probe` gate that fetches that URL after the producer returns, proves HTTP
> success and the marker, and **prints `EXP08_GATE_OUTPUT_OK` to stdout before exiting 0**. A silent
> `grep -q`-only check is not acceptable: the retained gate output must visibly contain the marker.
> The lead must inspect the exact target without editing it and prepare one owner `outcome_review`
> handoff. Keep all production attributable to Staff. Return truthful evidence or a blocker; no other
> outcome is required.

## Closeout

### Observed run

| Item | Observation |
| --- | --- |
| Config | `exp08_verification_output_preflight_test`; SHA-256 `b542b9aab44c4ce1cbbc4f3dd81f1c8ba6ff470fbb10a98440003491b2846644` |
| Accountable path | Exec formed team `Verification Output Readiness`; non-producing lead `web-quality` (Rowan Vale) supervised Staff producer `web-producer` (Mara Chen). The lead reviewed and escalated; all candidate and runtime-locator production is attributable to Staff. |
| Initial false green | Staff Attempt `648eea76-7530-45b9-9692-442b2bd7a460` produced the initial page for Work `88a206a4-fa05-42ef-8ff0-895818750d23`. Its gate `f43e6eca-880e-485e-9436-f47b7cf3461c` reported exit 0 while retaining both `EXP08_GATE_OUTPUT_OKn` and a curl connection-refused error. Lead native inspection independently got curl exit 7, declined handoff `f01829b8-521c-4fc1-bbb1-9b00b65bfe66`, and abandoned that Work. |
| Runtime repair | The first repair Attempt `c4f3f892-90f2-42ab-ab67-fb9326414f9b` failed honestly because `/company/repos/exp08-gate-output` did not exist. Lead commissioned separate Staff Work `dbb18f9b-c5aa-4805-95b6-12cb5b8b671c`, Attempt `1133e731-da73-4d18-bef1-6a6af0a8da16`, which created the exact symlink to authoritative `/company/projects/exp08-gate-output` and verified unchanged base commit `d40a5691664bc153f8d4ce032047605ee2e89e31`. |
| Corrected candidate | Corrective Work `0d6a0698-1b1a-47b6-9e03-056ac1c80be7`, Staff Attempt `7ef0f6c8-cb55-4311-a3f1-59c6d2a20ba4`, completed using that unchanged commit. Its `review_target` artifact `915fbfc8-2d05-41b7-8649-e6880ad89ebc` is the local URL `http://127.0.0.1:38009/`. Direct post-return Runtime inspection observed HTTP 200 and `<body>EXP08_GATE_OUTPUT_OK</body>`. |
| Corrected gate | `review-target-live-probe`, gate `439eb50a-c8db-4a81-b462-d394fc8ad1ea`, run `8486edff-075e-4e2a-8680-6368f14cc3f9`: `set -eu` made the curl/marker check fail closed before it could print success. It passed after the corrective Attempt and retained `EXP08_GATE_OUTPUT_OKn` in `gate_runs.output_excerpt`, which contains the required exact marker. The Staff-linked evidence artifact `Corrected fail-closed gate stdout` additionally records the marker on its own line, URL, body text, HTTP 200, exact commit, and clean repository. An adversarial equivalent against unused port 38019 exited 7 rather than falsely passing. |
| Owner review | Lead inspected the exact target without production edits, prepared owner handoff `f50d05ea-45f0-4873-a162-bea8396c5603`, and the normal Attention projection exposed it with the native target and gate evidence. Explicit deterministic acceptance closed the corrected Work at `2026-08-26T12:22:18.826278Z`. |
| Authority / health | `restless receipts` returned `[]`; accounted model spend was USD 0 against the USD 3 ceiling. Subscription list-price estimates in events are noncanonical. `restless doctor` reported the owner API, OrgIntel, Runtime, browser controller and bounded storage available. |

### Disposition

**Passed for validity gate 6.** The exact marker was retained in the owner-visible gate excerpt after a
post-return live-target probe, and the ordinary Exec → non-producing lead → attributable Staff →
native ReviewTarget → lead review → owner-attention → explicit acceptance path closed without an
external effect.

The literal trailing `n` in the product's stored gate excerpt is a small formatting defect caused by
the model-produced `printf` escaping. It does not change this disposition: the required exact marker
is visibly present, and the linked evidence artifact carries it as its own line. Future treatment
gates should quote the `printf` format string so their human-readable output has a conventional newline.

P1's output-less run is deliberately preserved as `product-invalid`; P2 does not rewrite it. This
passing readiness result is not an EXP-08 arm, does not establish a playbook effect, and cannot bypass
Sprint 20's terminal evidence or usable EXP-07 Restless-source evidence.
