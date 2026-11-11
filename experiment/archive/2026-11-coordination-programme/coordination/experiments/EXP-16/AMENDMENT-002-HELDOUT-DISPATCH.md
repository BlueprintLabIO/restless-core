# Amendment 002 — held-out evaluator dispatch repair

**Recorded:** 2026-08-30 06:34 UTC  
**Controller message:** 64  
**Classification:** `evaluation-infrastructure-invalid`

The first held-out validation Attempt stopped before any held-out product action. The frozen candidate's
delivery runner expected an obsolete controller release shape (`commitment_sha256` and `delivery`),
while the committed release contract uses `delivery_held_out` and `recovery_held_out`. The candidate
also had no recovery held-out dispatch mode. Those are evaluator defects, not NPC or game failures.

One symmetric harness-only repair is authorised from game/policy parent
`9251888f7af0582dbf9f998dde12740c58d4e1e8`. It must:

- accept only release schema `restless.exp16.held-out-release.v1` with exact file digest
  `117d9874812c6c1256ff607a37e3d6277908820c10011c4da2c670718c0bf237`;
- dispatch all six delivery and eight recovery held-out cases through ordinary paths;
- derive world/policy values deterministically from the released scenario identity and seed;
- make conformance placement depend on the released seed where the fixture permits;
- leave `npc/recovery_evaluator.gd` byte-identical at SHA-256
  `442eab665bf1bce9c4a2aaeb97407ede9a801933aa53f6abc9fc574acb0fc8aa`;
- report whenever a native mechanics seam does not recreate the complete injected placement;
- use the Runtime-leased port, retain product failures, and clean raw logs; and
- rerun held-out validation once, then stop before Stage 4.

The validation threshold remains at least five of six delivery completions and at least six of eight
recovery completions, with every recovery case terminating within its frozen bound. No product or
policy repair is permitted under this amendment.
