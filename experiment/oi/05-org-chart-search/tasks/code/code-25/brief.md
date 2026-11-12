# Resumable Work resumes itself

A staff turn cut by a watchdog (no output for 9 minutes, a wedged or
stopped session) closes Blocked with "[resumable] ... the next wake
continues from durable state". Nothing read that prefix and blocked Work
is never claimed again, so the promised wake never came: 34 Attempts on
23 Work items in production, and the Restless company's Sprint 107 sat
blocked from 29 Oct to 3 Nov with no card.

The staff outcome recorder now resumes such Work through a new
OrgIntel::resume_after_interruption, bounded like the restart resume:
at most twice per revision, never past a pending owner handoff, each one
a work_resumed_after_interruption event. The owner's own Stop is not
resumed.

S109 blockers audit, finding 1.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless-orgintel/tests/interruption_resume.rs`

```
use restless_orgintel::{NewWork, OrgIntel, WorkAttemptState, WorkStatus, WorkspaceSpec};
```
