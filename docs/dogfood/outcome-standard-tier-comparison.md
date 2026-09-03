# Outcome Standard four-tier landing-page dogfood

**Run date:** 31 August 2026
**Harness:** Restless daemon and OrgIntel schema 25
**Publication boundary:** local review only; no deploy, publish, push, outreach, purchase, or other external effect

## Frozen input

Each `_test` company received the byte-identical owner message below. The composer supplied no
per-message override, so the company setting was the sole intended variable.

> Create one standalone landing page at `/company/outputs/outcome-tier-site` for Restless Cloud. Use
> the real product and design system as the source of truth. It must explain the value in simple,
> outcome-focused language, feel mature and distinctive, and include responsive motion or graphics
> with a complete reduced-motion state. Start the candidate directory empty; do not copy a prior
> candidate. Operate and verify it in its native browser environment. Do not publish, deploy, push,
> contact anyone, or perform another external effect. Return one exact openable local ReviewTarget
> only when this outcome’s selected standard is met.

The four companies shared the same model policy, reasoning setting, mission, company spend ceiling,
empty final-candidate path, and local-only authority. They differed only in `outcome_standard`:
`fast`, `thorough`, `exceptional`, and `frontier`.

## Observed operating behavior

| Standard | Team shape chosen by Exec | Material behavior observed |
| --- | --- | --- |
| Fast | lead plus one website producer | Three compact directions, one selected production build, native desktop/mobile/reduced-motion evidence, then one independent critic. The critic reopened the build for a skipped first-load hero, one factually wrong command example, and an over-fast automatic sequence. |
| Thorough | lead plus product/brand, engineering, and critique specialists | Product grounding and three rendered directions, a complete selected build, independent native review, one contrast repair, a second review of the exact repaired candidate, then admission. |
| Exceptional | lead plus brand/web production and independent critique | Three rendered directions, full build, browser-behavior gate, independent comparison with the incumbent, and a separate accessibility audit before admission. That audit found 59 unique WCAG AA contrast failures, two focus-state failures, and mobile-menu focus escape, causing a root-cause revision rather than a premature owner handoff. |
| Frontier | lead plus brand experience, design engineering, and independent critique | A separate evidence baseline, three materially distinct rendered experiments, and independent selection. Two oversized critic attempts exceeded the provider context limit; the lead preserved the valid captures, abandoned the unbounded review, commissioned a 15,203-byte decision packet, and restarted selection from only decision-bearing evidence. |

These are emergent plans, not fixed tier-to-topology recipes. Fast still retained truth, usability,
accessibility, and native-operation floors. Higher standards spent their extra effort on stronger
grounding, independent challenge, broader experiments, and evidence-driven loops.

## Harness finding and repair

The live run exposed a repeatable inefficiency: after a provider context overflow, a successor
Attempt could see durable artifacts but was not explicitly told to avoid reconstructing the same
high-volume browser and media corpus. Sprint 29 now carries the newest prior Attempt's bounded
terminal state into the launch membrane. When its summary is a `[context]` failure, the successor is
told to resume from durable files and compact manifests, avoid repeated capture and broad scans, use
targeted probes only, and ask the lead to split capture from judgement when one bounded pass cannot
fit. Normal Attempts do not receive this recovery instruction.

This is intentionally recovery posture, not a new workflow or quality lifecycle. The Work graph,
artifacts, Attempt states, and accountable lead remain authoritative.

The run also found an admission-state defect. Gate retirement preserved the malformed declaration,
but the original table-wide `(work_id, name)` uniqueness constraint prevented a corrected active
gate from reusing the reserved `review-target-live-probe` name. The Runtime then correctly refused
owner review because no active passing gate carried that contract name. Migration 0024 moves
uniqueness to active gates only. Historical rows and runs remain unchanged; exactly one non-retired
gate may use a semantic name. The live Thorough test schema was repaired transactionally, the valid
replacement probe was renamed, and one attributable retry was restored through ordinary Work
resume semantics.

Frontier exposed a third quality-loop defect. A repaired candidate was initially judged by a report
whose contract still pinned the pre-repair commit. The lead eventually replaced that review with a
current-commit contract and an exact-identity gate, but not before one stale verdict re-invalidated
already-repaired work. The standing accountable-lead doctrine now requires every evaluation to name
the exact commit, digest, artifact version, or runtime generation it operated; a revision invalidates
prior verdicts even when paths are unchanged, and deterministic identity must gate acceptance when
available.

The final handoffs exposed a fourth, smaller projection gap. An accepted ordinary Work can link a
`review_target` as evidence without creating owner Attention; therefore a lead can possess the right
target but still fail the owner's request for an openable outcome. The doctrine now treats exact
review delivery as its own verifiable contract: close through one `--owner-review` Work, then inspect
the owner projection and prove that exactly one current available ReviewTarget is present before
declaring the charter complete. The two affected live runs were projected through ordinary native
handoffs without changing their sites.

## Verification

- `cargo test -p restlessd`: 232 passed, 0 failed, 8 environment-dependent tests ignored.
- `cargo test -p restless-orgintel`: all unit, integration, and doc tests passed.
- `cargo build --bins`: the production binaries compiled against schema 25.
- `web/pnpm check`: 0 errors and 0 warnings; the type-ramp check passed.
- `web/pnpm build`: static production build completed.
- `git diff --check`: passed.
- The focused recovery test proves that ordinary launches omit recovery posture and a `[context]`
  successor receives it with explicit context accounting.

Attributable per-outcome spend was unavailable in this OAuth-backed run: each company projection
reported zero accounted USD despite observable model activity. The result must therefore be recorded
as **unattributed**, not free, and cannot support a cost ranking between tiers.

## Owner review targets

All four links use the same owner-cockpit review path and issue a fresh isolated-origin ticket for the
company-local site. They remain local-only and require the Sprint 29 test Runtime at
`127.0.0.1:7788`; none is a public deployment.

| Standard | Distinct first-view thesis | Exact owner review link |
| --- | --- | --- |
| Fast | “Put a mission to work. Keep the proof.” | [Open Fast](http://127.0.0.1:7788/restless_tier_fast_test?review=orgintel%3Ahandoff%3Ab3d304fe-9fde-4669-bfb5-b3f0ec8ba6ab) |
| Thorough | “Work outlives the turn.” | [Open Thorough](http://127.0.0.1:7788/restless_tier_thorough_test?review=orgintel%3Ahandoff%3Aeffad28f-ca57-462c-9cce-c9081c3b8185) |
| Exceptional | “A company that keeps its thread.” | [Open Exceptional](http://127.0.0.1:7788/restless_tier_exceptional_test?review=orgintel%3Ahandoff%3Ae14dff15-f198-4954-a877-35c56ce48720) |
| Frontier | “Run an AI company you can inspect.” | [Open Frontier](http://127.0.0.1:7788/restless_tier_frontier_test?review=orgintel%3Ahandoff%3A9f0d12f9-2e4a-40bd-bde8-5b362110984b) |

Post-restart in-app Browser verification opened every exact cockpit link against the final schema-25
Runtime, observed one visible review region and the expected iframe heading, and found no browser
console errors. Every Attention source reported its ReviewTarget `available`. The four retained tabs
were refreshed after Runtime restart and marked as user-facing deliverables.

## Comparative diagnostics

| Standard | First Work to owner ReviewTarget | Context-limit Attempts | `changes_requested` Attempts | Observed loop character |
| --- | ---: | ---: | ---: | --- |
| Fast | about 1h 28m | 2 | 1 | Direct production plus one bounded truth/usability repair. |
| Thorough | about 1h 58m | 0 | 1 | Broader grounding and independent review; admission exposed the retired-gate defect. |
| Exceptional | about 1h 30m | 0 | 1 | Separate incumbent comparison and accessibility audit found 59 contrast failures before handoff. |
| Frontier | about 5h 52m to owner projection; accepted in about 4h 58m | 5 | 2 | Broad experiments, full-size evidence repair, exact-identity re-review, compact reconciliation, and an operational handoff gap. |

Elapsed time includes Runtime recovery and handoff work; it is diagnostic, not a quality score. Fast,
Thorough, Exceptional, and Frontier respectively produced 8, 13, 11, and 18 terminal Attempts in
the final graph. The Frontier cost was not merely “more polish”: it discovered context scaling,
stale-verdict identity, evidence-legibility, and owner-projection defects that the lower tiers did not
exercise.
