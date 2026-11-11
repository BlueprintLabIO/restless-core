# Amendment 004 — Runtime-owned gate evidence ordering

**Recorded:** 30 August 2026 07:29 UTC  
**Classification:** `runtime/harness-failure`

The first repaired held-out validation did not produce a held-out threshold result. Its candidate was
ready, but the Attempt brief required compact outputs to be linked before returning `outcome_met`,
while the sole declared Runtime gate is what creates those outputs. Repeating that contradiction is
not product evidence.

Sprint 26 amendment 002 corrects only the completion brief for non-review repository Work. Runtime
continues to bind the exact clean terminal commit/tree, then executes the declared gate, then records
the terminal Attempt. One fresh validation from harness candidate
`a7864155daa6364c132a562cf6e4f41fc2a80f77` is authorised under the unchanged release digest, frozen
policy hash, 5/6 delivery threshold, 6/8 recovery threshold, action bounds and Stage 4 stop.

No model or operator may run the gate manually, link a placeholder, change product/policy/seed bytes,
or reinterpret earlier partial rows as a pass.

This amendment closed on 30 August 2026. After a one-line recovery-wrapper timer initialisation and a
provider-invalid Attempt, the clean candidate was frozen at
`1804cbcdf721286813a4b8ce8f1dc25211f705cc` / tree
`eab1f3c3b39460ae5e5883cfd96f35a6776df4b4`. Staff reported only candidate readiness; Runtime gate
`b1bd3b52-3173-4b2c-ba94-0ff27a30b48f` then owned the run and compact output generation.

The governed result was delivery `6/6`, recovery `7/8`, all cases bounded, zero authority violations,
zero manual interventions, zero raw media and no leaked process. `REC-OSC-H` retained the intended
bounded terminal packet rather than pretending to complete. This accepts S2/S3 mechanics only; it is
not evidence of playability, fun, human legibility or player acceptance.
