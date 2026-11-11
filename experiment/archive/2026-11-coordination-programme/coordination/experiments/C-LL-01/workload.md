# Workload C-LL-01 — Prism Warden progression slice

Status: frozen for matched run

## Frozen success contract and native review target

The exact contract is `scenario.md`. The native target is the running basin → cavern gate → authored
boss battle → unlocked-gate loop.

## Domain, artifact type and why the work is real

Coding/product. The result adds the first authored progression encounter to the playable game and
connects its world presence, eligibility, battle rules, phase transition and persistent resolution.

## Starting artifact/ref and scenario hash

- Repository seed: `514b7b3d0a65e093af608b08ca142344412181f4`.
- Scenario SHA-256: `90457d619e0e051f8070f9f3e878bf76a5bbe26ac808f42dd36b70b63446efd2`.
- Evaluator SHA-256: `1063015de6b0f22fbf86270c0879ce6ca373e6f9dba37f64cc752061ef3085e7`.

## Size prediction — judge 1

- Expected strong-lead wall time/tool calls: 25–55 minutes; roughly 22–45 tool calls.
- Evidence: the seed contains only a cavern coordinate and optional trainer hook. The slice must add a
  world actor/prompt, priority encounter routing, boss-aware battle state, a timed phase attack,
  resolution/progression behavior and whole-loop browser proof across the same mutable lifecycle.
- Small / medium / large judgement: large for this seed. The operating envelope is 7,200 seconds so
  it does not become the semantic completion mechanism.

## Structural feature judgement — judge 1

| Feature | Observation and locator | Judgement |
|---|---|---|
| Independently acceptable artifact seams | Landmark, battle identity, phase attack and resolution are individually inspectable but none delivers the owner loop without the shared boss lifecycle. | Low |
| Dependency width / critical-path depth | Eligibility → prompt → battle start → phase transition → resolution → non-repeatability is mostly sequential. | Narrow/deep |
| Shared mutable-state surface | `Game.mode`, team/progression flags, encounter target, `Battle.phase`, enemy health, HUD and scene cleanup all participate. | High |
| Interface stability | Wild battle interfaces exist, but no authored encounter/boss contract exists between Game and Battle. | Low-to-medium |
| Independent verifiability | Individual states can be probed, but the decisive proof is the integrated native loop. | Medium |
| Specialist diversity | World presentation and combat behavior differ, but both must negotiate one encounter identity and lifecycle. | Medium |
| Tool/external latency available to overlap | Source inspection and bounded browser probes may overlap; most implementation and proof follows the same state chain. | Low |
| Breadth uncertainty / coherence requirement | Several plausible designs exist; one consistent player-facing progression model must win. | High coherence |
| Bad-contribution detection and repair cost | A clean-looking world or battle contribution can still hijack wild targeting, double-resolve rewards or leak scene state. | High |

## Candidate decomposition without assigning an arm

A bounded contributor might build the gate presence and eligibility/prompt while the lead owns the
battle and progression lifecycle, or might build a boss-aware Battle extension behind a negotiated
start/resolution contract. Both cuts require an early interface decision and integrated review; there
is no obvious artifact that is independently acceptable without that agreement.

## Judge 1 / Judge 2 disagreement and reconciliation

Judge 2 was a fresh read-only `gpt-5.6-terra` session. It saw only the exact seed and owner contract,
not this card or evaluator. It classified the task as large within a 60-minute expectation and judged
the proposed large/high-coupling label fair. Its evidence matched Judge 1 on the mostly serial chain,
high shared state, low-to-moderate interface stability, high coherence/repair cost and limited
beneficial parallelism. It identified only the gate visual as a meaningfully separable early artifact;
the other seams require an agreed boss contract across `Game` and `Battle`.

The material disagreement is expected duration. Judge 2 estimated 75–115 minutes and 30–50 tools,
versus Judge 1's 25–55 minutes and 22–45 tools, because it priced the native visual loop, exact pulse
timing and regression proof more heavily. Its strongest alternative was medium/serial in a small
codebase if the lead reuses an existing creature and battle primitives.

Reconciled prediction: **large for the experiment, 35–100 minutes and 22–50 tools; low potential
parallelism with one plausible world-presentation seam, a deep integrated critical path and high
whole-loop proof cost.** Judge 2 used 306,256 input tokens (238,848 cached), 3,208 output tokens and
863 reasoning tokens. This is pre-run design cost, not arm cost.

## B0/B1 arm order and fairness controls

Order seed: `C-LL-01:v26:ordinary-crossover:2026-08-23`; SHA-256
`1358de66e98c038c13f78f57061a048daf520f8159e93e2f54bc0f1aef3fc147`. Even first-32-bit parity
selects **B0 → B1**. Both arms receive the same seed, scenario bytes, Sol lead, tools, 7,200-second
outer envelope, 120-second drain, no actor timeout, USD 6 nominal ceiling and evaluator hash. B1 alone
receives one `gameplay-systems` producer on pinned `stealth/ox-alpha` and one Staff slot.

## Evaluator and blind-review contract

`evaluate.mjs` is frozen outside actor mounts and executed against an isolated candidate export. The
seed passes only 5/23 checks: team arrangement plus the four ordinary-wild/no-error regression checks.
It fails 18 absent boss/progression checks with zero browser errors. A positive control is impossible
before production without constructing the answer; the first arm's candidate will remain evidence,
not retroactive evaluator design input.

## Post-run correction of predicted size and parallelism

Both arms met the 23/23 frozen contract, but B0 reached its decision in 7.9 minutes and one lead turn.
The two judges' 35–100 minute reconciliation overpredicted realised solo wall time by roughly 4–13×
(and Judge 2's upper estimate by about 14×). Tool volume was not similarly small: B0 still used 50
tool calls, 2.27 million input tokens and changed 433 lines across five files.

The structural prediction was stronger than the duration prediction. The work had a deep shared
lifecycle, and the only useful parallel overlap was the anticipated world/gate versus battle seam.
B1 realised about 279 seconds of overlap, but its producer did not deliver its first artifact before
the solo arm had already completed. It also crossed into shared state before narrowing its commit,
then required evidence-based repair and lead integration.

Post-run classification: **broad and high-coupling, but below one strong lead's effective-session
saturation point**. Nominal breadth, estimated minutes, line count and even tool count are weak size
proxies on their own. Future “large” cells must freeze an observable saturation criterion—such as
multiple independently acceptable artifacts whose serial critical path exceeds one lead session—then
use the B0 pilot to confirm it before spending the matched B1 arm.
