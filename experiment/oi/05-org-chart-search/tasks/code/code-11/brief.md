# The model menu shows the five newest releases; "(latest)" aliases no longer pass as new

The menu kept the newest model of each family, reading the version only at the end of a name, so
"Claude Opus 4.5 (latest)" had no version, became a family of its own and sat beside Opus 5.5. The
version is now read wherever it is in the name (dates are not versions), a "(latest)" alias of a
model already listed is dropped, and the menu shows the five highest versions, the model in use
always among them, with the rest under Older models in the same order.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/model-groups.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { filterModels, release, splitModels } from './model-groups.ts';
```
