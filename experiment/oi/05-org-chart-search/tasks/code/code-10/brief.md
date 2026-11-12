# A persisted Inbox summary no longer reveals the cockpit a step early

The multiplayer smoke caught a member's cockpit appearing with the persisted Attention
summary and then adding the clear pill and the floor when the live read landed: the
first-paint barrier waited only for reads with no data. A persisted placeholder (sources
unknown) now holds the reveal while its live read is in flight.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/first-paint-hold.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { holdsReveal } from './first-paint-hold.ts';
```
