# Use actual agent provider for status (#547)

* Use actual agent provider for route status

* Align model picker with agent connection

* Exercise agent picker at desktop and phone widths

* Allow picker fixture to target a real test agent

* Show the selected agent status in the navigation

* Exercise separate agent statuses with a local service

## How it is checked

Your work is checked by test files you cannot see. They are placed at the paths below after you finish, and these are the lines they use to load your code, so the names and locations must match.

`web/src/lib/model/intelligence-route.test.ts`

```
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { agentPickerSelection, agentRouteState } from './intelligence-route.ts';
import type { IntelligenceView } from './intelligence.svelte';
```
