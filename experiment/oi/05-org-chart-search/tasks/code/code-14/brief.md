# Work: a Work item's run label follows its newest run

The Work page sorted runs by run number only. A run's number restarts at 1
when its Work moves to a new revision, so an old revision's third failed run
sorted after the new revision's first finished one, and finished Work kept
reading 'Failed'. One shared ordering (revision, then number, then start time)
now picks the newest run on the Work page, the board and the office.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/work/status.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { compareAttempts, newestAttempt, runStateLabel } from './status.ts';
```
