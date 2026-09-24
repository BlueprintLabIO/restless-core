#!/bin/sh
set -eu

/usr/local/bin/wait-for-company-display true

tabs=/company/browser-profile/restless-tabs.json

set -- \
	/usr/bin/chromium \
	--display=:1 \
	--user-data-dir=/company/browser-profile \
	--download-default-directory=/company/downloads \
	--remote-debugging-address=127.0.0.1 \
	--remote-debugging-port=9222 \
	--start-maximized \
	--password-store=basic \
	--no-first-run \
	--no-default-browser-check \
	--disable-dev-shm-usage \
	--disable-background-networking \
	--no-sandbox

# Chromium's own session store does not record tabs navigated through CDP in
# the Debian build used by this Runtime (observed in the Sprint 05 probe).
# Restore checkpoint URLs when available; also restoring Chromium's session
# would open those same tabs a second time. Keep native recovery as a fallback
# for profiles that do not yet have a broker checkpoint.
if [ -s "$tabs" ]; then
	jq -r '.[] | select(type == "string")' "$tabs" > /company/run/restless-tabs.urls
	while IFS= read -r url; do
		case "$url" in
			http://*|https://*|file://*) set -- "$@" "$url" ;;
		esac
	done < /company/run/restless-tabs.urls
else
	set -- "$@" --restore-last-session
fi

exec "$@"
