# Exec's question card stays open until it is actually answered

Reported on Cloud (7 October, 02:17 UTC): the owner had Exec's card "Confirm A$4,800 or a different
price, and give the booking email or link" open, sent an ordinary message saying it was not an
answer, and the card folded to "You answered" with its fields gone, while Exec and the Inbox still
waited on the price and booking.

The card treated any later message from the owner as the answer. An answer already has a form of
its own, a reply quoting the question, which is what the card's taps, fields and Answer send. Now
only such a reply answers a question, and a newer question supersedes it; anything else leaves the
card open under its question, wherever it now sits in the thread. "Something else…" starts an
"Answering: …" reply instead of a plain message, so a typed answer closes it too.

The rule lives in lib/model/open-question.ts with a test of the reported case (an ordinary message,
then an agent reply in between, both leave it open), a quoting reply answering it, and a newer
question superseding it.

Checks: npm run check passed; open-question.test.ts 4 passed (the first two fail on the old rule).

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/open-question.test.ts`

```
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { answerIndex, excerptOf, openQuestionIndex } from './open-question.ts';
```
