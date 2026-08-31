# Native gameplay action trace

Attempt: 61c7d1e9-8e46-4157-9ea8-2b335ea9b452
Display: `:1`

1. Capability probe
   - Command: `which godot && which xdotool && which scrot`
   - Observed output: `/usr/local/bin/godot`, `/usr/bin/xdotool`, `/usr/bin/scrot`; exit 0.
2. Launch host
   - Command: `DISPLAY=:1 godot --path /company/projects/swift-arrival-candidate -- --host`
   - Started asynchronously as job `bg_1`.
3. Launch client
   - Command: `DISPLAY=:1 godot --path /company/projects/swift-arrival-candidate -- --client`
   - Started asynchronously as job `bg_2`.
4. Enumerate visible names
   - Command: `DISPLAY=:1 xdotool search --onlyvisible --name 'Swift Arrival' getwindowname %@`
   - Output, in order: `Swift Arrival Dogfood 4 (DEBUG)`; `Swift Arrival Dogfood 4 — CLIENT (peer 906764513)`.
5. Enumerate visible numeric IDs
   - Command: `DISPLAY=:1 xdotool search --onlyvisible --name 'Swift Arrival'`
   - Output, in order: `23068675`; `18874371`.
6. Map IDs to names
   - Command: `DISPLAY=:1 xdotool getwindowname 23068675`
   - Output: `Swift Arrival Dogfood 4 — CLIENT (peer 906764513)`.
   - Command: `DISPLAY=:1 xdotool getwindowname 18874371`
   - Output: `Swift Arrival Dogfood 4 (DEBUG)`.
   - Numeric client ID selected: `23068675`.
7. Initial activation/capture attempt (the one allowed setup failure)
   - Command: `mkdir -p /company/outputs/exp11-referee/candidate-r7 && DISPLAY=:1 xdotool windowactivate --sync 23068675 && scrot -u /company/outputs/exp11-referee/candidate-r7/001-initial.png`
   - Output: `XGetWindowProperty[_NET_ACTIVE_WINDOW] failed (code=1)`; exit 0 was reported for the compound tool call, but no image observation was claimed from this failed attempt.
8. Bounded setup repair
   - Command: `DISPLAY=:1 xdotool windowactivate 23068675 windowfocus --sync 23068675 && scrot -u /company/outputs/exp11-referee/candidate-r7/001-initial.png`
   - Output: none; exit 0.
9. Exact PNG ingestion
   - OMP ordinary read called on exact path `/company/outputs/exp11-referee/candidate-r7/001-initial.png` in this active Sol session.
   - Direct pixel observation: the image header says `HOST — my peer id: 1 | peers connected: [906764513]`, not CLIENT. Other visible HUD text says `on foot | Mission: IN PROGRESS (0) | Crate: free | Driver: free | Truck z: 0.0`; server/authority messages are visible; bottom controls list WASD, mouse look, E, Q, and Esc. A first-person blocky scene is visible.
10. Stop condition
   - The selected ID was deterministically named CLIENT, but focused capture after the single permitted repair produced pixels identifying the HOST surface. Exact client routing therefore remained unavailable after the bounded repair. No gameplay keys or pointer actions were sent, because continuing would violate the requirement to act only through the verified client surface.

No source, Git history, producer material, tests, logs, known-defect material, or prior reports were inspected.
