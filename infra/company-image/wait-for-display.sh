#!/bin/sh
set -eu

remaining=300
while [ ! -S /tmp/.X11-unix/X1 ]; do
	remaining=$((remaining - 1))
	if [ "$remaining" -le 0 ]; then
		echo 'display :1 did not become ready within 30 seconds' >&2
		exit 1
	fi
	sleep 0.1
done

exec "$@"
