# Cold company load paints its own layout from the first byte

A full load of a company page showed the brand's dot field until every
critical read had answered: about 2-3 s of empty dotted page on restless.run
(2 Nov; Reno /work, first paint 0.40-1.18 s, reveal 2.10-3.16 s). The owner's
rule is no blank frames: the final layout appears at once.

Each settled company page now records where its surfaces sit (top bar, dock,
panes) and in which colours (lib/model/shell-snapshot.ts; geometry and colours
only, never text), once revealed, after each navigation and as the tab is
left. app.html draws that layout as plain boxes before any script or
stylesheet loads, when the window width and theme match; otherwise it draws
the top bar alone. The reveal fills panes already on screen, without the
fade that would blink them out first. Other pages keep the field.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/boot-preload.test.ts`

```
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';
import { shellFor, SHELL_KEY, withSection, type ShellSnapshot } from './shell-snapshot.ts';
```

`web/src/lib/model/shell-snapshot.test.ts`

```
import assert from 'node:assert/strict';
import test from 'node:test';
import { isPane, shellFor, shellPanel, shellSection, withSection, type ShellSnapshot } from './shell-snapshot.ts';
```
