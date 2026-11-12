# S101: an agent's question is answered in its card, with its own choices, and Discuss opens the thread there

Any chat that needs the owner, from Exec or a lead, is a card with the recent messages, the
question's own options as one-tap answers and a reply box; the reply goes to the agent as a
message and the card clears. Discuss on a handoff opens the same thread in the card instead of
another view.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/inline-reply.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { quickAnswers, replyBody } from './inline-reply.ts';
```
