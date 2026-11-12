# Let only the slow company read stop holding the reveal

A cap on the whole barrier, at 300 ms and then 3 s, made the zero-jank
gate's slow Inbox reveal before its chained reads answered, so the Inbox
detail's folds moved (they did not on main, run side by side). The read
that is slow on Cloud is the company read, which carries Health's live
checks, and every page that shows it has a skeleton in its place. That read
now holds the reveal for 2 s at most; every other read holds it as before.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/first-paint-hold.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { HOLDS_AT_MOST, holdsReveal, surfaceOf } from './first-paint-hold.ts';
```
