#!/usr/bin/env bash
# Deterministic host/client mechanics gates for the bounded Swift Arrival route.
# Each mode drives the game's built-in probe through real ENet authority; this
# script only starts the two processes, bounds them, and checks their terminal logs.
set -euo pipefail

mode="${1:-}"
case "$mode" in
  positive) flags=() ;;
  route-zero-rejection) flags=(--negative) ;;
  drop-recovery|seat-re-entry) flags=(--recovery) ;;
  *)
    printf 'usage: %s positive|route-zero-rejection|drop-recovery|seat-re-entry\n' "$0" >&2
    exit 2
    ;;
esac

root="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
out_root="${VERIFY_OUTPUT_DIR:-/tmp/swift-arrival-gates}"
out="$out_root/$mode-$$"
mkdir -p "$out"
host_pid=""
cleanup() {
  if [[ -n "$host_pid" ]]; then
    kill "$host_pid" 2>/dev/null || true
    wait "$host_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

HOME="${HOME:-/tmp}/swift-arrival-gate-host" timeout 90s godot --headless --path "$root" -- --host --probe "${flags[@]}" >"$out/host.log" 2>&1 &
host_pid=$!
sleep 1
set +e
HOME="${HOME:-/tmp}/swift-arrival-gate-client" timeout 90s godot --headless --path "$root" -- --client --probe "${flags[@]}" >"$out/client.log" 2>&1
client_status=$?
wait "$host_pid"
host_status=$?
set -e
host_pid=""

if [[ $client_status -ne 0 || $host_status -ne 0 ]]; then
  printf 'gate %s process failure: host=%d client=%d logs=%s\n' "$mode" "$host_status" "$client_status" "$out" >&2
  exit 1
fi

must_contain() {
  local log="$1" text="$2"
  awk -v needle="$text" 'index($0, needle) { found=1 } END { exit(found ? 0 : 1) }' "$log" || {
    printf 'gate %s missing %s in %s\n' "$mode" "$text" "$log" >&2
    exit 1
  }
}
must_not_contain() {
  local log="$1" text="$2"
  awk -v needle="$text" 'index($0, needle) { found=1 } END { exit(found ? 1 : 0) }' "$log" || {
    printf 'gate %s unexpectedly found %s in %s\n' "$mode" "$text" "$log" >&2
    exit 1
  }
}

must_contain "$out/host.log" 'PROBE RESULT: PASS'
must_contain "$out/client.log" 'PROBE RESULT: PASS'
case "$mode" in
  positive)
    must_contain "$out/host.log" 'MISSION: HOST completed delivery #1'
    must_contain "$out/client.log" 'MISSION: DELIVERY COMPLETED (host-authoritative)'
    ;;
  route-zero-rejection)
    must_contain "$out/host.log" 'HOST: REJECTED unload'
    must_contain "$out/client.log" 'ON-FOOT UNLOAD BLOCKED — drive the loaded truck to 40 m, then exit there'
    must_not_contain "$out/host.log" 'MISSION: HOST completed delivery'
    ;;
  drop-recovery)
    must_contain "$out/host.log" 'crate observed dropped BEHIND the truck'
    must_contain "$out/host.log" 'dropped crate RECOVERED'
    must_contain "$out/host.log" 'MISSION: HOST completed delivery #1'
    ;;
  seat-re-entry)
    must_contain "$out/host.log" 'incomplete route exit observed'
    must_contain "$out/host.log" 'natural driver-seat RE-ENTRY accepted'
    must_contain "$out/host.log" 'MISSION: HOST completed delivery #1'
    ;;
esac
printf 'PASS %s logs=%s\n' "$mode" "$out"
