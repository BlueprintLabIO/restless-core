# Sprint 76 — A lost connection keeps the page

**Status:** landed
**Programme:** owner cockpit, polish
**Depends on:** Sprint 75.

## Outcome

Losing the connection no longer costs a draft. The page stays as it was while offline and when the
connection returns, and whatever could not load tries again. On a phone, choosing a sheet thread's
cell closes the comments panel, so the cell is seen. Arriving from a link keeps the thread in view.

## Observed friction

| Friction | Evidence |
|---|---|
| Coming back online reloaded the whole page, losing a half-written comment and any other unsaved state | a sheet comment typed offline: kept with "You're offline" until reconnecting, then the page reloaded about a second later and the box was empty |
| On a phone, choosing a thread's cell changed nothing visible | the panel covers the sheet below 1100 px; the cell was selected behind it |

**Cause of the reload.** Phones load the Exec rail's code on the first tap or key. Offline, that
import fails. Session re-entry (`session-reentry.ts`) reads any failed code load as a Cloud update. Once the plane
answered again, it reloaded the page to fetch the new code. The failed rail load was also never
retried.

## Changes

- Session re-entry ignores a failed code load while the browser is offline: nothing moved, so
  nothing reloads. A failure while online still reloads after an update, as before.
- The rail's import clears itself when it fails, so the next reach for the rail tries again. It
  also loads on `online` when the rail is open.
- Sheet comments: choosing a thread's cell where the panel covers the sheet closes the panel, then
  selects the cell. A thread opened from a For you link only selects its cell quietly.

## Verification

- A probe at 390 px:
  - A comment sent offline says "The comment was not added. You're offline." and keeps the draft.
  - After reconnecting, the page does not navigate and the draft stays (checked at 0.5, 1.5, 3 and
    6 s). It then sends.
  - Choosing a thread's cell closes the panel and shows the grid.
- Sheet comments render without overflow at 1280 px and 390 px, light and dark.
- One "This page could not open" in an early phone-dark capture did not recur in nine direct loads
  and two further full captures. It most likely came from the gateway during a smoke run's
  teardown. Noted, not claimed fixed.
- The multiplayer smoke's sheet step now also goes offline mid-draft. It checks there is no
  reload, that the draft survives, and that the comment arrives once back online.

## Results

- Multiplayer smoke: 22/22, with the offline check in the sheet step.
- Web: svelte-check 0 errors and 0 warnings; unit tests 127/127.
- No migration; schema stays 97.
