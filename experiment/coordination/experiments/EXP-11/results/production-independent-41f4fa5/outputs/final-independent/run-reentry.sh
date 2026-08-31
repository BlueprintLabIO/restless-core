#!/bin/sh
set -eu
DIR=/company/worktrees/work-731bd86c6507-r1
OUT=/company/outputs/swift-arrival-r15-final-independent-b8e3161f/re-entry
GODOT=/usr/local/bin/godot
DISPLAY=:1
export DISPLAY
export GODOT_HOME=/tmp/godot-home-b8e3161f-reentry
mkdir -p "$GODOT_HOME" "$OUT/shots"
TRACE="$OUT/input-trace.tsv"
HOST_LOG="$OUT/host.log"
CLIENT_LOG="$OUT/client.log"
printf 'at\taction\n' > "$TRACE"
mark() { printf '%s\t%s\n' "$(date -Iseconds)" "$1" >> "$TRACE"; }
hold() { mark "keydown:$1"; xdotool keydown "$1"; sleep "$2"; xdotool keyup "$1"; mark "keyup:$1"; }
tap() { mark "tap:$1"; xdotool key "$1"; sleep 1; }
HOME="$GODOT_HOME" "$GODOT" --path "$DIR" --resolution 1280x720 --position 0,100 -- --host --shots="$OUT/shots" >"$HOST_LOG" 2>&1 &
HOST_PID=$!
sleep 2
HOME="$GODOT_HOME" "$GODOT" --path "$DIR" --resolution 1280x720 --position 1320,100 -- --client --shots="$OUT/shots" >"$CLIENT_LOG" 2>&1 &
CLIENT_PID=$!
cleanup() { kill "$HOST_PID" "$CLIENT_PID" 2>/dev/null || true; }
trap cleanup EXIT INT TERM
sleep 4
HOST_WINDOW="$(xdotool search --pid "$HOST_PID" | sed -n '$p')"
CLIENT_WINDOW="$(xdotool search --pid "$CLIENT_PID" | sed -n '$p')"
[ -n "$HOST_WINDOW" ]
[ -n "$CLIENT_WINDOW" ]
printf 'role\twindow_id\tname\tgeometry\n' > "$OUT/window-routing.tsv"
printf 'host\t%s\t%s\t%s\n' "$HOST_WINDOW" "$(xdotool getwindowname "$HOST_WINDOW")" "$(xdotool getwindowgeometry --shell "$HOST_WINDOW" | tr '\n' ' ')" >> "$OUT/window-routing.tsv"
printf 'client\t%s\t%s\t%s\n' "$CLIENT_WINDOW" "$(xdotool getwindowname "$CLIENT_WINDOW")" "$(xdotool getwindowgeometry --shell "$CLIENT_WINDOW" | tr '\n' ' ')" >> "$OUT/window-routing.tsv"
xdotool windowactivate --sync "$CLIENT_WINDOW"
mark "focused-client:$CLIENT_WINDOW"
tap e
hold w 1.9
tap e
hold w 4.0
tap e
mark "deliberate-early-exit"
hold w 2.0
tap e
mark "natural-seat-reentry"
sleep 3
[ "$(grep -c 'occupies DRIVER position' "$HOST_LOG")" -ge 2 ]
grep -q 'left driver position at rear cargo threshold' "$HOST_LOG"
grep -q 'SEATED — W/S drive' "$CLIENT_LOG"
grep -q 'EXITED EARLY — re-enter' "$CLIENT_LOG"
[ "$(grep -c 'SEATED — W/S drive' "$CLIENT_LOG")" -ge 2 ]
if grep -q 'HOST completed delivery' "$HOST_LOG"; then
  echo 'unexpected completion during early-exit recovery route' >&2
  exit 1
fi
{
  echo 'native X11 early-exit recovery: VERIFIED'
  echo 'ordinary input: E pickup, W approach, E seat, W partial drive, E early exit, W through cargo, E natural re-entry'
  echo 'authoritative result: host observed early exit and second DRIVER occupancy; client received truthful early-exit guidance and second SEATED feedback'
} > "$OUT/result.txt"
echo "re-entry playtest bundle: $OUT"
