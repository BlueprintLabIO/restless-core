# UXG-054..061, UXG-063..065, UXG-068: Work list search, status words, board and row actions

Members' "Needs you" view reads their own handoffs; one word per state (Blocked, Needs you,
No goal); search lives in ?q=, matches owner/goal/output names and stopped work, and a miss
says so with a clear button; goal views prefill "Add work toward <goal>" and confirm new Work;
goal rows are links; the board is a labelled region with column headings, caps Done with a
show-all footer, puts Blocked first when stacked and spells out "Revision 2"; run states use
owner words and only show when they add information; rows get Resume, Discuss and Copy link.
pass; prettier --check clean; node --test src/lib/model/work-views.test.ts 3/3 pass. Not run live.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/work-views.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { emptyWorkTitle, matchesSearch } from './work-views.ts';
```
