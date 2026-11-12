# Abandoned Work no longer strands the Work that requires it

Readiness waits for a completed upstream, so Work that requires abandoned
Work can never start, and nothing said so: Frankensurf's "Prove which
agent tools actually work with Frankensurf" has sat proposed with 0
Attempts since its upstream was abandoned on 19 Oct.

Abandoning Work (by command or by an Attempt that abandons it) now tells
each open dependent's lead, in the same transaction, how to go on: drop
the requirement with `restless work edge ... --remove`, or abandon it too.
New requirements on abandoned Work are refused, both on an existing node
and when Work is created with initial edges.

S109 blockers audit, finding 9.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless-orgintel/tests/abandoned_upstream.rs`

```
use restless_orgintel::{NewWork, OrgIntel, WorkEdgeKind, WorkspaceSpec};
```
