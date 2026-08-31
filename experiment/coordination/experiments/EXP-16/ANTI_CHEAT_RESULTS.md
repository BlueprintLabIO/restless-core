# EXP-16 S1 anti-cheat results

## Result

Candidate `0e02451bf99c7d53981ebb14afbc4949b789eac9` / tree `47efc95514b40975a84f5f554800f71a1df8f199` preserves the bounded implementation and corrects the port contract. `verify-npc.sh` now requires the Runtime-injected `RESTLESS_GATE_PORT` in both modes and forwards that exact value to network execution; it does not hard-code or infer a gate port.

Missing-variable probes for `body` and `anti-cheat` both exited 1 before Godot execution. Uncounted local checks with explicit ports observed:

```text
RESTLESS_GATE_PORT=26381 bash verify-npc.sh body       -> PASS verify-npc body
RESTLESS_GATE_PORT=26382 bash verify-npc.sh anti-cheat -> PASS verify-npc anti-cheat
```

The final versioned scenario, run separately with explicit uncounted `S1_NETWORK_PORT=26383`, returned `mechanical_status: verified`. These observations are useful runtime evidence, but they are not counted deterministic-gate acceptance. The registered gates have `resources:[]` and cannot supply an authoritative leased port. Exact acceptance is blocked until the lead retires them and installs replacement port-resource gates.

## Runtime fault matrix

| Fault | Injected forbidden behavior | Expected gate | Observed gate | Result |
| --- | --- | --- | --- | --- |
| CHEAT-01 | walk intent authority path changed to `direct_transform`, with direct transform delta | movement-authority violation | movement-authority violation | governed action rejected |
| CHEAT-02 | interaction receipt attempted direct `mission_complete` delta | outcome-mutation violation | outcome-mutation violation | governed action rejected |
| CHEAT-03 | pickup receipt attempted direct `cargo_holder_direct` delta | cargo-authority violation | cargo-authority violation | governed action rejected |
| CHEAT-04 | evaluator observation included `hidden_destination` | observation-scope violation | observation-scope violation | observation rejected |
| CHEAT-05 | terminal action omitted source observation digest | receipt-lineage violation | receipt-lineage violation | receipt rejected |
| CHEAT-06 | recovery requested beyond the five-step ladder | progress-bound violation | progress-bound violation | terminal failure packet emitted |
| CHEAT-07 | controlling peer 2 claimed a fire result without authoritative confirmation | host-authority violation | host-authority violation | material result rejected |

The clean BODY-01 through BODY-10 suite passed through the same guard implementation. Clean action dispatch rejects mismatched adapter acknowledgements, and material client actions require host confirmation.

## Supplementary source audit

A case-sensitive search under `npc/` for direct transform/position assignments, mission-state assignment, cargo-holder assignment, direct damage assignment, unbounded `while true`, setters, and teleport calls returned no matches. Runtime injection above is the primary evidence.

## Evidence

- `/company/outputs/exp16/S1_SCENARIO/faults-results.json`
- `/company/outputs/exp16/S1_SCENARIO/s1-summary.json`
- `/company/outputs/exp16/S1_SCENARIO/network-host.json`
- `/company/outputs/exp16/S1_SCENARIO/network-client.json`
- `/company/outputs/exp16/S1_SCENARIO/run-manifest.json`
- `/company/outputs/exp16/RUNS.jsonl`

## Gaps and interpretation boundary

The fault suite proves the declared S1 guards turn red for the seven planted fixtures and that the clean suite passes. It does not prove that future role code cannot introduce a new bypass; those profiles must continue to execute through this shared body and rerun the gates. It does not prove delivery completion, production combat fairness, playability, legibility, human acceptance, or fun.
