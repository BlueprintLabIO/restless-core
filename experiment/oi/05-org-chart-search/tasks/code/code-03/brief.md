# Work's 'Needs you' matches the Inbox; coordination outages say so

Work, Waiting on you, the goal readings, the rail's work-in-flight lines
and Library's review marks now count Work as waiting on the owner only
when the Inbox shows it: pending, unassigned, not being prepared, not set
aside. Judgement a lead or Exec still holds reads 'With <name>' instead.

When a company's coordination (OrgIntel) is unavailable, Work and a Work
link show a failure notice instead of 'No work yet' or 'Work not found',
and desktop notifications pause instead of re-announcing every item on
recovery. Switching companies no longer announces the other company's
items as new.

Checks: npm run check (svelte-check 0 errors, 0 warnings, type/ui/
document-search checks passed); node --experimental-strip-types --test
src/lib/model/attention-presentation.test.ts (3 pass).

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/attention-presentation.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { coordinationFailure, judgementHolders, workWaitingOnOwner } from './attention-presentation.ts';
import { describeFailure } from './failure.ts';
import type { AttentionItem } from './view';
```
