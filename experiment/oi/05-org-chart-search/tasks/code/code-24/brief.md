# F7: queued sends wait above the composer, distinct from sent messages

A message sent while the agent's reply is under way (send after this reply)
is no longer drawn as a sent bubble with a "Queued" tag. It waits in a
fixed-height row above the composer: dashed outline, clock, "After this
reply", a one-line excerpt, and Edit / Cancel. Queuing, cancelling and a
failed cancel change the row's fixed label slot in place; the transcript
does not reset its scroll or anchor for a queued send.

Cancel deletes the unread Room message (the agent inbox skips deleted
messages). Edit takes it back into the composer; the next send queues
again while the reply is still under way. A message with files can be
cancelled, not edited.

The conversation query keeps following the reply under way when a
message is queued behind it (it used to switch to the queued message,
dropping the live reply), and follows the earliest queued message once
that reply ends. An interrupting send clears the queue.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/queued-sends.test.ts`

```
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nextFollow, queuedMessages, queuedRowState, queuedScheduleLabel, turnEnded, turnUnderWay } from './queued-sends.ts';
```
