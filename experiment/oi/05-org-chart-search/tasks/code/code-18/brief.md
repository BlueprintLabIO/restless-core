# S109-G5: agents' working notes leave the default Library views

Files under /company/org, notes outside the outputs and downloads folders,
and anything in a scratch folder are working notes. They leave All files and
the kind views for their own "Working notes" view in the sidebar and the
narrow view strip; one waiting on the owner's review stays in Needs your
review, and a pinned one stays pinned.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/library-files.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { isWorkingNote, libraryFileById, libraryFiles, replacedFile, type LibraryArtifact, type LibraryFile } from './library-files.ts';
import { retiredWorkIds } from './library-files.ts';
import { workFileLinks } from './inbox-files.ts';
```
