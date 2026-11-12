# S88-01: the owner can archive and re-hire an agent

- OrgIntel restore_actor reverses retire_actor. The same identity returns,
  with its name, model, skills and history, and rejoins the team it left
  when that team still stands. Recorded as actor_restored with who and why.
- POST /companies/{c}/actors/{a}/archive and /rehire, owner only, with an
  optional reason. OrgIntel's refusals (a lead, open Work, a pending
  judgement) come back as sentences with 409.
- Both are recorded as setting changes, so they show in Activity
  ("<name> archived: <why>", "<name> re-hired"), and Exec sees them under
  Changed by people.

Tests: actors_and_teams 14 pass (re-hire assertions added).

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`crates/restless-orgintel/tests/actors_and_teams.rs`

```
use restless_orgintel::{ ArtifactRefState, InitialWorkGate, NewArtifactRef, NewAttemptRecovery, NewWork, NewWorkGate, OrgIntel, OutcomeStandard, OutcomeStandardSource, WorkAttemptState, WorkspaceSpec, };
```
