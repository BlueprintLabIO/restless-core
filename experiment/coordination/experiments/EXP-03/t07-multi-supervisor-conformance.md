# EXP-03 multi-worker supervisor conformance

**Date:** 24 August 2026
**Run:** `exp03-supervisor-multi-architecture-r2`
**Result:** 9/9 deterministic checks passed

The experiment harness now admits a frozen roster of distinct worker identities without changing the
production architecture. The accountable lead remains non-producing.

Proved before any multi-worker model call:

1. the roster is explicit and attributable while concurrency is a separate bound;
2. two independent Work items can be leased to distinct workers concurrently;
3. downstream synthesis receives exact commit artifacts, not only prose summaries;
4. a clean partial worker artifact cannot satisfy the company protocol;
5. every frozen worker owns exactly one produced commit Work;
6. the final candidate is one exact downstream worker artifact;
7. the Work dependency closure of that final artifact covers every contribution;
8. the supervisor owns no production Work; and
9. the coordination database remains healthy.

This is experiment-only admission machinery. It does not introduce a production workflow engine,
fixed organisation chart or new OrgIntel entity.
