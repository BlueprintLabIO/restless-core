#!/bin/sh
# Fresh native-input reproduction of the formerly successful on-foot bypass.
# Drives the rendered CLIENT only through X11 keyboard/mouse events, carries the
# crate around the stationary truck into the destination, and proves the host
# rejects unload while route progress remains zero.
set -eu

DIR="/company/worktrees/work-af5ef98a1cff-r1"
OUT="${1:-/company/outputs/exp11/playtests/bypass-$(date -u +%Y%m%dT%H%M%SZ)}"
GODOT="${GODOT:-/usr/local/bin/godot}"
DISPLAY="${DISPLAY:-:1}"
export DISPLAY
export GODOT_HOME="${GODOT_HOME:-/tmp/godot-home}"
mkdir -p "$GODOT_HOME" "$OUT/shots"

TRACE="$OUT/input-trace.tsv"
HOST_LOG="$OUT/host.log"
CLIENT_LOG="$OUT/client.log"
printf 'at\taction\n' > "$TRACE"
mark() { printf '%s\t%s\n' "$(date -Iseconds)" "$1" >> "$TRACE"; }
hold() { mark "keydown:$1"; xdotool keydown "$1"; sleep "$2"; xdotool keyup "$1"; mark "keyup:$1"; }
tap() { mark "tap:$1"; xdotool key "$1"; sleep 1; }
turn() { mark "mousemove_relative:$1"; xdotool mousemove_relative -- $1; sleep 0.5; }

command -v xdotool >/dev/null
HOME="$GODOT_HOME" "$GODOT" --path "$DIR" --resolution 1280x720 --position 0,100 -- --host --shots="$OUT/shots" >"$HOST_LOG" 2>&1 &
HOST_PID=$!
sleep 2
HOME="$GODOT_HOME" "$GODOT" --path "$DIR" --resolution 1280x720 --position 1320,100 -- --client --shots="$OUT/shots" >"$CLIENT_LOG" 2>&1 &
CLIENT_PID=$!
cleanup() { kill "$HOST_PID" "$CLIENT_PID" 2>/dev/null || true; }
trap cleanup EXIT INT TERM
sleep 4

CLIENT_WINDOW="$(xdotool search --onlyvisible --name 'CLIENT' | tail -n 1)"
[ -n "$CLIENT_WINDOW" ]
xdotool windowactivate --sync "$CLIENT_WINDOW"
mark "focused-client:$CLIENT_WINDOW"

tap e
tap q
tap e
# Capture the pointer, turn from the cab toward the rear ramp, and leave on foot.
xdotool mousemove --window "$CLIENT_WINDOW" 640 400
xdotool click 1
mark "click:capture-pointer"
turn "1256 0"
hold w 1.2
# Face the destination, sidestep around the stationary truck, traverse the
# entire route on foot, then return to the yellow zone centre.
turn "1256 0"
hold a 1.5
hold w 14.2
hold d 1.5
sleep 1
tap e
sleep 3
mark "done"

grep -q 'HOST: REJECTED unload.*truck journey incomplete' "$HOST_LOG" || { echo 'host did not reject the on-foot journey bypass' >&2; exit 1; }
grep -q 'route=0.0' "$HOST_LOG" || { echo 'rejection was not observed at route zero' >&2; exit 1; }
grep -q 'FEEDBACK\[reject\]: REJECTED: truck journey incomplete' "$CLIENT_LOG" || { echo 'client did not receive visible journey-stage feedback' >&2; exit 1; }
if grep -q 'HOST completed delivery' "$HOST_LOG"; then echo 'bypass incorrectly completed delivery' >&2; exit 1; fi
find "$OUT/shots" -type f -name '*ev_reject_journey*.png' | grep -q . || { echo 'journey rejection screenshot missing' >&2; exit 1; }

{
	echo "native X11 on-foot route-zero bypass: REJECTED"
	echo "controls exercised: E pickup/unload attempt, W/A/D walking, captured native mouse turns"
	echo "authoritative result: route=0.0, truck journey incomplete, delivery remained in progress"
	echo "screenshots: game-side event capture ev_reject_journey*.png plus interval frames in shots/"
} > "$OUT/result.txt"
echo "bypass playtest bundle: $OUT"
