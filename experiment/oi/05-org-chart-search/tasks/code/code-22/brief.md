# Inbox Skip and Approve leave at once; Skip Undo restores in place

Sprint 109 C1, the set-aside and Authority slice. Skip is saved, then the
card leaves straight onto the next one; the full company read now follows
in the background instead of holding the card. Undo puts the card back
under the card it sat below at once, and every read keeps it there until
the source has restored it (or says it resolved meanwhile). Approve and
decline move straight to the next card with no empty frame between.
Skip, restore and Authority decisions are sent keepalive so one given as
the tab closes still arrives.

The decision is still recorded before the card leaves, per the C1
delivery contract; no page-owned timer or replay queue is added. A real
Undo window for Authority grants needs an Authority-held pending grant.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/inbox-restore.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { placeOf, reinstate, RestoringCards } from './inbox-restore.ts';
```
