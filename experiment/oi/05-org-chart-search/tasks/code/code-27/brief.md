# Owner cards: titles never carry a URL, no computer panel without a prepared computer

A sign-in step titled with its whole Cloudflare OAuth URL rendered as a ten-line heading and
embedded a broken computer panel with an internal error. Titles are stripped of links and
bounded in the engine and the web guard, clamped to two lines; a step with no prepared computer
shows its plain instruction and the Open action; a failed computer shows one plain line.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/owner-copy.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { decidedNotice, leaksInternals, ownerLine, ownerText, RESOLVED_ELSEWHERE, scrubInternals, ownerTitle, withoutLinks } from './owner-copy.ts';
import { blockerSentence } from './work-actions.ts';
```
