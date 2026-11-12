# Deliver team results without waiting on an empty queue

- Machine mail still settles to coalesce a burst into one wake, but only while something could add
  to it: Work running or ready, or a turn holding its lease. With nothing in motion it is owed at
  once, so a lone result no longer waits out the 20-second quiet window.
- After an Exec turn, settling mail is looked at again at once rather than when its window ends.
- A turn that delivers a result or report stores its reply as soon as it is complete, before the
  agent session is torn down, as an owner's reply already was.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless-orgintel/tests/mail_settle.rs`

```
use restless_orgintel::{ MailSettle, NewWork, OrgIntel, ProducingTopology, WorkAttemptState, WorkspaceSpec, };
```
