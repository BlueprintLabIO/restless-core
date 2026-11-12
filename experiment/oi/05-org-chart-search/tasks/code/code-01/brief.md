# restless decide: Staff ask a structured decision model for bounded triage

Repeated, bounded choices over many similar items (lead quality, support
routing, spam) cost a full model turn each today. `restless decide --state
... --questions ...` sends a state and typed questions (noul, choice, score)
through the plane to the hosted relay's decision model and returns answers
with probabilities in under a second. The daemon pins the request to the
authenticated actor (a runtime session cannot claim another actor) and logs
each decision as kind "staff" with its probabilities. The operating rules
say when to use it and never for quality judgement, writing or authority.

On labelled queue triage (16 inbound emails) decision models scored 15/16
on lead quality and 16/16 on routing.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless-engine/tests/decisions_shadow.rs`

```
use restless_engine::decisions::{ DecisionClient, EXEC_GATE, settle_exec_results, shadow_exec_results_with, };
use restless_orgintel::OrgIntel;
```
