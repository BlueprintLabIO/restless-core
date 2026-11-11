# Swift Arrival candidate R7 — blind native gameplay report

## Validity

**INVALID: exact client routing remained unavailable after the one permitted bounded setup repair.**

The visible-window enumeration mapped numeric ID `23068675` to `Swift Arrival Dogfood 4 — CLIENT (peer 906764513)` and ID `18874371` to `Swift Arrival Dogfood 4 (DEBUG)`. The initial `windowactivate --sync 23068675` / focused-capture command emitted `XGetWindowProperty[_NET_ACTIVE_WINDOW] failed (code=1)`. The bounded repair used explicit activation plus `windowfocus --sync 23068675`, then `scrot -u`. OMP ordinary `read` ingested that exact PNG, whose pixels visibly identified the surface as `HOST`, not CLIENT. Thus command success did not establish correct routing, and the pixel evidence contradicted the intended target.

## Pixel observations

From the only exact PNG read in this Sol session, `001-initial.png`:

- Top header visibly reads `HOST — my peer id: 1 | peers connected: [906764513]`.
- Status visibly reads `on foot | Mission: IN PROGRESS (0) | Crate: free | Driver: free | Truck z: 0.0`.
- Visible network/authority text identifies a server listening on `127.0.0.1:24565`, a connected peer `906764513`, and authoritative world state sent to that peer.
- A blocky first-person scene is visible, with arms, a central tan rectangular object, a humanoid figure ahead, blue walls, and a brown floor.
- Bottom HUD visibly lists WASD movement, mouse look, E interaction/grab/seat/unload-in-zone, Q drop, and Esc free mouse.

## Actions

- Probed the installed native tools.
- Launched one host and one client Godot process on `DISPLAY=:1`.
- Enumerated visible Swift Arrival window names and numeric IDs, then mapped each ID to its name.
- Selected numeric client ID `23068675`.
- Retained the initial focus/capture failure and one bounded repair in `action-trace.md`.
- Read the resulting exact PNG through OMP ordinary `read` in this same Sol session.
- Sent no gameplay input after the host/client contradiction.

## Inference

- The focused-capture route likely did not result in the client being the focused window, despite the repair command exiting without output. This is inference; the direct evidence is only the ID/name mapping and the `HOST` pixels.
- A host and client appear connected based on visible host HUD text, but this does not prove the client surface was controllable or correctly captured.

## Uncertainty

- I did not determine why focus remained on or returned to the host.
- I did not inspect logs, source, tests, or any alternate evidence path, as prohibited.
- Native keyboard/pointer control and post-input client recapture were not tested because exact client routing was not established after the allowed repair.

## Goal outcome

**Not assessed.** I did not attempt crate pickup, driving, destination delivery, or unloading because inputs could not be safely routed to a visually verified client window.

## Intentional mistake and recovery

**Not assessed.** No ordinary gameplay mistake was made and no recovery was attempted. The only failure was setup-level exact routing, which consumed the single allowed bounded setup repair and still contradicted the intended client target in pixels.

## Evidence

- `001-initial.png` — exact PNG ingested through OMP ordinary `read`; pixels show HOST.
- `action-trace.md` — exact commands, outputs, failure, repair, and image-read record.
- This report.

This invalid result is not a gameplay-quality or founder-acceptance verdict.
