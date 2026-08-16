#!/bin/sh
set -eu

/usr/local/bin/wait-for-company-display true

set -- \
	/usr/bin/chromium \
	--display=:1 \
	--user-data-dir=/company/browser-profile \
	--download-default-directory=/company/downloads \
	--remote-debugging-address=127.0.0.1 \
	--remote-debugging-port=9222 \
	--restore-last-session \
	--password-store=basic \
	--no-first-run \
	--no-default-browser-check \
	--disable-dev-shm-usage \
	--disable-background-networking \
	--no-sandbox

# Chromium's own session store does not record tabs navigated through CDP in
# the Debian build used by this Runtime (observed in the Sprint 05 probe).
# The broker checkpoints only generic URLs, so restore those into the same
# persistent profile. Cookies, storage and downloads still belong to Chromium.
tabs=/company/browser-profile/restless-tabs.json
if [ -s "$tabs" ]; then
	jq -r '.[] | select(type == "string")' "$tabs" > /company/run/restless-tabs.urls
	while IFS= read -r url; do
		case "$url" in
			http://*|https://*|file://*) set -- "$@" "$url" ;;
		esac
	done < /company/run/restless-tabs.urls
fi

exec "$@"
