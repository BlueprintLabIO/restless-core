# Work and Inbox say why each item waits and on whom

S109 A1: every open Work row, board card and Work page carries one line
under its title, and every Inbox row's meta says it: "Waiting on you:
approve the brand rules", "Waiting on Baloo: running", "Waiting on a
model: connect a model for Baloo", "Waiting on other Work: ...". One
pure module (lib/work/waiting.ts) derives it from data already returned:
the owner queue or a member's passed handoffs, pending handoffs held by a
lead, unfinished required Work, the newest run, and the shared model
route state (routeActionLabel), so the line uses the rail's words.

The line sits in a reserved sub-row (Item note, board card line, Work
page line), so rows and cards never grow when it arrives, and it stays
empty until the model routes are read rather than naming a lead and then
switching. The blocker reason moves from the row meta into the line.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/work/waiting.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import type { AttentionItem } from '../model/view.ts';
import { actionPhrase, inboxWaitingLine, workWaitingLine, type WaitingSources } from './waiting.ts';
```
