#!/bin/sh
set -eu

export DISPLAY="${DISPLAY:-:1}"

# Prefer the already-supervised persistent browser. If it is between restarts,
# invoking the same profile asks Chromium's process singleton to create/focus a
# window without creating a second company profile.
if /usr/bin/wmctrl -xa Chromium 2>/dev/null; then
	exit 0
fi

exec /usr/bin/chromium \
	--display=:1 \
	--user-data-dir=/company/browser-profile \
	--no-first-run \
	--no-default-browser-check \
	--no-sandbox
