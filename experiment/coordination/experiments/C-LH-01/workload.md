# Workload C-LH-01 — Lumaara field-research desk

Status: re-frozen for Experiment Sprint 01 matched run

## Frozen success contract and native review target

The exact contract is `scenario.md`. The native target is the starter screen → Field Atlas → return →
Squad Workshop → return → unchanged game loop.

## Domain, artifact type and why the work is real

Coding/product. The result is two owner-visible companion applications over the game's real roster and
rules, not fixtures or coordination prose.

## Starting artifact/ref and scenario hash

- Repository seed: `514b7b3d0a65e093af608b08ca142344412181f4`.
- Scenario SHA-256: `bfda2c7d47285c794a9d4284f22164a42fbb5810870894b90c2a00e2de916c12`.
- Evaluator SHA-256: `dba9e9ff15047b4c20f43562efa9e6dc29d699fe7fb4ff0fcd913f9de1bda5d8`.

## Size prediction — judge 1

- Expected strong-lead wall time/tool calls: 30–70 minutes; roughly 35–70 tool calls.
- Evidence: each tool has its own substantial state model, interactive UI, responsive/accessibility
  work and browser proof. The Atlas combines roster search/filter/detail/evolution/comparison semantics;
  the Workshop combines ordered editing, rule-derived forecasts/analysis, durable named saves and
  atomic JSON exchange. Existing game regressions add a shared proof tail.
- Size judgement: intended large by serial owner latency; realised size will be corrected after B0.

## Structural feature judgement — judge 1

| Feature | Observation and locator | Judgement |
|---|---|---|
| Independently acceptable artifact seams | Atlas and Workshop each have a direct native review target and complete standalone owner value. | High; two large seams |
| Dependency width / critical-path depth | Both consume the frozen roster/config modules; otherwise their feature and proof paths are parallel. | Wide, two medium-depth paths |
| Shared mutable-state surface | Only the starter-screen links and read-only domain exports are shared. Tool state and storage are explicitly separate. | Low |
| Interface stability | `SPECIES`, abilities, evolution declarations, `STRENGTH` and `elementMult` already exist and are executable. | High |
| Independent verifiability | Each page has its own semantic review handles and browser behavior; either can pass before integration. | High |
| Specialist diversity | Atlas information architecture/comparison and Workshop state/rules/persistence reward related but distinct craft. | Medium |
| Tool/external latency available to overlap | Two page implementations and two browser suites can proceed concurrently; only launch links and regressions converge. | High |
| Breadth uncertainty / coherence requirement | Visual family should agree, but each tool can choose its internal design independently inside a frozen semantic contract. | Medium-low coherence burden |
| Bad-contribution detection and repair cost | DOM handles and atomic behaviors localise failures; shared-domain drift is detectable by exact roster/rule checks. | Low-to-medium |

## Candidate decomposition without assigning an arm

One actor can deliver the complete Atlas page/module/proof while another delivers the complete
Workshop page/module/proof. The lead owns the two starter-screen links, combined regression and final
visual coherence. Neither actor needs to negotiate an evolving feature interface with the other.

## Judge 1 / Judge 2 disagreement and reconciliation

Judge 2 was a fresh read-only `gpt-5.6-terra` session. It saw only the exact seed and owner contract,
not this card, evaluator or prior run evidence. It agreed that large/high-separability is a fair label:
the Atlas and Workshop are independently acceptable artifacts over stable read-only domain exports,
with low shared mutable state, strong independent browser verifiability and roughly 60–70% potentially
overlapable implementation. It rated the late shared shell, mobile/accessibility coherence and full
regression tail as moderate coupling and medium-to-high repair cost.

The duration disagreement is material. Judge 2 estimated 10–15 hours and 50–85 tools, versus Judge
1's 30–70 minutes and 35–70 tools. Its strongest alternative is a medium single-app extension whose
late integration work serialises enough to erase the apparent parallelism. The 10–15-hour estimate is
retained but discounted as an operating forecast because a Sol lead completed the prior 433-line,
50-tool integrated seed change in 7.9 minutes; the second judge did not see that evidence.

Reconciled prediction: **large/high-separability as a work shape; 40–180 minutes and 35–85 tools for
B0, with two independently reviewable feature paths and a moderate shared proof tail.** The 14,400
second envelope accepts the duration uncertainty without becoming semantic completion. Judge 2 used
62,365 reported tokens. This is pre-run design cost, not arm cost.

## B0/B1 arm order and fairness controls

Order seed: `C-LH-01:v27:ordinary-crossover:2026-08-23`; SHA-256
`04d1c44d0edce1beca8f0a68a0ec468e06b09dfcdf326e4b686f88bea45ec92d`. Odd first-32-bit parity
selects **B1 → B0**. Both arms receive the same seed, scenario bytes, Sol lead, tools, 14,400-second
outer envelope, 120-second drain, no actor timeout, USD 6 nominal ceiling and evaluator hash. B1 alone
receives one `experience-presentation` producer on pinned `z-ai/glm-5.2:free` and one Staff slot.

### Experiment Sprint 01 fresh allocation

The owner removed the free-provider variable. Two new independent read-only judges saw only the exact
scenario and seed. GPT-5.6 Sol estimated 6–10 focused hours and 35–70 meaningful iterations; GPT-5.6
Terra estimated 8–14 hours and 40–80 calls. Both classified lead saturation and artifact separability
high, interface volatility low-to-medium, and the Atlas vertical slice as the fairest one-worker seam.
Both judged the robust release-quality outcome credibly beyond one effective strong-lead session while
retaining a narrower single-lead implementation as the strongest alternative.

Fresh order seed: `EXP-01:E01:C-LH-01:gpt56:2026-08-23`; SHA-256
`b0a88b7d6fb85ce3a5fda1eb7ec6039dd2430f649bf27cccfc889667980db5cc`. Odd first-32-bit parity
selects **B1 → B0**.

- Accountable lead in both arms: `gpt-5.6-sol`, medium reasoning.
- B1 producer: `experience-presentation` on `gpt-5.6-terra`, low/runtime-default reasoning.
- Same seed, scenario/evaluator bytes, tools, 14,400-second outer envelope, 120-second drain, no actor
  timeout and USD 6 nominal ceiling.
- B1's one fair seam is the complete Field Atlas vertical slice; the lead retains Workshop, shared
  launch/domain conventions, integration and combined native proof.

## Evaluator and blind-review contract

`evaluate.mjs` is frozen outside actor mounts and executed against isolated candidate exports. It
tests Atlas and Workshop independently before native return navigation and core game rules/battle.
The final seed negative control passes 7/53 checks, fails 46 intended absent-feature checks and records
zero browser errors.

A fresh Sol evaluator audit found four material pre-run defects: partial candidates could abort with a
variable check count, the public JSON schema was underspecified, return links were not exercised, and
core game behavior was too weakly sampled. Those were corrected before the final hash. It also
identified broader quality/accessibility requirements that remain native-proof and blind-review
evidence rather than pretending one DOM proxy exhausts them. The audit used 26,634 reported tokens;
this is design/evaluation overhead, not arm cost. A later blind review compares interaction clarity,
responsive coherence and source-truth discipline without producer reasoning.

## Post-run correction of predicted size and parallelism

Pending.
