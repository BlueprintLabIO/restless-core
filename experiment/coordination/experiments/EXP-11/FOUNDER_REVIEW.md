# EXP-11 founder review

**Recommendation:** `revise`  
**Review status:** Withheld because the frozen independent-usability gate failed  
**Candidate:** `41f4fa53a2cd05ab17aea473f3d1be28979b2dcf`

There is no founder acceptance target to play in this sprint. The strongest candidate is mechanically
improved and reproducible, but the final fresh clean-room player could not complete the intended
delivery after reaching the visible route end.

## What to inspect

Review the native evidence before any source or task narrative:

1. `results/referee-r19/output/shortcut-attempt-d6f3d5c5/step48-toward-route-end.png` — on foot,
   carrying the parcel at route zero, with `TRUCK JOURNEY REQUIRED`.
2. `results/referee-r19/output/shortcut-attempt-d6f3d5c5/after-shortcut-e.png` — the immediate explicit
   rejection after one `E` input.
3. `results/referee-r19/output/journey-attempt-b399989b/repair-step39-loaded-truck-start.png` — loaded
   driving begins at route zero.
4. `results/referee-r19/output/journey-attempt-b399989b/repair-step42-route-end-confirmed.png` — the
   rendered route-end state instructs the player to exit.
5. `results/referee-r19/output/journey-attempt-b399989b/repair-step43-on-foot-after-exit.png` — after
   exit, delivery remains in progress and the next action is not legible enough to complete.
6. `results/referee-r19/output/journey-attempt-b399989b/repair-step68-matched-reenter.png` through
   `repair-step72-unload-attempt-blocked.png` — the smallest retained re-entry, exit, destination, and
   failed-unload sequence.

The terminal player report is
`results/referee-r19/output/journey-attempt-b399989b/terminal-report.txt`. The independent lead's final
verdict is preserved in the R19 event/session evidence.

## Why review is withheld

The sprint contract requires two consecutive fresh independent sessions to complete the loop or
correctly identify the same remaining blocker. It received one conclusive strict final run, and that
run found the blocker. Asking the founder to play now would use founder time to rediscover a defect
already established by valid independent evidence.

## Decision

Record `revise` for the experimental candidate. Do not promote it into the Dogfood 4 source company.

The next review target is permitted only after one bounded repair makes route-end exit, destination
placement, and unload visibly coherent, the deterministic suite passes from the exact new commit, and
two fresh withheld-context runs complete the journey within one aggregate experiment budget.
