# Open the model editor on the live choice and show switches at once

Owner, 31 Oct: the Company default row read Anthropic max, Claude Opus
5.5, while its editor showed an empty Connection, "GPT-6 Sol (current)"
and "This account connection is unavailable". The editor copied its
connection and model once, when it opened, and never looked at the view
again; when the view changed underneath (a switch elsewhere, a re-read)
or its connection dropped out of the list (signed out, key gone), the
select had no matching option and showed nothing, and "(current)" was
put on any model the presets lacked, including one never saved.

- The editor follows the live intelligence view until the owner edits
  it, so the row and the editor read one source.
- A saved connection that is no longer listed is shown as
  "<name> (unavailable)", with a line saying to choose another or
  reconnect it, and the save is disabled until a working one is chosen.
- "(current)" marks only what the agent runs on now.
- The Models list says "Unavailable" on such a row, by the same rule
  (listedConnection / connectionUnavailable) the chip and rail use.

Switching was slow to show: the editor and the chip waited for the
whole intelligence view to be re-read (every credential probed) before
closing, the view then polled every 15 s while the gateway loaded the new
connection, and a gateway reload took every connection down, so every
chip and the rail flashed "Checking model..." and back.

- optimisticAssign puts the choice into the one intelligence query at
  once (agents on the default follow a new default, as the server's do),
  rolls it back if the save fails, and re-reads in the background.
- The chip closes on choosing; a refusal restores the old model and
  says why in a toast.
- Right after a change the view is re-read every 2 s while a connection
  is loading, instead of 15 s.
- A connection that was serving stays "loaded" through a gateway reload
  for up to a minute (holdLoaded); a connection that never loaded shows
  as it is. The chip names a chosen model whose connection is loading.

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/intelligence-route.test.ts`

```
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { agentPickerSelection, agentRouteState, connectionUnavailable, holdLoaded, listedConnection, routeProvider, withAssignment } from './intelligence-route.ts';
import type { IntelligenceView } from './intelligence.svelte';
```
