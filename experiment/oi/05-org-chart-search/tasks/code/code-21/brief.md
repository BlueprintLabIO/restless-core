# Let agents on the company default follow a new default

An agent with no route of its own saved a copy of the default as its
model preference (Exec on its first wake, Staff on their first run), and
agent_preference kept reading that copy while the default stayed on the
same provider. Switching the default from Opus to Sonnet left Exec and
every hired agent on Opus.

When the default changes (Models editor, an account made the default,
company setup), clear preferences that equal the outgoing default so
those agents read the default again. A different model Exec chose for an
agent, and any agent with the owner's own route, keep theirs.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless-orgintel/tests/actors_and_teams.rs`

```
use restless_orgintel::{ ArtifactRefState, InitialWorkGate, NewArtifactRef, NewAttemptRecovery, NewWork, NewWorkGate, OrgIntel, WorkAttemptState, WorkspaceSpec, };
```
